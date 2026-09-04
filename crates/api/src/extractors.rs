//! `AuthUser` extracts and verifies the Bearer access token, yielding Claims.
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use pg_application::ports::auth::{Claims, TokenKind};
use pg_application::AppError;

pub struct AuthUser(pub Claims);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let deps = state.deps()?;
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        let claims = deps
            .tokens
            .verify(token)
            .map_err(|_| AppError::Unauthorized)?;
        if claims.kind != TokenKind::Access {
            return Err(AppError::Unauthorized.into());
        }
        // Explicitly revoked access tokens (logout / security action) are
        // rejected even if their exp is still valid. Without this, a stolen
        // access token stayed usable until its TTL ran out.
        if state.is_jti_revoked(&claims.jti) {
            return Err(AppError::Unauthorized.into());
        }
        if !state
            .check_active_membership(claims.user_id, claims.org_id, claims.role)
            .await?
        {
            return Err(AppError::Unauthorized.into());
        }
        Ok(AuthUser(claims))
    }
}
