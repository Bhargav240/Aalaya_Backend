use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::resolve_ctx;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use pg_application::context::current_request_id;
use pg_application::services;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/change-requests", get(list_pending))
        .route("/change-requests/{id}/approve", post(approve))
        .route("/change-requests/{id}/reject", post(reject))
}

#[derive(Serialize)]
struct ChangeRequestOut {
    id: String,
    entity_type: String,
    entity_id: String,
    operation: String,
    status: String,
    requested_by: String,
}

async fn list_pending(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<ChangeRequestOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let items = services::list_pending_changes(deps.cr_repo.as_ref(), &ctx, claims.role).await?;
    Ok(Json(
        items
            .into_iter()
            .map(|c| ChangeRequestOut {
                id: c.id.to_string(),
                entity_type: c.entity_type,
                entity_id: c.entity_id.to_string(),
                operation: c.operation,
                status: c.status,
                requested_by: c.requested_by.to_string(),
            })
            .collect(),
    ))
}

async fn approve(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    services::approve_change(
        deps.cr_repo.as_ref(),
        &ctx,
        claims.role,
        claims.user_id,
        current_request_id(),
        id,
    )
    .await?;
    Ok(Json(json!({"status":"approved"})))
}

async fn reject(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    services::reject_change(deps.cr_repo.as_ref(), &ctx, claims.role, claims.user_id, id).await?;
    Ok(Json(json!({"status":"rejected"})))
}
