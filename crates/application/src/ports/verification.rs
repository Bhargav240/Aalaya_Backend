//! Identity verification port. DigiLocker is one adapter; the registry in
//! infrastructure dispatches by `method`, so more providers (passport, other
//! national IDs, KYC vendors) can be added per region without changing callers.
use crate::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

pub struct VerificationRequest {
    pub method: String,
    /// The registered (expected) Aadhaar name, for name matching.
    pub expected_name: Option<String>,
    /// The name the provider reports (mock/dev supplies this; a real adapter
    /// fetches it from the provider).
    pub provider_name: Option<String>,
    pub provider_ref: String,
    pub phone: Option<String>,
    pub emergency_contact: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    Verified,
    Failed,
}

pub struct VerificationOutcome {
    pub status: VerificationStatus,
    pub verified_name: Option<String>,
    pub reason: Option<String>,
}

#[async_trait]
pub trait IdentityVerifier: Send + Sync {
    async fn verify(&self, req: &VerificationRequest) -> Result<VerificationOutcome, AppError>;
}

pub struct DigiLockerIdentity {
    pub provider_reference: String,
    pub document_type: Option<String>,
    pub nonce: String,
}

#[async_trait]
pub trait DigiLockerClient: Send + Sync {
    fn authorization_url(
        &self,
        state: &str,
        nonce: &str,
        code_challenge: &str,
    ) -> Result<String, AppError>;

    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<DigiLockerIdentity, AppError>;
}

pub struct DigiLockerSession {
    pub invitation_id: Uuid,
    pub user_id: pg_domain::UserId,
    pub nonce_hash: String,
    pub pkce_verifier: String,
}

pub struct NewDigiLockerSession<'a> {
    pub invitation_id: Uuid,
    pub user_id: pg_domain::UserId,
    pub state_hash: &'a str,
    pub nonce_hash: &'a str,
    pub pkce_verifier: &'a str,
    pub encryption_key: &'a str,
    pub expires_at: time::OffsetDateTime,
}

pub struct InvitationVerification {
    pub provider: String,
    pub provider_reference: Option<String>,
    pub document_type: Option<String>,
    pub status: String,
    pub verified_at: Option<time::OffsetDateTime>,
    pub safe_failure_reason: Option<String>,
}

#[async_trait]
pub trait InvitationVerificationRepository: Send + Sync {
    async fn create_digilocker_session(
        &self,
        session: NewDigiLockerSession<'_>,
    ) -> Result<(), AppError>;

    async fn consume_digilocker_session(
        &self,
        state_hash: &str,
        encryption_key: &str,
    ) -> Result<Option<DigiLockerSession>, AppError>;

    async fn complete_digilocker(
        &self,
        invitation_id: Uuid,
        provider_reference: Option<&str>,
        document_type: Option<&str>,
        verified: bool,
        safe_failure_reason: Option<&str>,
    ) -> Result<(), AppError>;

    async fn latest(&self, invitation_id: Uuid)
        -> Result<Option<InvitationVerification>, AppError>;

    async fn manual_review(
        &self,
        invitation_id: Uuid,
        reviewer_user_id: pg_domain::UserId,
        approved: bool,
        reason: &str,
        request_id: Option<Uuid>,
    ) -> Result<(), AppError>;
}
