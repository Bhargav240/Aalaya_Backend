use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use axum::{extract::State, middleware::from_fn_with_state, routing::post, Json, Router};
use pg_application::services;
use pg_domain::{OrgId, Role};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub fn router() -> Router<AppState> {
    let sensitive = Router::new()
        .route("/auth/email/send-otp", post(request_email_otp))
        .route("/auth/email/verify-otp", post(verify_email_otp))
        .route_layer(from_fn_with_state(
            crate::middleware::rate_limit::RateLimiter::new(
                100,
                std::time::Duration::from_secs(15 * 60),
            ),
            crate::middleware::rate_limit::enforce,
        ));

    let r = Router::new()
        .merge(sensitive)
        .route("/auth/refresh", post(refresh))
        .route("/auth/switch-organization", post(switch_organization))
        .route("/auth/logout", post(logout))
        .route("/auth/verify-identity", post(verify_identity))
        .route("/auth/google", post(google_sign_in))
        .route_layer(from_fn_with_state(
            crate::middleware::rate_limit::RateLimiter::new(
                300,
                std::time::Duration::from_secs(15 * 60),
            ),
            crate::middleware::rate_limit::enforce,
        ));

    // Dev-only direct login — no OTP, no rate limit. Debug builds only.
    #[cfg(debug_assertions)]
    let r = r.merge(Router::new().route("/auth/dev-login", post(dev_login)));

    r
}

#[derive(Serialize)]
struct LoginResp {
    access_token: String,
    refresh_token: String,
    is_verified: bool,
    is_new_device: bool,
}

#[derive(Deserialize)]
struct RequestEmailOtpReq {
    email: String,
}

/// Sign-in step 1: send a one-time code by email.
async fn request_email_otp(
    State(state): State<AppState>,
    Json(req): Json<RequestEmailOtpReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    services::request_email_otp(services::RequestEmailOtpCommand {
        user_repo: deps.user_repo.as_ref(),
        otp_repo: deps.email_otp_repo.as_ref(),
        codegen: deps.otp_codegen.as_ref(),
        sender: deps.email_sender.as_ref(),
        platform_org_id: deps.platform_org_id,
        resend_cooldown_secs: deps.otp_resend_cooldown_secs,
        max_sends: deps.email_otp_max_sends,
        window_secs: deps.email_otp_window_secs,
        otp_ttl_secs: deps.otp_ttl_secs,
        email: &req.email,
    })
    .await?;
    Ok(Json(json!({ "status": "sent" })))
}

#[derive(Deserialize)]
struct VerifyEmailOtpReq {
    email: String,
    code: String,
}

/// Sign-in step 2: code in, tokens out. Tells the caller if this is the
/// account's first sight of this hardware (`is_new_device`) and whether the
/// resident has passed identity verification yet.
async fn verify_email_otp(
    State(state): State<AppState>,
    Json(req): Json<VerifyEmailOtpReq>,
) -> Result<Json<LoginResp>, ApiError> {
    let deps = state.deps()?;
    let tokens = services::verify_email_otp(services::VerifyEmailOtpCommand {
        user_repo: deps.user_repo.as_ref(),
        otp_repo: deps.email_otp_repo.as_ref(),
        tokens: deps.tokens.as_ref(),
        refresh_repo: deps.refresh_repo.as_ref(),
        refresh_ttl_secs: deps.refresh_ttl_secs,
        otp_max_attempts: deps.otp_max_attempts,
        email: &req.email,
        code: req.code.trim(),
    })
    .await?;
    Ok(Json(LoginResp {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        is_verified: true,
        is_new_device: false,
    }))
}

#[derive(Deserialize)]
struct GoogleReq {
    id_token: String,
}

async fn google_sign_in(
    State(state): State<AppState>,
    Json(req): Json<GoogleReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let pair = services::google_sign_in(
        deps.google_verifier.as_ref(),
        deps.user_repo.as_ref(),
        deps.tokens.as_ref(),
        deps.refresh_repo.as_ref(),
        deps.refresh_ttl_secs,
        req.id_token.trim(),
    )
    .await?;
    Ok(Json(json!({
        "access_token": pair.access_token,
        "refresh_token": pair.refresh_token,
        "is_verified": true,
        "is_new_device": false,
    })))
}

#[derive(Deserialize)]
struct RefreshReq {
    refresh_token: String,
}

async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let pair = services::refresh(services::RefreshCommand {
        user_repo: deps.user_repo.as_ref(),
        tokens: deps.tokens.as_ref(),
        refresh_repo: deps.refresh_repo.as_ref(),
        refresh_ttl_secs: deps.refresh_ttl_secs,
        refresh_token: &req.refresh_token,
    })
    .await?;
    Ok(Json(json!({
        "access_token": pair.access_token,
        "refresh_token": pair.refresh_token,
    })))
}

#[derive(Deserialize)]
struct SwitchOrganizationReq {
    organization_id: uuid::Uuid,
    role: String,
    refresh_token: String,
}

async fn switch_organization(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<SwitchOrganizationReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::parse(&req.role).ok_or_else(|| {
        pg_application::AppError::Validation("unsupported membership role".into())
    })?;
    let deps = state.deps()?;
    let pair = services::switch_organization(services::SwitchOrganizationCommand {
        user_repo: deps.user_repo.as_ref(),
        tokens: deps.tokens.as_ref(),
        refresh_repo: deps.refresh_repo.as_ref(),
        refresh_ttl_secs: deps.refresh_ttl_secs,
        actor_user_id: claims.user_id,
        target_org_id: OrgId::new(req.organization_id),
        target_role: role,
        refresh_token: req.refresh_token.trim(),
    })
    .await?;
    Ok(Json(json!({
        "access_token": pair.access_token,
        "refresh_token": pair.refresh_token,
    })))
}

async fn logout(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    services::logout(deps.refresh_repo.as_ref(), claims.user_id).await?;
    // Revoke *this* access token's jti so a replayed/stolen one can't be used
    // for the remainder of its TTL. Bounded by the access-token TTL, and the
    // blocklist is opportunistically GC'd on insert.
    state.revoke_jti(&claims.jti, claims.exp);
    state.invalidate_membership_cache(claims.user_id);
    Ok(Json(json!({ "status": "logged_out" })))
}

#[derive(Deserialize)]
struct VerifyReq {
    method: Option<String>,
    provider_name: Option<String>,
    provider_ref: String,
    phone: Option<String>,
    emergency_contact: Option<String>,
    country: Option<String>,
}

#[derive(Serialize)]
struct VerifyResp {
    status: String,
    reason: Option<String>,
}

#[cfg(debug_assertions)]
#[derive(Deserialize)]
struct DevLoginReq {
    email: String,
}

/// Dev-only: issue tokens for any active user by email with no OTP step.
/// Only compiled into debug builds; never present in release binaries.
#[cfg(debug_assertions)]
async fn dev_login(
    State(state): State<AppState>,
    Json(req): Json<DevLoginReq>,
) -> Result<Json<LoginResp>, ApiError> {
    let deps = state.deps()?;
    let email = req.email.trim().to_lowercase();
    let user =
        deps.user_repo.find_by_email(&email).await?.ok_or_else(|| {
            pg_application::AppError::Validation("no account with that email".into())
        })?;
    let pair = deps
        .tokens
        .issue_pair(user.user_id, user.org_id, user.role)?;
    deps.refresh_repo
        .store(
            &services::sha256_hex(&pair.refresh_token),
            user.user_id,
            user.org_id,
            deps.refresh_ttl_secs,
        )
        .await?;
    Ok(Json(LoginResp {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        is_verified: true,
        is_new_device: false,
    }))
}

async fn verify_identity(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<VerifyReq>,
) -> Result<Json<VerifyResp>, ApiError> {
    let deps = state.deps()?;
    let out = services::verify_identity(services::VerifyIdentityCommand {
        verifier: deps.verifier.as_ref(),
        user_repo: deps.user_repo.as_ref(),
        verification_repo: deps.verification_repo.as_ref(),
        default_method: &deps.verification_method,
        user_id: claims.user_id,
        org_id: claims.org_id,
        input: services::VerifyInput {
            method: req.method,
            provider_name: req.provider_name,
            provider_ref: req.provider_ref,
            phone: req.phone,
            emergency_contact: req.emergency_contact,
            country: req.country,
        },
    })
    .await?;
    Ok(Json(VerifyResp {
        status: out.status,
        reason: out.reason,
    }))
}
