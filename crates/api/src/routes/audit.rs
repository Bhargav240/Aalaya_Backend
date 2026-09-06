use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::resolve_ctx;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use pg_application::services;
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new().route("/audit", get(list))
}

#[derive(Deserialize)]
struct AuditQuery {
    limit: Option<i64>,
}

#[derive(Serialize)]
struct AuditOut {
    id: i64,
    action: String,
    entity_type: Option<String>,
    entity_id: Option<String>,
    actor_id: Option<String>,
    actor_name: Option<String>,
    at_epoch: i64,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let items = services::list_audit(deps.audit_repo.as_ref(), &ctx, claims.role, limit).await?;
    Ok(Json(
        items
            .into_iter()
            .map(|a| AuditOut {
                id: a.id,
                action: a.action,
                entity_type: a.entity_type,
                entity_id: a.entity_id.map(|e| e.to_string()),
                actor_id: a.actor_id.map(|e| e.to_string()),
                actor_name: a.actor_name,
                at_epoch: a.created_at_epoch,
            })
            .collect(),
    ))
}
