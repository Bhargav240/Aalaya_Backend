//! Identity verification use case. Name mismatch => FAIL. Provider selection is
//! delegated to the verifier (a registry), so worldwide methods extend cleanly.
use crate::error::AppError;
use crate::ports::repositories::{UserRepository, VerificationRepository};
use crate::ports::verification::{IdentityVerifier, VerificationRequest, VerificationStatus};
use pg_domain::{OrgId, UserId};
use uuid::Uuid;

pub struct VerifyInput {
    pub method: Option<String>,
    pub provider_name: Option<String>,
    pub provider_ref: String,
    pub phone: Option<String>,
    pub emergency_contact: Option<String>,
    pub country: Option<String>,
}

pub struct VerifyResult {
    pub status: String,
    pub reason: Option<String>,
}

/// All dependencies and request-scoped values needed for one identity
/// verification. Grouping them makes the use case's transaction boundary
/// explicit without introducing a long-lived service object or new trait.
pub struct VerifyIdentityCommand<'a> {
    pub verifier: &'a dyn IdentityVerifier,
    pub user_repo: &'a dyn UserRepository,
    pub verification_repo: &'a dyn VerificationRepository,
    pub default_method: &'a str,
    pub user_id: UserId,
    pub org_id: OrgId,
    pub input: VerifyInput,
}

fn normalize(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub async fn verify_identity(command: VerifyIdentityCommand<'_>) -> Result<VerifyResult, AppError> {
    let expected = command.user_repo.expected_name(command.user_id).await?;
    let method = command
        .input
        .method
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| command.default_method.to_string());

    let outcome = command
        .verifier
        .verify(&VerificationRequest {
            method: method.clone(),
            expected_name: expected.clone(),
            provider_name: command.input.provider_name.clone(),
            provider_ref: command.input.provider_ref.clone(),
            phone: command.input.phone.clone(),
            emergency_contact: command.input.emergency_contact.clone(),
            country: command.input.country.clone(),
        })
        .await?;

    // Provider result, then the name-match gate: a mismatch always fails.
    let (status, reason, verified_name) = match outcome.status {
        VerificationStatus::Failed => (
            "failed",
            outcome.reason.clone(),
            outcome.verified_name.clone(),
        ),
        VerificationStatus::Verified => match (&expected, &outcome.verified_name) {
            (Some(exp), Some(got)) if normalize(exp) != normalize(got) => (
                "failed",
                Some("aadhaar name mismatch".to_string()),
                Some(got.clone()),
            ),
            _ => ("verified", None, outcome.verified_name.clone()),
        },
    };

    command
        .verification_repo
        .record(
            Uuid::new_v4(),
            command.user_id,
            command.org_id,
            &method,
            status,
            verified_name.as_deref(),
            command.input.phone.as_deref(),
            command.input.emergency_contact.as_deref(),
            reason.as_deref(),
        )
        .await?;

    if status == "verified" {
        command.user_repo.mark_verified(command.user_id).await?;
    }

    Ok(VerifyResult {
        status: status.to_string(),
        reason,
    })
}
