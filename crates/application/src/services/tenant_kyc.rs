use crate::error::AppError;
use crate::ports::repositories::{TenantInvitationRepository, UserRepository};
use crate::ports::{
    DigiLockerClient, InvitationVerification, InvitationVerificationRepository,
    NewDigiLockerSession,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use pg_domain::{InvitationStatus, OrgId, Role, UserId};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub struct StartDigiLockerCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub verifications: &'a dyn InvitationVerificationRepository,
    pub client: &'a dyn DigiLockerClient,
    pub encryption_key: &'a str,
    pub session_ttl_secs: i64,
    pub actor_id: UserId,
    pub actor_role: Role,
    pub invitation_id: Uuid,
}

pub async fn start_digilocker(command: StartDigiLockerCommand<'_>) -> Result<String, AppError> {
    if command.actor_role == Role::PlatformOperator {
        return Err(AppError::Forbidden);
    }
    let invitation = command
        .invitations
        .get(command.invitation_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let verified_email = command
        .users
        .verified_email_of(command.actor_id)
        .await?
        .ok_or(AppError::Forbidden)?;
    if !verified_email.eq_ignore_ascii_case(&invitation.email) {
        return Err(AppError::Forbidden);
    }
    if !invitation.require_identity_verification
        || invitation.status != InvitationStatus::KycPending
    {
        return Err(AppError::Conflict(
            "invitation is not awaiting DigiLocker verification".into(),
        ));
    }

    let state = random_secret();
    let nonce = random_secret();
    let pkce_verifier = random_secret();
    let code_challenge = hash_base64(&pkce_verifier);
    command
        .verifications
        .create_digilocker_session(NewDigiLockerSession {
            invitation_id: invitation.id,
            user_id: command.actor_id,
            state_hash: &hash_hex(&state),
            nonce_hash: &hash_hex(&nonce),
            pkce_verifier: &pkce_verifier,
            encryption_key: command.encryption_key,
            expires_at: OffsetDateTime::now_utc()
                + Duration::seconds(command.session_ttl_secs.clamp(60, 900)),
        })
        .await?;
    command
        .client
        .authorization_url(&state, &nonce, &code_challenge)
}

pub struct CompleteDigiLockerCommand<'a> {
    pub verifications: &'a dyn InvitationVerificationRepository,
    pub client: &'a dyn DigiLockerClient,
    pub encryption_key: &'a str,
    pub state: &'a str,
    pub code: Option<&'a str>,
    pub provider_error: Option<&'a str>,
}

pub struct DigiLockerCallbackResult {
    pub invitation_id: Uuid,
    pub status: InvitationStatus,
}

pub async fn complete_digilocker(
    command: CompleteDigiLockerCommand<'_>,
) -> Result<DigiLockerCallbackResult, AppError> {
    let session = command
        .verifications
        .consume_digilocker_session(&hash_hex(command.state), command.encryption_key)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if command.provider_error.is_some() || command.code.is_none() {
        command
            .verifications
            .complete_digilocker(
                session.invitation_id,
                None,
                None,
                false,
                Some("identity provider authorization was not completed"),
            )
            .await?;
        return Ok(DigiLockerCallbackResult {
            invitation_id: session.invitation_id,
            status: InvitationStatus::KycFailed,
        });
    }

    let identity = match command
        .client
        .exchange_code(
            command.code.ok_or(AppError::Unauthorized)?,
            &session.pkce_verifier,
        )
        .await
    {
        Ok(identity) => identity,
        Err(_) => {
            command
                .verifications
                .complete_digilocker(
                    session.invitation_id,
                    None,
                    None,
                    false,
                    Some("identity provider verification failed"),
                )
                .await?;
            return Ok(DigiLockerCallbackResult {
                invitation_id: session.invitation_id,
                status: InvitationStatus::KycFailed,
            });
        }
    };
    if !constant_time_equal(&hash_hex(&identity.nonce), &session.nonce_hash) {
        command
            .verifications
            .complete_digilocker(
                session.invitation_id,
                None,
                None,
                false,
                Some("identity provider response could not be validated"),
            )
            .await?;
        return Ok(DigiLockerCallbackResult {
            invitation_id: session.invitation_id,
            status: InvitationStatus::KycFailed,
        });
    }

    command
        .verifications
        .complete_digilocker(
            session.invitation_id,
            Some(&identity.provider_reference),
            identity.document_type.as_deref(),
            true,
            None,
        )
        .await?;
    Ok(DigiLockerCallbackResult {
        invitation_id: session.invitation_id,
        status: InvitationStatus::ApprovalPending,
    })
}

pub async fn get_invitation_verification(
    invitations: &dyn TenantInvitationRepository,
    users: &dyn UserRepository,
    verifications: &dyn InvitationVerificationRepository,
    actor_id: UserId,
    actor_org_id: OrgId,
    actor_role: Role,
    invitation_id: Uuid,
) -> Result<Option<InvitationVerification>, AppError> {
    let invitation = invitations
        .get(invitation_id)
        .await?
        .ok_or(AppError::NotFound)?;
    authorize_view(users, actor_id, actor_org_id, actor_role, &invitation).await?;
    verifications.latest(invitation_id).await
}

pub struct ManualKycReviewCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub verifications: &'a dyn InvitationVerificationRepository,
    pub actor_id: UserId,
    pub actor_org_id: OrgId,
    pub actor_role: Role,
    pub invitation_id: Uuid,
    pub approved: bool,
    pub reason: String,
    pub request_id: Option<Uuid>,
    pub manual_review_allowed: bool,
}

pub async fn manual_kyc_review(command: ManualKycReviewCommand<'_>) -> Result<(), AppError> {
    if !command.manual_review_allowed {
        return Err(AppError::Forbidden);
    }
    let invitation = command
        .invitations
        .get(command.invitation_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if invitation.org_id != command.actor_org_id {
        return Err(AppError::Forbidden);
    }
    match command.actor_role {
        Role::Owner => {}
        Role::Manager
            if command
                .users
                .has_branch_access(
                    command.actor_id,
                    command.actor_org_id,
                    invitation.branch_id,
                )
                .await? => {}
        _ => return Err(AppError::Forbidden),
    }
    let reason = command.reason.trim();
    if reason.len() < 3 || reason.len() > 500 {
        return Err(AppError::Validation(
            "manual review reason must be between 3 and 500 characters".into(),
        ));
    }
    command
        .verifications
        .manual_review(
            invitation.id,
            command.actor_id,
            command.approved,
            reason,
            command.request_id,
        )
        .await
}

async fn authorize_view(
    users: &dyn UserRepository,
    actor_id: UserId,
    actor_org_id: OrgId,
    actor_role: Role,
    invitation: &crate::ports::repositories::TenantInvitation,
) -> Result<(), AppError> {
    match actor_role {
        Role::Owner if actor_org_id == invitation.org_id => Ok(()),
        Role::Manager
            if actor_org_id == invitation.org_id
                && users
                    .has_branch_access(actor_id, actor_org_id, invitation.branch_id)
                    .await? =>
        {
            Ok(())
        }
        Role::PlatformOperator => Err(AppError::Forbidden),
        _ => {
            let email = users
                .verified_email_of(actor_id)
                .await?
                .ok_or(AppError::Forbidden)?;
            if email.eq_ignore_ascii_case(&invitation.email) {
                Ok(())
            } else {
                Err(AppError::Forbidden)
            }
        }
    }
}

fn random_secret() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hash_base64(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(value.as_bytes()))
}

fn hash_hex(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn constant_time_equal(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{
        DigiLockerIdentity, DigiLockerSession, InvitationVerification, NewDigiLockerSession,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct TestClient {
        identity: Mutex<Option<Result<DigiLockerIdentity, AppError>>>,
    }

    impl TestClient {
        fn identity(nonce: &str) -> Self {
            Self {
                identity: Mutex::new(Some(Ok(DigiLockerIdentity {
                    provider_reference: "provider-ref".into(),
                    document_type: Some("aadhaar".into()),
                    nonce: nonce.into(),
                }))),
            }
        }

        fn failure() -> Self {
            Self {
                identity: Mutex::new(Some(Err(AppError::Unauthorized))),
            }
        }
    }

    #[async_trait]
    impl DigiLockerClient for TestClient {
        fn authorization_url(
            &self,
            _state: &str,
            _nonce: &str,
            _code_challenge: &str,
        ) -> Result<String, AppError> {
            Ok("https://example.test/authorize".into())
        }

        async fn exchange_code(
            &self,
            _code: &str,
            _pkce_verifier: &str,
        ) -> Result<DigiLockerIdentity, AppError> {
            self.identity
                .lock()
                .expect("identity")
                .take()
                .unwrap_or(Err(AppError::Unauthorized))
        }
    }

    struct TestVerificationRepo {
        expected_state_hash: String,
        session: Mutex<Option<DigiLockerSession>>,
        completed: Mutex<Vec<(bool, Option<String>)>>,
    }

    impl TestVerificationRepo {
        fn new(state: &str, nonce: &str) -> Self {
            Self {
                expected_state_hash: hash_hex(state),
                session: Mutex::new(Some(DigiLockerSession {
                    invitation_id: Uuid::new_v4(),
                    user_id: UserId::new(Uuid::new_v4()),
                    nonce_hash: hash_hex(nonce),
                    pkce_verifier: "pkce".into(),
                })),
                completed: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl InvitationVerificationRepository for TestVerificationRepo {
        async fn create_digilocker_session(
            &self,
            _session: NewDigiLockerSession<'_>,
        ) -> Result<(), AppError> {
            Ok(())
        }

        async fn consume_digilocker_session(
            &self,
            state_hash: &str,
            _encryption_key: &str,
        ) -> Result<Option<DigiLockerSession>, AppError> {
            if state_hash != self.expected_state_hash {
                return Ok(None);
            }
            Ok(self.session.lock().expect("session").take())
        }

        async fn complete_digilocker(
            &self,
            _invitation_id: Uuid,
            _provider_reference: Option<&str>,
            _document_type: Option<&str>,
            verified: bool,
            safe_failure_reason: Option<&str>,
        ) -> Result<(), AppError> {
            self.completed
                .lock()
                .expect("completed")
                .push((verified, safe_failure_reason.map(str::to_string)));
            Ok(())
        }

        async fn latest(
            &self,
            _invitation_id: Uuid,
        ) -> Result<Option<InvitationVerification>, AppError> {
            Ok(None)
        }

        async fn manual_review(
            &self,
            _invitation_id: Uuid,
            _reviewer_user_id: UserId,
            _approved: bool,
            _reason: &str,
            _request_id: Option<Uuid>,
        ) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[test]
    fn generated_oauth_secrets_are_url_safe_and_unique() {
        let first = random_secret();
        let second = random_secret();
        assert_ne!(first, second);
        assert!(first
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')));
    }

    #[test]
    fn constant_time_comparison_rejects_different_values() {
        assert!(constant_time_equal("same", "same"));
        assert!(!constant_time_equal("same", "diff"));
        assert!(!constant_time_equal("short", "longer"));
    }

    #[tokio::test]
    async fn wrong_or_expired_state_is_rejected_without_provider_exchange() {
        let repo = TestVerificationRepo::new("expected", "nonce");
        let result = complete_digilocker(CompleteDigiLockerCommand {
            verifications: &repo,
            client: &TestClient::identity("nonce"),
            encryption_key: "key",
            state: "wrong-or-expired",
            code: Some("code"),
            provider_error: None,
        })
        .await;
        assert!(matches!(result, Err(AppError::Unauthorized)));
        assert!(repo.completed.lock().expect("completed").is_empty());
    }

    #[tokio::test]
    async fn wrong_nonce_marks_safe_failure() {
        let repo = TestVerificationRepo::new("state", "expected-nonce");
        let result = complete_digilocker(CompleteDigiLockerCommand {
            verifications: &repo,
            client: &TestClient::identity("tampered-nonce"),
            encryption_key: "key",
            state: "state",
            code: Some("code"),
            provider_error: None,
        })
        .await
        .expect("safe failure");
        assert_eq!(result.status, InvitationStatus::KycFailed);
        assert!(!repo.completed.lock().expect("completed")[0].0);
    }

    #[tokio::test]
    async fn provider_failure_is_safe_and_callback_cannot_be_replayed() {
        let repo = TestVerificationRepo::new("state", "nonce");
        let result = complete_digilocker(CompleteDigiLockerCommand {
            verifications: &repo,
            client: &TestClient::failure(),
            encryption_key: "key",
            state: "state",
            code: Some("code"),
            provider_error: None,
        })
        .await
        .expect("safe provider failure");
        assert_eq!(result.status, InvitationStatus::KycFailed);

        let replay = complete_digilocker(CompleteDigiLockerCommand {
            verifications: &repo,
            client: &TestClient::identity("nonce"),
            encryption_key: "key",
            state: "state",
            code: Some("code"),
            provider_error: None,
        })
        .await;
        assert!(matches!(replay, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn provider_authorization_error_does_not_exchange_a_code() {
        let repo = TestVerificationRepo::new("state", "nonce");
        let result = complete_digilocker(CompleteDigiLockerCommand {
            verifications: &repo,
            client: &TestClient::identity("nonce"),
            encryption_key: "key",
            state: "state",
            code: None,
            provider_error: Some("access_denied"),
        })
        .await
        .expect("safe denial");
        assert_eq!(result.status, InvitationStatus::KycFailed);
    }
}
