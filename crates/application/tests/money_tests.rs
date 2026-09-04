//! Money use cases: move-out settlement math, ledger accrual, payment→receipt.
//!
//! These exercise the REAL production billing engine (SimpleBillingEngine via
//! the pg-infrastructure dev-dep) — the paise asserted here are the exact
//! numbers a live system would produce, not a re-implemented formula.
//!
//! Kebab-style `fn__scenario` names are intentional; silence the naming lint.
#![allow(non_snake_case)]
mod fakes;

use fakes::*;
use pg_application::error::AppError;
use pg_application::ports::tenancy::{Placement, TenantContext};
use pg_application::services::money::{
    ledger, move_out, record_payment, void_last_payment, Deduction, MoveOutCommand,
    RecordPaymentCommand,
};
use pg_domain::{OrgId, Role};
use pg_infrastructure::billing::SimpleBillingEngine;
use time::macros::datetime;
use time::OffsetDateTime;
use uuid::Uuid;

fn pooled_ctx() -> TenantContext {
    TenantContext {
        org_id: OrgId::new(Uuid::new_v4()),
        placement: Placement::Pooled,
    }
}

fn actor() -> pg_application::ports::AuditActor {
    pg_application::ports::AuditActor {
        user_id: None,
        request_id: None,
    }
}

/// `Result<T, AppError>` unwrap that works even when `T` has no `Debug`.
fn must<T>(r: Result<T, AppError>, what: &str) -> T {
    r.unwrap_or_else(|e| panic!("{what} failed: {e:?}"))
}

fn must_err<T>(r: Result<T, AppError>, what: &str) -> AppError {
    match r {
        Ok(_) => panic!("{what} unexpectedly succeeded"),
        Err(e) => e,
    }
}

const OCT_01: OffsetDateTime = datetime!(2024-10-01 00:00 UTC);
const NOV_05: OffsetDateTime = datetime!(2024-11-05 00:00 UTC);

fn seeded(over: impl FnOnce(&mut SeededResident)) -> (FakeResidentRepo, SeededResident) {
    let repo = FakeResidentRepo::new();
    let mut r = SeededResident::active(pg_domain::BranchId::new(Uuid::new_v4()));
    over(&mut r);
    repo.seed(r.clone());
    (repo, r)
}

// ------------------------------- move_out ----------------------------------

#[tokio::test]
async fn move_out__short_notice__forfeits_entire_deposit() {
    let (residents, r) = seeded(|r| {
        r.rent_paise = 1_000_000;
        r.deposit_paise = 50_0000;
        r.joined = OCT_01;
    });
    let s = must(
        move_out(MoveOutCommand {
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            settlement_repo: &FakeSettlementRepo::new(),
            billing: &SimpleBillingEngine,
            ctx: &pooled_ctx(),
            role: Role::Manager,
            actor: &actor(),
            resident_id: r.id,
            last_day: NOV_05.date(),
            notice_days: 10,
            deductions: vec![],
        })
        .await,
        "settlement",
    );

    assert!(s.forfeited, "deposit forfeited: note={}", s.note);
    assert_eq!(s.refund_paise, 0);
    assert_eq!(s.deposit_held_paise, 50_0000);
    assert!(
        s.note.contains("forfeited"),
        "note explains forfeit: {}",
        s.note
    );
    assert_eq!(
        residents.moved_out_calls.lock().expect("poisoned").len(),
        1,
        "move_out must finalize via mark_moved_out"
    );
}

#[tokio::test]
async fn move_out__full_notice__refunds_deposit_minus_outstanding_minus_deductions() {
    let (residents, r) = seeded(|r| {
        r.rent_paise = 3_000_000; // ₹30,000/mo => ₹1,000/day
        r.deposit_paise = 1_000_000;
        r.joined = OCT_01;
    });
    let payments = FakePaymentRepo::new();
    payments
        .paid
        .lock()
        .expect("poisoned")
        .insert(r.id.as_uuid(), 2_000_000);

    // join Oct 1 -> Nov 5 == 35 days => accrued 35,000; paid 20,000 => outstanding 15,000
    // refund = 10,000 - 15,000 - 500 = negative => capped at 0
    let s = must(
        move_out(MoveOutCommand {
            payment_repo: &payments,
            resident_repo: &residents,
            settlement_repo: &FakeSettlementRepo::new(),
            billing: &SimpleBillingEngine,
            ctx: &pooled_ctx(),
            role: Role::Owner,
            actor: &actor(),
            resident_id: r.id,
            last_day: NOV_05.date(),
            notice_days: 30,
            deductions: vec![Deduction {
                reason: "wall damage".into(),
                amount_paise: 50_000,
            }],
        })
        .await,
        "settlement",
    );

    assert!(!s.forfeited);
    assert_eq!(s.rent_to_date_paise, 350_0000);
    assert_eq!(s.paid_total_paise, 2_000_000);
    assert_eq!(s.outstanding_paise, 150_0000);
    assert_eq!(s.deductions_total_paise, 50_000);
    assert_eq!(s.refund_paise, 0, "no negative refunds; capped at zero");
}

#[tokio::test]
async fn move_out__clean_ledger__full_deposit_back() {
    let (residents, r) = seeded(|r| {
        r.rent_paise = 0; // free stay
        r.deposit_paise = 50_0000;
        r.joined = OCT_01;
    });
    let s = must(
        move_out(MoveOutCommand {
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            settlement_repo: &FakeSettlementRepo::new(),
            billing: &SimpleBillingEngine,
            ctx: &pooled_ctx(),
            role: Role::Owner,
            actor: &actor(),
            resident_id: r.id,
            last_day: NOV_05.date(),
            notice_days: 45,
            deductions: vec![],
        })
        .await,
        "settlement",
    );

    assert_eq!(s.outstanding_paise, 0);
    assert_eq!(s.refund_paise, 50_0000);
}

#[tokio::test]
async fn move_out__second_time__conflicts() {
    let (residents, r) = seeded(|r| r.status = "moved_out".into());
    let err = must_err(
        move_out(MoveOutCommand {
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            settlement_repo: &FakeSettlementRepo::new(),
            billing: &SimpleBillingEngine,
            ctx: &pooled_ctx(),
            role: Role::Owner,
            actor: &actor(),
            resident_id: r.id,
            last_day: NOV_05.date(),
            notice_days: 30,
            deductions: vec![],
        })
        .await,
        "double move-out",
    );
    assert!(
        matches!(err, AppError::Conflict(_)),
        "409 on re-move-out, got {err:?}"
    );
}

#[tokio::test]
async fn move_out__tenant_cannot_settle__forbidden() {
    let (residents, r) = seeded(|_| {});
    let err = must_err(
        move_out(MoveOutCommand {
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            settlement_repo: &FakeSettlementRepo::new(),
            billing: &SimpleBillingEngine,
            ctx: &pooled_ctx(),
            role: Role::Tenant,
            actor: &actor(),
            resident_id: r.id,
            last_day: NOV_05.date(),
            notice_days: 30,
            deductions: vec![],
        })
        .await,
        "tenant settles",
    );
    assert!(matches!(err, AppError::Forbidden));
}

// -------------------------------- ledger -----------------------------------

#[tokio::test]
async fn ledger__active_resident__accrued_minus_paid() {
    let (residents, r) = seeded(|r| {
        r.rent_paise = 3_000_000;
        r.joined = OCT_01;
    });
    let payments = FakePaymentRepo::new();
    payments
        .paid
        .lock()
        .expect("poisoned")
        .insert(r.id.as_uuid(), 1_000_000);
    let clock = FixedClock::at(datetime!(2024-10-16 00:00 UTC).date()); // 15 days

    let v = must(
        ledger(
            &payments,
            &residents,
            &SimpleBillingEngine,
            &clock,
            &pooled_ctx(),
            Role::Manager,
            r.id,
        )
        .await,
        "ledger",
    );

    assert_eq!(v.accrued_paise, 150_0000); // 15 days * ₹1,000
    assert_eq!(v.paid_total_paise, 1_000_000);
    assert_eq!(v.outstanding_paise, 50_0000);
}

#[tokio::test]
async fn ledger__moved_out__accrual_stops_at_moved_out_date() {
    let (residents, r) = seeded(|r| {
        r.rent_paise = 3_000_000;
        r.joined = OCT_01;
        r.status = "moved_out".into();
        r.moved_out_at = Some(datetime!(2024-10-11 00:00 UTC)); // 10 days only
    });
    // clock says Dec 31, but accrual must stop Oct 11
    let clock = FixedClock::at(datetime!(2024-12-31 00:00 UTC).date());

    let v = must(
        ledger(
            &FakePaymentRepo::new(),
            &residents,
            &SimpleBillingEngine,
            &clock,
            &pooled_ctx(),
            Role::Manager,
            r.id,
        )
        .await,
        "ledger",
    );

    assert_eq!(v.accrued_paise, 1_000_000, "accrual stops at moved_out_at");
}

#[tokio::test]
async fn ledger__overpaid__outstanding_caps_at_zero() {
    let (residents, r) = seeded(|r| {
        r.rent_paise = 3_000_000;
        r.joined = OCT_01;
    });
    let payments = FakePaymentRepo::new();
    payments
        .paid
        .lock()
        .expect("poisoned")
        .insert(r.id.as_uuid(), 9_900_000);
    let clock = FixedClock::at(datetime!(2024-10-05 00:00 UTC).date()); // 4 days => ₹4,000

    let v = must(
        ledger(
            &payments,
            &residents,
            &SimpleBillingEngine,
            &clock,
            &pooled_ctx(),
            Role::Manager,
            r.id,
        )
        .await,
        "ledger",
    );
    assert_eq!(v.outstanding_paise, 0);
}

// ------------------------------- record_payment ----------------------------

#[tokio::test]
async fn record_payment__formats_receipt_number_rc_upper8() {
    let (residents, r) = seeded(|_| {});
    let payments = FakePaymentRepo::new();
    let renderer = SpyRenderer::new();

    let out = must(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: &payments,
            resident_repo: &residents,
            branch_repo: &FakeBranchRepo::with_count(1),
            user_repo: &FakeUserRepo::new(),
            store: &FakeObjectStore,
            renderer: &renderer,
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Manager,
            actor: &actor(),
            resident_id: r.id,
            amount_paise: 1_234_500,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "payment",
    );

    assert!(
        out.receipt_number.starts_with("RC-") && out.receipt_number.len() == 11,
        "receipt number shape: {}",
        out.receipt_number
    );
    let suffix = &out.receipt_number[3..];
    assert!(
        suffix.chars().all(|c| c.is_ascii_hexdigit()),
        "hex suffix: {suffix}"
    );
    assert_eq!(
        suffix,
        suffix.to_uppercase(),
        "receipt number must be uppercased: {suffix}"
    );
    assert_eq!(payments.total_for(r.id.as_uuid()), 1_234_500);
    assert_eq!(payments.ready.lock().expect("poisoned").len(), 1);
}

#[tokio::test]
async fn record_payment__multi_branch__branch_name_on_receipt() {
    let (residents, r) = seeded(|r| r.branch_name = "Koramangala".into());
    let renderer = SpyRenderer::new();

    must(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            branch_repo: &FakeBranchRepo::with_count(2),
            user_repo: &FakeUserRepo::new(),
            store: &FakeObjectStore,
            renderer: &renderer,
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Manager,
            actor: &actor(),
            resident_id: r.id,
            amount_paise: 10_000,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "payment",
    );

    let seen = renderer.seen.lock().expect("poisoned");
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].branch_name.as_deref(), Some("Koramangala"));
}

#[tokio::test]
async fn record_payment__single_branch__omits_branch_line() {
    let (residents, r) = seeded(|_| {});
    let renderer = SpyRenderer::new();

    must(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            branch_repo: &FakeBranchRepo::with_count(1),
            user_repo: &FakeUserRepo::new(),
            store: &FakeObjectStore,
            renderer: &renderer,
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Manager,
            actor: &actor(),
            resident_id: r.id,
            amount_paise: 10_000,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "payment",
    );

    assert_eq!(renderer.seen.lock().expect("poisoned")[0].branch_name, None);
}

#[tokio::test]
async fn record_payment__zero_or_negative__rejected() {
    let (residents, r) = seeded(|_| {});
    let err = must_err(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            branch_repo: &FakeBranchRepo::with_count(1),
            user_repo: &FakeUserRepo::new(),
            store: &FakeObjectStore,
            renderer: &SpyRenderer::new(),
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Manager,
            actor: &actor(),
            resident_id: r.id,
            amount_paise: 0,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "zero payment",
    );
    assert!(matches!(err, AppError::Validation(_)));
}

#[tokio::test]
async fn record_payment__tenant_cannot_record__forbidden() {
    let (residents, r) = seeded(|_| {});
    let err = must_err(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            branch_repo: &FakeBranchRepo::with_count(1),
            user_repo: &FakeUserRepo::new(),
            store: &FakeObjectStore,
            renderer: &SpyRenderer::new(),
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Tenant,
            actor: &actor(),
            resident_id: r.id,
            amount_paise: 10_000,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "tenant records",
    );
    assert!(matches!(err, AppError::Forbidden));
}

// --------------------------- reversing a payment ----------------------------

/// Helper: record a cash payment against `resident` with the standard fakes.
async fn pay(
    payments: &FakePaymentRepo,
    residents: &FakeResidentRepo,
    resident: pg_domain::ResidentId,
    amount: i64,
) {
    must(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: payments,
            resident_repo: residents,
            branch_repo: &FakeBranchRepo::with_count(1),
            user_repo: &FakeUserRepo::new(),
            store: &FakeObjectStore,
            renderer: &SpyRenderer::new(),
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Manager,
            actor: &actor(),
            resident_id: resident,
            amount_paise: amount,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "payment",
    );
}

#[tokio::test]
async fn void_last_payment__round_trips_paid_and_unpaid() {
    // The bug this covers: an accidental "mark as paid" used to be permanent,
    // because a payment row could only ever be added.
    let (residents, r) = seeded(|_| {});
    let payments = FakePaymentRepo::new();

    pay(&payments, &residents, r.id, 1_250_000).await;
    assert_eq!(payments.total_for(r.id.as_uuid()), 1_250_000, "paid");

    let voided = must(
        void_last_payment(&payments, &pooled_ctx(), Role::Manager, &actor(), r.id).await,
        "void",
    );
    assert_eq!(voided.amount_paise, 1_250_000);
    assert_eq!(payments.total_for(r.id.as_uuid()), 0, "back to unpaid");

    // ...and paying again works, so the status is not one-way in either
    // direction.
    pay(&payments, &residents, r.id, 1_250_000).await;
    assert_eq!(payments.total_for(r.id.as_uuid()), 1_250_000, "paid again");
}

#[tokio::test]
async fn void_last_payment__reverses_only_the_most_recent() {
    let (residents, r) = seeded(|_| {});
    let payments = FakePaymentRepo::new();
    pay(&payments, &residents, r.id, 1_000_000).await;
    pay(&payments, &residents, r.id, 250_000).await;

    let voided = must(
        void_last_payment(&payments, &pooled_ctx(), Role::Manager, &actor(), r.id).await,
        "void",
    );
    assert_eq!(voided.amount_paise, 250_000, "the latest, not the first");
    assert_eq!(
        payments.total_for(r.id.as_uuid()),
        1_000_000,
        "earlier payment survives"
    );
}

#[tokio::test]
async fn void_last_payment__nothing_to_reverse__is_a_conflict_not_a_silent_ok() {
    let (_residents, r) = seeded(|_| {});
    let err = void_last_payment(
        &FakePaymentRepo::new(),
        &pooled_ctx(),
        Role::Manager,
        &actor(),
        r.id,
    )
    .await
    .expect_err("no payment recorded");
    assert!(matches!(err, AppError::Conflict(_)), "got {err:?}");
}

#[tokio::test]
async fn void_last_payment__tenant__forbidden() {
    // Reversing money is staff-only, exactly like recording it.
    let (residents, r) = seeded(|_| {});
    let payments = FakePaymentRepo::new();
    pay(&payments, &residents, r.id, 500_000).await;

    let err = void_last_payment(&payments, &pooled_ctx(), Role::Tenant, &actor(), r.id)
        .await
        .expect_err("tenant");
    assert!(matches!(err, AppError::Forbidden), "got {err:?}");
    assert_eq!(
        payments.total_for(r.id.as_uuid()),
        500_000,
        "a refused reversal must not change the balance"
    );
}

// ------------------- receipt attribution & contact details -------------------

/// The shared `actor()` carries no user_id (most tests do not care who acted).
/// Receipt attribution does, so these use an identified actor.
fn identified_actor() -> pg_application::ports::AuditActor {
    pg_application::ports::AuditActor {
        user_id: Some(pg_domain::UserId::new(Uuid::new_v4())),
        request_id: None,
    }
}

#[tokio::test]
async fn record_payment__names_the_staff_member_who_took_the_cash() {
    // Every receipt used to read "Owner (Admin)" regardless of who collected,
    // which was plainly false for a manager-recorded payment.
    let (residents, r) = seeded(|_| {});
    let renderer = SpyRenderer::new();
    let users = FakeUserRepo::new();
    *users.expected_name.lock().expect("poisoned") = Some("Bhargav".to_string());

    must(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            branch_repo: &FakeBranchRepo::with_count(1),
            user_repo: &users,
            store: &FakeObjectStore,
            renderer: &renderer,
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Manager,
            actor: &identified_actor(),
            resident_id: r.id,
            amount_paise: 500_000,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "payment",
    );

    let seen = renderer.seen.lock().expect("poisoned");
    let data = seen.first().expect("rendered once");
    assert_eq!(data.recorded_by, "Bhargav (Manager)");
    assert_ne!(data.recorded_by, "Owner (Admin)", "the old hardcoded value");
}

#[tokio::test]
async fn record_payment__unnamed_staff__falls_back_to_role_not_another_title() {
    let (residents, r) = seeded(|_| {});
    let renderer = SpyRenderer::new();
    let users = FakeUserRepo::new();
    *users.expected_name.lock().expect("poisoned") = None;

    must(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            branch_repo: &FakeBranchRepo::with_count(1),
            user_repo: &users,
            store: &FakeObjectStore,
            renderer: &renderer,
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Owner,
            actor: &identified_actor(),
            resident_id: r.id,
            amount_paise: 500_000,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "payment",
    );

    let seen = renderer.seen.lock().expect("poisoned");
    assert_eq!(seen.first().expect("rendered once").recorded_by, "Owner");
}

#[tokio::test]
async fn record_payment__prints_the_residents_phone_when_known() {
    let (residents, r) = seeded(|r| r.phone = Some("9000010103".into()));
    let renderer = SpyRenderer::new();

    must(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            branch_repo: &FakeBranchRepo::with_count(1),
            user_repo: &FakeUserRepo::new(),
            store: &FakeObjectStore,
            renderer: &renderer,
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Owner,
            actor: &actor(),
            resident_id: r.id,
            amount_paise: 500_000,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "payment",
    );

    let seen = renderer.seen.lock().expect("poisoned");
    assert_eq!(
        seen.first().expect("rendered once").tenant_phone.as_deref(),
        Some("9000010103"),
    );
}

#[tokio::test]
async fn record_payment__resident_without_phone__omits_it_rather_than_failing() {
    let (residents, r) = seeded(|r| r.phone = None);
    let renderer = SpyRenderer::new();

    must(
        record_payment(RecordPaymentCommand {
            template_repo: &FakeReceiptTemplateRepo,
            payment_repo: &FakePaymentRepo::new(),
            resident_repo: &residents,
            branch_repo: &FakeBranchRepo::with_count(1),
            user_repo: &FakeUserRepo::new(),
            store: &FakeObjectStore,
            renderer: &renderer,
            clock: &FixedClock::at(NOV_05.date()),
            ctx: &pooled_ctx(),
            role: Role::Owner,
            actor: &actor(),
            resident_id: r.id,
            amount_paise: 500_000,
            method: "cash",
            reference: None,
            idempotency_key: None,
        })
        .await,
        "payment",
    );

    let seen = renderer.seen.lock().expect("poisoned");
    assert!(seen.first().expect("rendered once").tenant_phone.is_none());
}
