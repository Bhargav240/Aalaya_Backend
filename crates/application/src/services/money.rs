//! Money use cases: record cash payment (+ templated receipt), ledger, receipt
//! download, move-out settlement, and per-org receipt template management.
use crate::authz;
use crate::error::AppError;
use crate::ports::billing::BillingEngine;
use crate::ports::clock::Clock;
use crate::ports::repositories::{
    AuditActor, BranchRepository, PaymentRepository, ReceiptMeta, ReceiptTemplateRepository,
    ResidentRepository, SettlementRepository, UserRepository,
};
use crate::ports::storage::{ObjectStore, ReceiptData, ReceiptRenderer, ReceiptTemplate};
use crate::ports::tenancy::TenantContext;
use pg_domain::{OrgId, PaymentId, PlanKind, ReceiptId, ResidentId, Role};
use uuid::Uuid;

pub struct RecordPaymentOutput {
    pub payment_id: Uuid,
    pub receipt_id: Uuid,
    pub receipt_number: String,
}

/// Minimum notice (days) before a move-out for the deposit to be refundable.
/// Env-overridable; defaults to the common 30-day notice period.
fn notice_days_required() -> i64 {
    std::env::var("MOVE_OUT_NOTICE_DAYS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)
}

/// How a role is printed on a receipt.
fn role_label(role: Role) -> &'static str {
    match role {
        Role::Owner => "Owner",
        Role::Manager => "Manager",
        Role::PlatformOperator => "Platform operator",
        Role::Tenant => "Resident",
        Role::Prospect => "Prospect",
    }
}

/// Cash payment -> payment row + pending receipt (committed) -> render PDF from
/// the org's template -> store under a namespaced key -> mark receipt ready.
/// The branch line appears only for multi-branch orgs (a single PG stays clean).
pub struct RecordPaymentCommand<'a> {
    pub template_repo: &'a dyn ReceiptTemplateRepository,
    pub payment_repo: &'a dyn PaymentRepository,
    pub resident_repo: &'a dyn ResidentRepository,
    pub branch_repo: &'a dyn BranchRepository,
    /// Resolves the name printed as "Recorded by" on the receipt.
    pub user_repo: &'a dyn UserRepository,
    pub store: &'a dyn ObjectStore,
    pub renderer: &'a dyn ReceiptRenderer,
    pub clock: &'a dyn Clock,
    pub ctx: &'a TenantContext,
    pub role: Role,
    pub actor: &'a AuditActor,
    pub resident_id: ResidentId,
    pub amount_paise: i64,
    /// How the money arrived: "cash", "upi", "card"… Defaults to cash at the
    /// route, so existing callers behave exactly as before.
    pub method: &'a str,
    /// Provider reference for a non-cash payment, if the collector has one.
    pub reference: Option<&'a str>,
    pub idempotency_key: Option<&'a str>,
}
pub async fn record_payment(
    command: RecordPaymentCommand<'_>,
) -> Result<RecordPaymentOutput, AppError> {
    let RecordPaymentCommand {
        template_repo,
        payment_repo,
        resident_repo,
        branch_repo,
        user_repo,
        store,
        renderer,
        clock,
        ctx,
        role,
        actor,
        resident_id,
        amount_paise,
        method,
        reference,
        idempotency_key,
    } = command;
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    if amount_paise <= 0 {
        return Err(AppError::Validation("amount must be positive".into()));
    }
    let detail = resident_repo
        .get(ctx, resident_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let branch_count = branch_repo.count(ctx).await?;
    let template = template_repo.resolve(ctx.org_id).await?;
    // Name the person who actually took the money. This used to be the literal
    // "Owner (Admin)" on every receipt, which was simply false whenever a
    // manager collected the cash. The role is kept alongside the name so the
    // receipt still says in what capacity it was taken, and a user with no
    // stored name degrades to the role rather than to someone else's title.
    let recorded_by = match actor.user_id {
        Some(user_id) => match user_repo.expected_name(user_id).await? {
            Some(name) if !name.trim().is_empty() => {
                format!("{} ({})", name.trim(), role_label(role))
            }
            _ => role_label(role).to_string(),
        },
        None => role_label(role).to_string(),
    };

    let payment_id = PaymentId::new(Uuid::new_v4());
    let receipt_id = ReceiptId::new(Uuid::new_v4());

    // The repo owns idempotency: on a replayed key it returns the ORIGINAL
    // receipt number (stored_receipt_number != our candidate means "this was a
    // retry"), and we skip the side effects so no duplicate PDF ever lands.
    let candidate_receipt_number = format!(
        "RC-{}",
        receipt_id.as_uuid().simple().to_string()[..8].to_uppercase()
    );

    let recorded = payment_repo
        .record(
            ctx,
            payment_id,
            receipt_id,
            resident_id,
            amount_paise,
            &candidate_receipt_number,
            actor.user_id,
            method,
            reference,
            idempotency_key,
            actor,
        )
        .await?;

    // A retried request already has its payment, its receipt row and its PDF.
    // Rendering again would write a second file and then try to ready a receipt
    // that was never inserted, so return the original and stop here.
    if recorded.replayed {
        return Ok(RecordPaymentOutput {
            payment_id: recorded.payment_id,
            receipt_id: recorded.receipt_id,
            receipt_number: recorded.receipt_number,
        });
    }

    let receipt_number = recorded.receipt_number.clone();
    let data = ReceiptData {
        receipt_number: receipt_number.clone(),
        invoice_number: receipt_number.clone(),
        resident_name: detail.full_name.clone(),
        branch_name: if branch_count > 1 {
            Some(detail.branch_name.clone())
        } else {
            None
        },
        amount_paise,
        issued_epoch: clock.now().unix_timestamp(),
        tenant_phone: detail.phone.clone(),
        room_number: detail.room_number.clone(),
        plan_kind: detail.plan_kind.clone(),
        // The payment route only collects rent; deposits are seeded directly
        // for now (payments.kind carries the distinction in the database).
        line_item: "Rent".to_string(),
        // Read from the payment, not assumed. This used to be the literal
        // "Cash" on every invoice, which was wrong the moment a resident paid
        // by UPI — the database recorded the real method while the PDF the
        // resident kept said otherwise.
        payment_method: payment_method_label(method),
        payment_reference: reference
            .map(str::to_string)
            .filter(|r| !r.trim().is_empty()),
        floor_label: detail.room_number.as_deref().map(floor_label_for_room),
        recorded_by,
    };
    // Skip side effects on an idempotent replay: the PDF for the original key
    // already exists (or will, once the first request finishes).
    let pdf = renderer.render(&template, &data);
    let key = format!("receipts/{}/{}/{}.pdf", ctx.org_id, resident_id, receipt_id);
    store.put(&key, pdf, "application/pdf").await?;
    payment_repo
        .set_receipt_ready(ctx, receipt_id, &key)
        .await?;

    Ok(RecordPaymentOutput {
        payment_id: payment_id.as_uuid(),
        receipt_id: receipt_id.as_uuid(),
        receipt_number,
    })
}

/// How a payment method is printed on an invoice. Unknown values are shown
/// as they were stored rather than swallowed, so a method added server-side
/// later still reads correctly without a client change.
fn payment_method_label(method: &str) -> String {
    match method.trim().to_ascii_lowercase().as_str() {
        "" | "cash" => "Cash".to_string(),
        "upi" => "UPI".to_string(),
        "card" => "Card".to_string(),
        "bank" | "bank_transfer" => "Bank transfer".to_string(),
        other => {
            let mut c = other.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => "Cash".to_string(),
            }
        }
    }
}

/// The floor a room sits on, from the room-numbering convention the rest of
/// the product already uses ("201" is floor 2, "G01" is the ground floor).
/// Mirrors Floors.kt on the client so both agree.
fn floor_label_for_room(room_number: &str) -> String {
    match room_number.trim().chars().next() {
        Some(c) if c.is_ascii_digit() => format!("Floor {c}"),
        Some(c) if c.eq_ignore_ascii_case(&'G') => "Ground floor".to_string(),
        Some(c) if c.is_alphabetic() => format!("Block {}", c.to_uppercase()),
        _ => "Floor".to_string(),
    }
}

/// Reverse the most recent payment recorded against a resident.
///
/// The counterpart to [`record_payment`], so a mistaken "mark as paid" can be
/// corrected instead of standing forever. Same authorization as recording one:
/// whoever can take the money can undo taking it. The stored PDF is left in the
/// object store — the receipt row is soft-deleted, so nothing links to it any
/// more, and keeping the bytes costs nothing while preserving the paper trail.
#[derive(Debug)]
pub struct VoidPaymentOutput {
    pub payment_id: Uuid,
    pub amount_paise: i64,
    pub invoice_number: Option<String>,
}

pub async fn void_last_payment(
    payment_repo: &dyn PaymentRepository,
    ctx: &TenantContext,
    role: Role,
    actor: &AuditActor,
    resident_id: ResidentId,
) -> Result<VoidPaymentOutput, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    let voided = payment_repo
        .void_last_payment(ctx, resident_id, actor)
        .await?
        .ok_or_else(|| AppError::Conflict("no payment to reverse".into()))?;
    Ok(VoidPaymentOutput {
        payment_id: voided.payment_id,
        amount_paise: voided.amount_paise,
        invoice_number: voided.invoice_number,
    })
}

pub async fn list_receipts(
    payment_repo: &dyn PaymentRepository,
    ctx: &TenantContext,
    role: Role,
    resident_id: ResidentId,
) -> Result<Vec<ReceiptMeta>, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    payment_repo.list_receipts(ctx, resident_id).await
}

pub enum ReceiptDownload {
    Redirect(String),
    Inline(Vec<u8>),
}

/// Resolve a receipt download after an authorization + readiness check. Prefers
/// a short-lived signed URL (Supabase/S3); falls back to streaming the bytes.
pub async fn resolve_receipt_download(
    payment_repo: &dyn PaymentRepository,
    store: &dyn ObjectStore,
    ctx: &TenantContext,
    role: Role,
    receipt_id: ReceiptId,
    ttl_secs: i64,
) -> Result<ReceiptDownload, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    let file = payment_repo
        .get_receipt(ctx, receipt_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if file.status != "ready" {
        return Err(AppError::Conflict("receipt not ready".into()));
    }
    let key = file.storage_key.ok_or(AppError::NotFound)?;
    if let Some(url) = store.signed_url(&key, ttl_secs).await? {
        return Ok(ReceiptDownload::Redirect(url));
    }
    Ok(ReceiptDownload::Inline(store.get(&key).await?))
}

pub struct LedgerView {
    pub full_name: String,
    pub branch_id: String,
    pub branch_name: String,
    pub room_number: Option<String>,
    pub bed_label: Option<String>,
    pub plan_kind: String,
    pub status: String,
    pub rent_paise: i64,
    pub deposit_paise: i64,
    pub paid_total_paise: i64,
    pub accrued_paise: i64,
    pub outstanding_paise: i64,
}

/// Accrual stops at the settled last day for moved-out residents.
fn accrual_end(
    status: &str,
    moved_out_at: Option<time::OffsetDateTime>,
    today: time::Date,
) -> time::Date {
    if status == "moved_out" {
        moved_out_at.map(|t| t.date()).unwrap_or(today)
    } else {
        today
    }
}

pub async fn ledger(
    payment_repo: &dyn PaymentRepository,
    resident_repo: &dyn ResidentRepository,
    billing: &dyn BillingEngine,
    clock: &dyn Clock,
    ctx: &TenantContext,
    role: Role,
    resident_id: ResidentId,
) -> Result<LedgerView, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    let d = resident_repo
        .get(ctx, resident_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let paid = payment_repo.paid_total(ctx, resident_id).await?;
    let plan = PlanKind::parse(&d.plan_kind).unwrap_or(PlanKind::Monthly);
    let end = accrual_end(&d.status, d.moved_out_at, clock.now().date());
    let accrued = billing.accrued_paise(plan, d.rent_paise, d.joined.date(), end);
    Ok(LedgerView {
        full_name: d.full_name,
        branch_id: d.branch_id.to_string(),
        branch_name: d.branch_name,
        room_number: d.room_number,
        bed_label: d.bed_label,
        plan_kind: d.plan_kind,
        status: d.status,
        rent_paise: d.rent_paise,
        deposit_paise: d.deposit_paise,
        paid_total_paise: paid,
        accrued_paise: accrued,
        outstanding_paise: (accrued - paid).max(0),
    })
}

pub struct Deduction {
    pub reason: String,
    pub amount_paise: i64,
}

pub struct Settlement {
    pub rent_to_date_paise: i64,
    pub paid_total_paise: i64,
    pub outstanding_paise: i64,
    pub deposit_held_paise: i64,
    pub deductions_total_paise: i64,
    pub refund_paise: i64,
    pub forfeited: bool,
    pub note: String,
}

pub struct MoveOutCommand<'a> {
    pub payment_repo: &'a dyn PaymentRepository,
    pub resident_repo: &'a dyn ResidentRepository,
    pub settlement_repo: &'a dyn SettlementRepository,
    pub billing: &'a dyn BillingEngine,
    pub ctx: &'a TenantContext,
    pub role: Role,
    pub actor: &'a AuditActor,
    pub resident_id: ResidentId,
    pub last_day: time::Date,
    pub notice_days: i64,
    pub deductions: Vec<Deduction>,
}
pub async fn move_out(command: MoveOutCommand<'_>) -> Result<Settlement, AppError> {
    let MoveOutCommand {
        payment_repo,
        resident_repo,
        settlement_repo,
        billing,
        ctx,
        role,
        actor,
        resident_id,
        last_day,
        notice_days,
        deductions,
    } = command;
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    let d = resident_repo
        .get(ctx, resident_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if d.status == "moved_out" {
        return Err(AppError::Conflict("resident already moved out".into()));
    }
    let plan = PlanKind::parse(&d.plan_kind).unwrap_or(PlanKind::Monthly);
    let rent_to_date = billing.accrued_paise(plan, d.rent_paise, d.joined.date(), last_day);
    let paid = payment_repo.paid_total(ctx, resident_id).await?;
    let outstanding = (rent_to_date - paid).max(0);
    // Reject negative deductions instead of silently zeroing them: a negative
    // deduction is a data-entry error, not a top-up.
    if let Some(bad) = deductions.iter().find(|x| x.amount_paise < 0) {
        return Err(AppError::Validation(format!(
            "deduction '{}' cannot be negative",
            bad.reason
        )));
    }
    let deductions_total: i64 = deductions.iter().map(|x| x.amount_paise).sum();

    let notice_required = notice_days_required();
    let (refund, forfeited, note) = if notice_days < notice_required {
        (
            0,
            true,
            format!("deposit forfeited: notice under {notice_required} days"),
        )
    } else {
        let refund = (d.deposit_paise - outstanding - deductions_total).max(0);
        (
            refund,
            false,
            "refund = deposit - outstanding - deductions".to_string(),
        )
    };

    resident_repo
        .mark_moved_out(ctx, resident_id, last_day, actor)
        .await?;

    // Persist the immutable settlement + deduction lines: a refund that later
    // becomes "I never got that" is settled history, not tribal memory.
    settlement_repo
        .record(
            ctx,
            Uuid::new_v4(),
            &crate::ports::repositories::SettlementRecord {
                resident_id: resident_id.as_uuid(),
                last_day,
                notice_days,
                rent_to_date_paise: rent_to_date,
                paid_total_paise: paid,
                outstanding_paise: outstanding,
                deposit_held_paise: d.deposit_paise,
                deductions_total_paise: deductions_total,
                refund_paise: refund,
                forfeited,
                note: note.clone(),
                deductions: deductions
                    .iter()
                    .map(|x| (x.reason.clone(), x.amount_paise))
                    .collect(),
            },
            actor,
        )
        .await?;

    Ok(Settlement {
        rent_to_date_paise: rent_to_date,
        paid_total_paise: paid,
        outstanding_paise: outstanding,
        deposit_held_paise: d.deposit_paise,
        deductions_total_paise: deductions_total,
        refund_paise: refund,
        forfeited,
        note,
    })
}

// ---- Per-org receipt template management ----
pub async fn get_receipt_template(
    repo: &dyn ReceiptTemplateRepository,
    role: Role,
    org_id: OrgId,
) -> Result<ReceiptTemplate, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    repo.resolve(org_id).await
}

pub async fn set_receipt_template(
    repo: &dyn ReceiptTemplateRepository,
    role: Role,
    org_id: OrgId,
    template: ReceiptTemplate,
) -> Result<(), AppError> {
    if !authz::is_owner(role) {
        return Err(AppError::Forbidden);
    }
    repo.upsert(org_id, &template).await
}

/// The resident a receipt belongs to — used by routes to branch-scope managers.
pub async fn receipt_resident(
    payment_repo: &dyn PaymentRepository,
    ctx: &TenantContext,
    role: Role,
    receipt_id: ReceiptId,
) -> Result<ResidentId, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    let file = payment_repo
        .get_receipt(ctx, receipt_id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(ResidentId::new(file.resident_id))
}

/// Tenant self-service download: the receipt must belong to the caller's own
/// linked resident row. Same readiness + signed-URL rules as the staff path.
pub async fn my_receipt_download(
    payment_repo: &dyn PaymentRepository,
    resident_repo: &dyn ResidentRepository,
    store: &dyn ObjectStore,
    ctx: &TenantContext,
    user_id: pg_domain::UserId,
    receipt_id: ReceiptId,
    ttl_secs: i64,
) -> Result<ReceiptDownload, AppError> {
    let rid = resident_repo
        .resident_id_of_user(ctx, user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let file = payment_repo
        .get_receipt(ctx, receipt_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if file.resident_id != rid.as_uuid() {
        return Err(AppError::NotFound);
    }
    if file.status != "ready" {
        return Err(AppError::Conflict("receipt not ready".into()));
    }
    let key = file.storage_key.ok_or(AppError::NotFound)?;
    if let Some(url) = store.signed_url(&key, ttl_secs).await? {
        return Ok(ReceiptDownload::Redirect(url));
    }
    Ok(ReceiptDownload::Inline(store.get(&key).await?))
}

// ---- Tenant self-service (resolve the caller's own resident row) ----
pub async fn my_receipts(
    payment_repo: &dyn PaymentRepository,
    resident_repo: &dyn ResidentRepository,
    ctx: &TenantContext,
    user_id: pg_domain::UserId,
) -> Result<Vec<ReceiptMeta>, AppError> {
    let rid = resident_repo
        .resident_id_of_user(ctx, user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    payment_repo.list_receipts(ctx, rid).await
}

pub async fn my_ledger(
    payment_repo: &dyn PaymentRepository,
    resident_repo: &dyn ResidentRepository,
    billing: &dyn BillingEngine,
    clock: &dyn Clock,
    ctx: &TenantContext,
    user_id: pg_domain::UserId,
) -> Result<LedgerView, AppError> {
    let rid = resident_repo
        .resident_id_of_user(ctx, user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let d = resident_repo
        .get(ctx, rid)
        .await?
        .ok_or(AppError::NotFound)?;
    let paid = payment_repo.paid_total(ctx, rid).await?;
    let plan = PlanKind::parse(&d.plan_kind).unwrap_or(PlanKind::Monthly);
    let end = accrual_end(&d.status, d.moved_out_at, clock.now().date());
    let accrued = billing.accrued_paise(plan, d.rent_paise, d.joined.date(), end);
    Ok(LedgerView {
        full_name: d.full_name,
        branch_id: d.branch_id.to_string(),
        branch_name: d.branch_name,
        room_number: d.room_number,
        bed_label: d.bed_label,
        plan_kind: d.plan_kind,
        status: d.status,
        rent_paise: d.rent_paise,
        deposit_paise: d.deposit_paise,
        paid_total_paise: paid,
        accrued_paise: accrued,
        outstanding_paise: (accrued - paid).max(0),
    })
}
