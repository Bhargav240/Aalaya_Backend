use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use pg_application::ports::storage::ReceiptTemplate;
use pg_application::services;
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new().route("/org/receipt-template", get(get_template).put(put_template))
}

#[derive(Serialize, Deserialize)]
struct TemplateBody {
    display_name: String,
    address: Option<String>,
    footer: Option<String>,
    accent: Option<String>,
    logo_key: Option<String>,
}

async fn get_template(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<TemplateBody>, ApiError> {
    let deps = state.deps()?;
    let t = services::get_receipt_template(deps.template_repo.as_ref(), claims.role, claims.org_id)
        .await?;
    Ok(Json(TemplateBody {
        display_name: t.display_name,
        address: t.address,
        footer: t.footer,
        accent: t.accent,
        logo_key: t.logo_key,
    }))
}

async fn put_template(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<TemplateBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let template = ReceiptTemplate {
        display_name: body.display_name,
        address: body.address,
        footer: body.footer,
        accent: body.accent,
        logo_key: body.logo_key,
    };
    services::set_receipt_template(
        deps.template_repo.as_ref(),
        claims.role,
        claims.org_id,
        template,
    )
    .await?;
    Ok(Json(serde_json::json!({"status":"saved"})))
}
