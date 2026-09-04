use pg_domain::DomainError;
use thiserror::Error;

/// Application-level error. The API layer owns the single mapping from this to
/// an HTTP `problem+json` response (the top-level error boundary).
#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("tenancy error: {0}")]
    Tenancy(String),
    #[error("organization is {0}")]
    OrgSuspended(String),
    #[error("persistence error: {0}")]
    Persistence(String),
    #[error("internal error: {0}")]
    Internal(String),
}
