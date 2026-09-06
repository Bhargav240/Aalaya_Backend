use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::resolve_ctx;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use pg_application::services;
use pg_application::services::ReceiptDownload;
use pg_domain::ReceiptId;
use pg_domain::Role;
use serde::Serialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/me/profile", axum::routing::post(update_profile))
        .route("/me/memberships", get(memberships))
        .route("/me/receipts", get(my_receipts))
        .route("/me/receipts/{id}/download", get(my_receipt_download))
        .route("/me/ledger", get(my_ledger))
}

#[derive(serde::Deserialize)]
struct UpdateProfileReq {
    full_name: Option<String>,
    phone: Option<String>,
}

async fn update_profile(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<UpdateProfileReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    deps.user_repo
        .update_profile(claims.user_id, req.full_name.as_deref(), req.phone.as_deref())
        .await?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

#[derive(Serialize)]
struct Me {
    user_id: String,
    org_id: String,
    role: String,
    branch_id: Option<String>,
    full_name: Option<String>,
    verified: bool,
}

async fn me(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Me>, ApiError> {
    let deps = state.deps()?;
    let branch_id = deps
        .user_repo
        .branch_in_org(claims.user_id, claims.org_id)
        .await?;
    let full_name = deps.user_repo.expected_name(claims.user_id).await?;
    let verified = deps.user_repo.is_verified(claims.user_id).await?;
    Ok(Json(Me {
        user_id: claims.user_id.to_string(),
        org_id: claims.org_id.to_string(),
        role: claims.role.as_str().to_string(),
        branch_id: branch_id.map(|id| id.to_string()),
        full_name,
        verified,
    }))
}

#[derive(Serialize)]
struct MembershipOut {
    membership_id: String,
    organization_id: String,
    organization_name: String,
    role: String,
    branch_id: Option<String>,
}

async fn memberships(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<MembershipOut>>, ApiError> {
    let deps = state.deps()?;
    let contexts = deps
        .user_repo
        .list_active_memberships(claims.user_id)
        .await?;

    // A prospect membership is a marketplace lead, not somewhere to work from:
    // switching into one lands the caller in the three-tab explore shell with no
    // dashboard, ledger or residents. Nearly every real account carries one,
    // because any unknown address that requests a login code is auto-created as
    // a prospect (see `create_email_explorer`) — and that row usually sits in a
    // different org than the person's actual role. Listing it therefore offers
    // an owner a switch that only takes capability away.
    //
    // Someone who is *only* a prospect keeps their rows: that is a legitimate
    // marketplace identity, and filtering it would leave them an empty switcher.
    //
    // This governs what the switcher lists; it is not an authorization check.
    // `switch_organization` validates independently via `has_active_membership`,
    // so a caller naming a prospect membership directly is still admitted.
    let has_non_prospect_role = contexts.iter().any(|c| c.role != Role::Prospect);

    Ok(Json(
        contexts
            .into_iter()
            .filter(|context| !has_non_prospect_role || context.role != Role::Prospect)
            .map(|context| MembershipOut {
                membership_id: context.membership_id.to_string(),
                organization_id: context.org_id.to_string(),
                organization_name: context.organization_name,
                role: context.role.as_str().to_string(),
                branch_id: context.branch_id.map(|id| id.to_string()),
            })
            .collect(),
    ))
}

/// Tenant self-service PDF download for the caller's OWN receipts.
async fn my_receipt_download(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(receipt_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let target = services::my_receipt_download(
        deps.payment_repo.as_ref(),
        deps.resident_repo.as_ref(),
        deps.store.as_ref(),
        &ctx,
        claims.user_id,
        ReceiptId::new(receipt_id),
        deps.receipt_url_ttl_secs,
    )
    .await?;
    Ok(match target {
        ReceiptDownload::Redirect(url) => Redirect::temporary(&url).into_response(),
        ReceiptDownload::Inline(bytes) => {
            ([(header::CONTENT_TYPE, "application/pdf")], bytes).into_response()
        }
    })
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

async fn my_receipts(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<ReceiptOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let items = services::my_receipts(
        deps.payment_repo.as_ref(),
        deps.resident_repo.as_ref(),
        &ctx,
        claims.user_id,
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
    branch_name: String,
    room_number: Option<String>,
    bed_label: Option<String>,
    plan_kind: String,
    status: String,
    rent_paise: i64,
    deposit_paise: i64,
    paid_total_paise: i64,
    accrued_paise: i64,
    outstanding_paise: i64,
}

async fn my_ledger(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<LedgerOut>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let l = services::my_ledger(
        deps.payment_repo.as_ref(),
        deps.resident_repo.as_ref(),
        deps.billing.as_ref(),
        deps.clock.as_ref(),
        &ctx,
        claims.user_id,
    )
    .await?;
    Ok(Json(LedgerOut {
        full_name: l.full_name,
        branch_id: l.branch_id,
        branch_name: l.branch_name,
        room_number: l.room_number,
        bed_label: l.bed_label,
        plan_kind: l.plan_kind,
        status: l.status,
        rent_paise: l.rent_paise,
        deposit_paise: l.deposit_paise,
        paid_total_paise: l.paid_total_paise,
        accrued_paise: l.accrued_paise,
        outstanding_paise: l.outstanding_paise,
    }))
}
