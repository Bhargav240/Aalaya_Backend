//! The enquiry funnel: anonymous seekers verify a phone with OTP, and their
//! lead lands in the branch manager's inbox (owner sees the whole org). The
//! manager works the inbox; the owner sees all leads.
use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use pg_application::services;
use pg_domain::BranchId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Manager/owner inbox routes. Every handler here takes `AuthUser`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/enquiries", get(inbox))
        .route(
            "/listings/{branch_id}/enquire",
            axum::routing::post(enquire),
        )
        .route("/enquiries/{id}/status", axum::routing::post(set_status))
}

#[derive(Deserialize)]
struct EnquireReq {
    note: Option<String>,
}

/// Prospect step 2: the code from step 1 verifies the phone. On success the
/// lead is recorded for the listing's branch (and a Prospect identity exists
/// under the platform org).
async fn enquire(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
    Json(req): Json<EnquireReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    services::create_authenticated_enquiry(services::AuthenticatedEnquiryCommand {
        enquiry_repo: deps.enquiry_repo.as_ref(),
        branch_id: BranchId::new(branch_id),
        user_id: claims.user_id,
        note: req.note.as_deref(),
    })
    .await?;
    Ok(Json(json!({ "status": "received" })))
}

#[derive(Serialize)]
struct EnquiryOut {
    id: String,
    branch_id: String,
    phone: String,
    note: Option<String>,
    status: String,
    /// When the lead arrived. The repository has always selected this and the
    /// client has always required it; it was simply not carried across here,
    /// which failed decoding for any non-empty inbox.
    created_at_epoch: i64,
}

/// The manager's inbox (own branch) / owner's org inbox.
async fn inbox(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<EnquiryOut>>, ApiError> {
    let deps = state.deps()?;
    let list = services::list_enquiries(
        deps.enquiry_repo.as_ref(),
        deps.user_repo.as_ref(),
        claims.role,
        claims.user_id,
        claims.org_id,
    )
    .await?;
    Ok(Json(
        list.into_iter()
            .map(|e| EnquiryOut {
                id: e.id.to_string(),
                branch_id: e.branch_id.to_string(),
                phone: e.phone,
                note: e.note,
                status: e.status,
                created_at_epoch: e.created_epoch,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct StatusReq {
    status: String,
}

async fn set_status(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<StatusReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    services::set_enquiry_status(
        deps.enquiry_repo.as_ref(),
        claims.role,
        id,
        req.status.as_str(),
    )
    .await?;
    Ok(Json(json!({ "status": "ok" })))
}
