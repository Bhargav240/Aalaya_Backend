//! Device registration for push (FCM tokens) — one call on app start.
use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::{delete, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me/devices", post(register))
        .route("/me/devices/{token}", delete(unregister))
}

#[derive(Deserialize)]
struct RegisterReq {
    token: String,
    platform: Option<String>,
}

async fn register(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<RegisterReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    deps.device_token_repo
        .register(
            Uuid::new_v4(),
            claims.user_id,
            claims.org_id,
            req.token.trim(),
            req.platform.as_deref().unwrap_or("android"),
        )
        .await?;
    Ok(Json(json!({ "status": "registered" })))
}

async fn unregister(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    axum::extract::Path(token): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    deps.device_token_repo
        .unregister(claims.user_id, &token)
        .await?;
    Ok(Json(json!({ "status": "removed" })))
}
