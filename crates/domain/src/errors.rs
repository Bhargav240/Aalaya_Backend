use thiserror::Error;

/// Errors that arise from business rules. Infrastructure/transport errors live
/// elsewhere -- these are the domain's own vocabulary.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("not found")]
    NotFound,
    #[error("invalid state transition: {0}")]
    InvalidTransition(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("forbidden")]
    Forbidden,
}
