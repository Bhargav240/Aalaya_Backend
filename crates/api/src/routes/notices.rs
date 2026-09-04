use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{actor_of, resolve_ctx};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use pg_application::{services, AppError};
use pg_domain::BranchId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/notices", get(list).post(create))
}

#[derive(Deserialize)]
struct CreateNoticeReq {
    title: String,
    body: Option<String>,
    branch_id: Option<String>,
}

async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateNoticeReq>,
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
    let id = services::create_notice(services::CreateNoticeCommand {
        notice_repo: deps.notice_repo.as_ref(),
        user_repo: deps.user_repo.as_ref(),
        notifier: deps.notifier.as_ref(),
        ctx: &ctx,
        role: claims.role,
        actor: &actor,
        user_id: claims.user_id,
        branch_id_req: branch_id,
        title: req.title,
        body: req.body,
    })
    .await?;
    Ok(Json(json!({ "id": id.to_string() })))
}

#[derive(Serialize)]
struct NoticeOut {
    id: String,
    branch_id: Option<String>,
    title: String,
    body: Option<String>,
    created_epoch: i64,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<NoticeOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let items = services::list_notices(
        deps.notice_repo.as_ref(),
        deps.user_repo.as_ref(),
        deps.resident_repo.as_ref(),
        &ctx,
        claims.role,
        claims.user_id,
    )
    .await?;
    Ok(Json(
        items
            .into_iter()
            .map(|n| NoticeOut {
                id: n.id.to_string(),
                branch_id: n.branch_id.map(|b| b.to_string()),
                title: n.title,
                body: n.body,
                created_epoch: n.created_epoch,
            })
            .collect(),
    ))
}
