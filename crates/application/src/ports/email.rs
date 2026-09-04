use crate::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_login_otp(
        &self,
        email: &str,
        recipient_name: Option<&str>,
        code: &str,
        ttl_secs: i64,
    ) -> Result<(), AppError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitationEmailKind {
    Created,
    Resent,
    KycRequired,
    Approved,
    Rejected,
}

impl InvitationEmailKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Resent => "resent",
            Self::KycRequired => "kyc_required",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

pub struct InvitationEmail {
    pub invitation_id: Uuid,
    pub recipient: String,
    pub kind: InvitationEmailKind,
}

/// Dedicated delivery boundary for tenant-onboarding messages. Login OTP
/// delivery deliberately remains a separate concern.
#[async_trait]
pub trait InvitationEmailSender: Send + Sync {
    async fn send(&self, email: InvitationEmail) -> Result<(), AppError>;
}
