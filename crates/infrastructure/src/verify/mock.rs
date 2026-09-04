//! Dev/test verifier. Echoes the provider-reported name so the service's
//! name-match gate can be exercised; `provider_ref == "fail"` simulates a
//! provider rejection.
use async_trait::async_trait;
use pg_application::ports::verification::{
    IdentityVerifier, VerificationOutcome, VerificationRequest, VerificationStatus,
};
use pg_application::AppError;

pub struct MockIdentityVerifier;

#[async_trait]
impl IdentityVerifier for MockIdentityVerifier {
    async fn verify(&self, req: &VerificationRequest) -> Result<VerificationOutcome, AppError> {
        if req.provider_ref == "fail" {
            return Ok(VerificationOutcome {
                status: VerificationStatus::Failed,
                verified_name: None,
                reason: Some("provider rejected".to_string()),
            });
        }
        let name = req
            .provider_name
            .clone()
            .or_else(|| req.expected_name.clone());
        Ok(VerificationOutcome {
            status: VerificationStatus::Verified,
            verified_name: name,
            reason: None,
        })
    }
}
