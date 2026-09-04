//! Authentication ports: token issuance/verification. Sign-in is phone+OTP
//! (or Google); there are no passwords anywhere in the system.
use crate::error::AppError;
use pg_domain::{OrgId, Role, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Access,
    Refresh,
}

#[derive(Debug, Clone)]
pub struct Claims {
    pub user_id: UserId,
    pub org_id: OrgId,
    pub role: Role,
    pub kind: TokenKind,
    /// JWT ID — unique per issued token. Used by the in-memory blocklist to
    /// invalidate *this specific access token* on explicit logout, closing the
    /// "stolen access token works until its TTL expires" window.
    pub jti: String,
    /// Absolute expiry (unix secs) — lets the blocklist GC entries for tokens
    /// that have already expired naturally.
    pub exp: i64,
}

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

/// Issues and verifies JWTs (jsonwebtoken adapter in infrastructure).
pub trait TokenService: Send + Sync {
    fn issue_pair(&self, user_id: UserId, org_id: OrgId, role: Role)
        -> Result<TokenPair, AppError>;
    fn issue_access(&self, user_id: UserId, org_id: OrgId, role: Role) -> Result<String, AppError>;
    fn verify(&self, token: &str) -> Result<Claims, AppError>;
}
