use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{actor_of, resolve_ctx};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use pg_application::{services, AppError};
use pg_domain::{BranchId, ComplaintId, ComplaintState};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/complaints", get(list).post(raise))
        .route("/complaints/{id}/transition", post(transition))
}

#[derive(Deserialize)]
struct RaiseReq {
    title: String,
    description: Option<String>,
    business_level: Option<bool>,
    branch_id: Option<String>,
}

async fn raise(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<RaiseReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let actor = actor_of(&claims);
    let branch_id = match req.branch_id {
        Some(s) => {
            Some(BranchId::new(Uuid::parse_str(&s).map_err(|_| {
                AppError::Validation("invalid branch_id".into())
            })?))
        }
        None => None,
    };
    let id = services::raise_complaint(services::RaiseComplaintCommand {
        complaint_repo: deps.complaint_repo.as_ref(),
        user_repo: deps.user_repo.as_ref(),
        ctx: &ctx,
        role: claims.role,
        actor: &actor,
        user_id: claims.user_id,
        title: req.title,
        description: req.description,
        business_level: req.business_level.unwrap_or(false),
        branch_id_req: branch_id,
    })
    .await?;
    Ok(Json(json!({ "id": id.to_string() })))
}

#[derive(Serialize)]
struct ComplaintOut {
    id: String,
    branch_id: Option<String>,
    raised_by: String,
    title: String,
    description: Option<String>,
    state: String,
    is_business_level: bool,
    /// Epoch seconds, matching every other timestamp the API returns.
    created_epoch: i64,
    updated_epoch: i64,
    tenant_name: Option<String>,
    floor_number: Option<String>,
    room_number: Option<String>,
}

#[derive(Deserialize)]
struct ListComplaintsQuery {
    branch_id: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    axum::extract::Query(query): axum::extract::Query<ListComplaintsQuery>,
) -> Result<Json<Vec<ComplaintOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let branch_id = match query.branch_id {
        Some(s) if !s.trim().is_empty() => {
            Some(BranchId::new(Uuid::parse_str(&s).map_err(|_| {
                AppError::Validation("invalid branch_id".into())
            })?))
        }
        _ => None,
    };
    let items = services::list_complaints(
        deps.complaint_repo.as_ref(),
        deps.user_repo.as_ref(),
        &ctx,
        claims.role,
        claims.user_id,
        branch_id,
    )
    .await?;
    Ok(Json(
        items
            .into_iter()
            .map(|c| ComplaintOut {
                id: c.id.to_string(),
                branch_id: c.branch_id.map(|b| b.to_string()),
                raised_by: c.raised_by.to_string(),
                title: c.title,
                description: c.description,
                state: c.state,
                is_business_level: c.is_business_level,
                created_epoch: c.created_at.unix_timestamp(),
                updated_epoch: c.updated_at.unix_timestamp(),
                tenant_name: c.tenant_name,
                floor_number: c.floor_number,
                room_number: c.room_number,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct TransitionReq {
    to: String,
}

async fn transition(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<TransitionReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let actor = actor_of(&claims);
    let to = ComplaintState::parse(&req.to)
        .ok_or(AppError::Validation("invalid target state".into()))?;
    services::transition_complaint(services::TransitionComplaintCommand {
        complaint_repo: deps.complaint_repo.as_ref(),
        user_repo: deps.user_repo.as_ref(),
        ctx: &ctx,
        role: claims.role,
        actor: &actor,
        user_id: claims.user_id,
        complaint_id: ComplaintId::new(id),
        to,
    })
    .await?;
    Ok(Json(json!({ "status": "ok", "state": req.to })))
}
