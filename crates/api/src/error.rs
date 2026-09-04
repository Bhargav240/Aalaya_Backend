//! The single top-level error boundary. Every failure funnels through here,
//! which (1) maps it to an RFC-7807 `problem+json` response and (2) logs it --
//! so handlers carry no error-shaping or logging boilerplate.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use pg_application::AppError;
use pg_domain::DomainError;
use serde::Serialize;

#[derive(Serialize)]
struct Problem {
    #[serde(rename = "type")]
    kind: &'static str,
    title: String,
    status: u16,
}

pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        ApiError(e)
    }
}

/// Strip raw SQL constraint names from Conflict messages: log the raw name,
/// show the user a stable, safe title.
fn conflict_title(raw: &str) -> String {
    // Well-known user-safe translations for the constraints we know about.
    let friendly = match raw {
        "residents_one_live_per_bed_uidx" => Some("that bed is already occupied"),
        "platform_users_phone_uidx" => Some("phone number already registered"),
        "payments_idempotency_uidx" => {
            Some("this request was already submitted with a different amount or resident")
        }
        "public_listings_branch_uidx" | "listings_branch_uidx" => {
            Some("a listing already exists for this branch")
        }
        _ if raw.ends_with("_pkey") => Some("duplicate key"),
        _ => None,
    };
    if friendly.is_none() {
        tracing::warn!(constraint = %raw, "untranslated conflict constraint");
    }
    friendly.unwrap_or(raw).to_string()
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, title) = match &self.0 {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden".to_string()),
            AppError::NotFound | AppError::Domain(DomainError::NotFound) => {
                (StatusCode::NOT_FOUND, "not found".to_string())
            }
            AppError::Conflict(m) => (StatusCode::CONFLICT, conflict_title(m)),
            AppError::Validation(m) | AppError::Domain(DomainError::Validation(m)) => {
                (StatusCode::BAD_REQUEST, m.clone())
            }
            AppError::Domain(d) => (StatusCode::BAD_REQUEST, d.to_string()),
            // Tenancy resolution failure is infrastructure, not client misuse.
            AppError::Tenancy(m) => {
                tracing::error!(tenancy = %m, "tenancy resolution failed");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "tenant unavailable".to_string(),
                )
            }
            // The org exists but is administratively blocked (e.g. suspended)
            // -- distinct from an infra failure, so it's a stable 403, not 503.
            AppError::OrgSuspended(status) => {
                tracing::warn!(org_status = %status, "request rejected: organization not active");
                (
                    StatusCode::FORBIDDEN,
                    "organization is not active".to_string(),
                )
            }
            AppError::Persistence(_) | AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal error".to_string(),
            ),
        };

        if status.is_server_error() {
            tracing::error!(error = ?self.0, "unhandled server error");
        } else {
            tracing::warn!(error = %self.0, "request error");
        }

        let body = Problem {
            kind: "about:blank",
            title,
            status: status.as_u16(),
        };
        (status, Json(body)).into_response()
    }
}
