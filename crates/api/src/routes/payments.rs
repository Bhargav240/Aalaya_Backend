use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{actor_of, require_resident_management_scope, resolve_ctx};
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use pg_application::services;
use pg_domain::ResidentId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/residents/{id}/payments", post(record))
        .route("/residents/{id}/payments/void", post(void_last))
        .route("/residents/{id}/receipts", get(receipts))
        .route("/residents/{id}/ledger", get(ledger))
}

#[derive(Deserialize)]
struct PaymentReq {
    amount_paise: i64,
    /// How the money arrived. Absent means cash, which is what every existing
    /// client sends and what the collector does by default.
    #[serde(default)]
    method: Option<String>,
    /// Provider reference for a non-cash payment (a UPI transaction id, say).
    #[serde(default)]
    reference: Option<String>,
}

#[derive(Serialize)]
struct PaymentResp {
    payment_id: String,
    receipt_id: String,
    receipt_number: String,
    invoice_number: String,
}

async fn record(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(resident_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<PaymentReq>,
) -> Result<Json<PaymentResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_resident_management_scope(deps, &ctx, &claims, ResidentId::new(resident_id)).await?;
    let actor = actor_of(&claims);
    // Optional client-supplied Idempotency-Key makes retried POSTs a no-op
    // rather than charging the resident twice (B9).
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let out = services::record_payment(services::RecordPaymentCommand {
        template_repo: deps.template_repo.as_ref(),
        payment_repo: deps.payment_repo.as_ref(),
        resident_repo: deps.resident_repo.as_ref(),
        branch_repo: deps.branch_repo.as_ref(),
        user_repo: deps.user_repo.as_ref(),
        store: deps.store.as_ref(),
        renderer: deps.renderer.as_ref(),
        clock: deps.clock.as_ref(),
        ctx: &ctx,
        role: claims.role,
        actor: &actor,
        resident_id: ResidentId::new(resident_id),
        amount_paise: req.amount_paise,
        method: req.method.as_deref().unwrap_or("cash"),
        reference: req.reference.as_deref(),
        idempotency_key,
    })
    .await?;
    Ok(Json(PaymentResp {
        payment_id: out.payment_id.to_string(),
        receipt_id: out.receipt_id.to_string(),
        receipt_number: out.receipt_number.clone(),
        invoice_number: out.receipt_number,
    }))
}

#[derive(Serialize)]
struct VoidPaymentResp {
    payment_id: String,
    amount_paise: i64,
    invoice_number: Option<String>,
}

/// Undo the resident's latest payment — the reverse of `record`.
///
/// Branch-scoped exactly like recording one, so a manager can only reverse a
/// payment for a resident in their own branch.
async fn void_last(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(resident_id): Path<Uuid>,
) -> Result<Json<VoidPaymentResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_resident_management_scope(deps, &ctx, &claims, ResidentId::new(resident_id)).await?;
    let actor = actor_of(&claims);
    let out = services::void_last_payment(
        deps.payment_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        ResidentId::new(resident_id),
    )
    .await?;
    Ok(Json(VoidPaymentResp {
        payment_id: out.payment_id.to_string(),
        amount_paise: out.amount_paise,
        invoice_number: out.invoice_number,
    }))
}

#[derive(Serialize)]
struct ReceiptOut {
    id: String,
    receipt_number: String,
    invoice_number: String,
    amount_paise: i64,
    status: String,
    issued_epoch: i64,
}

async fn receipts(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(resident_id): Path<Uuid>,
) -> Result<Json<Vec<ReceiptOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_resident_management_scope(deps, &ctx, &claims, ResidentId::new(resident_id)).await?;
    let items = services::list_receipts(
        deps.payment_repo.as_ref(),
        &ctx,
        claims.role,
        ResidentId::new(resident_id),
    )
    .await?;
    Ok(Json(
        items
            .into_iter()
            .map(|r| ReceiptOut {
                id: r.id.to_string(),
                receipt_number: r.receipt_number.clone(),
                invoice_number: r.receipt_number,
                amount_paise: r.amount_paise,
                status: r.status,
                issued_epoch: r.issued_epoch,
            })
            .collect(),
    ))
}

#[derive(Serialize)]
struct LedgerOut {
    full_name: String,
    branch_id: String,
    plan_kind: String,
    status: String,
    rent_paise: i64,
    deposit_paise: i64,
    paid_total_paise: i64,
    accrued_paise: i64,
    outstanding_paise: i64,
}

async fn ledger(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(resident_id): Path<Uuid>,
) -> Result<Json<LedgerOut>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_resident_management_scope(deps, &ctx, &claims, ResidentId::new(resident_id)).await?;
    let l = services::ledger(
        deps.payment_repo.as_ref(),
        deps.resident_repo.as_ref(),
        deps.billing.as_ref(),
        deps.clock.as_ref(),
        &ctx,
        claims.role,
        ResidentId::new(resident_id),
    )
    .await?;
    Ok(Json(LedgerOut {
        full_name: l.full_name,
        branch_id: l.branch_id,
        plan_kind: l.plan_kind,
        status: l.status,
        rent_paise: l.rent_paise,
        deposit_paise: l.deposit_paise,
        paid_total_paise: l.paid_total_paise,
        accrued_paise: l.accrued_paise,
        outstanding_paise: l.outstanding_paise,
    }))
}
