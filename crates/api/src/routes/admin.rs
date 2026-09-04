use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{actor_of, resolve_ctx};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use pg_application::services;
use serde::Deserialize;
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/archive", post(archive))
        .route("/admin/promote", post(promote))
}

#[derive(Deserialize)]
struct ArchiveReq {
    older_than_days: Option<i64>,
}

/// Move this org's soft-deleted rows (older than the cutoff) to cold storage
/// and purge them from the hot DB. Owner-only, manual trigger.
async fn archive(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ArchiveReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let actor = actor_of(&claims);
    let summary = services::archive_deleted(services::ArchiveDeletedCommand {
        archive_repo: deps.archive_repo.as_ref(),
        store: deps.store.as_ref(),
        clock: deps.clock.as_ref(),
        ctx: &ctx,
        role: claims.role,
        actor: &actor,
        older_than_days: req.older_than_days.unwrap_or(30),
    })
    .await?;
    let total: i64 = summary.values().sum();
    Ok(Json(json!({ "archived": summary, "total": total })))
}

/// Promote the caller's org from the shared pool into its own dedicated schema,
/// backfilling its data. Owner-only.
async fn promote(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let schema = services::promote_org(
        deps.provisioner.as_ref(),
        deps.resolver.as_ref(),
        claims.role,
        claims.org_id,
    )
    .await?;
    Ok(Json(
        json!({ "status": "promoted", "mode": "siloed", "schema": schema }),
    ))
}
