//! Google sign-in port. The adapter verifies Google's RS256 identity tokens
//! against Google's JWKS and hands us only the claims we trust.
use crate::error::AppError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct VerifiedGoogleUser {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub name: Option<String>,
}

#[async_trait]
pub trait GoogleVerifier: Send + Sync {
    /// Verify a raw Google identity token; failures mean 401.
    async fn verify(&self, id_token: &str) -> Result<VerifiedGoogleUser, AppError>;
}
