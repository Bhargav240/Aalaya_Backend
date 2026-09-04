use crate::error::ApiError;
use crate::state::AppState;
use axum::{extract::State, http::HeaderMap, routing::post, Json, Router};
use pg_application::{services, AppError};
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new().route("/orgs", post(provision))
}

#[derive(Deserialize)]
struct ProvisionReq {
    name: String,
    owner_email: String,
    owner_phone: Option<String>,
    owner_name: String,
}

#[derive(Serialize)]
struct ProvisionResp {
    org_id: String,
    owner_id: String,
}

/// Bootstrap an org + its owner + a default branch. Back-office only: guarded
/// by `x-provisioning-key` (no self-serve PG creation). The owner signs in
/// using the provisioned email and the normal email OTP flow.
async fn provision(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ProvisionReq>,
) -> Result<Json<ProvisionResp>, ApiError> {
    let deps = state.deps()?;

    let provided = headers
        .get("x-provisioning-key")
        .and_then(|v| v.to_str().ok());
    match &deps.provisioning_key {
        Some(expected) if provided == Some(expected.as_str()) => {}
        _ => return Err(AppError::Forbidden.into()),
    }

    let out = services::provision_org(
        deps.org_repo.as_ref(),
        deps.user_repo.as_ref(),
        deps.branch_repo.as_ref(),
        deps.resolver.as_ref(),
        services::ProvisionOrgInput {
            name: req.name,
            owner_email: req.owner_email,
            owner_phone: req.owner_phone,
            owner_name: req.owner_name,
        },
    )
    .await?;

    if let Some(layout) = &deps.onboarding_layout {
        let ctx = deps
            .resolver
            .resolve(out.org_id)
            .await
            .map_err(|error| AppError::Tenancy(error.to_string()))?;
        services::reconcile_onboarding_layout(
            layout,
            deps.room_repo.as_ref(),
            deps.bed_repo.as_ref(),
            deps.floor_repo.as_ref(),
            &ctx,
            out.branch_id,
            &pg_application::ports::AuditActor {
                user_id: Some(out.owner_id),
                request_id: None,
            },
        )
        .await?;
    }

    Ok(Json(ProvisionResp {
        org_id: out.org_id.to_string(),
        owner_id: out.owner_id.to_string(),
    }))
}
