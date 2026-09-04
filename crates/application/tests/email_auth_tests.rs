mod fakes;

use async_trait::async_trait;
use fakes::{FakeRefreshTokenRepo, FakeTokenService, FakeUserRepo};
use pg_application::ports::{EmailOtpRepository, EmailSender, LoginRecord, OtpCodeGenerator};
use pg_application::services::{
    normalize_email, request_email_otp, verify_email_otp, RequestEmailOtpCommand,
    VerifyEmailOtpCommand,
};
use pg_application::AppError;
use pg_domain::{OrgId, Role, UserId, UserStatus};
use std::sync::Mutex;
use time::OffsetDateTime;
use uuid::Uuid;

struct FixedCode;
impl OtpCodeGenerator for FixedCode {
    fn generate(&self) -> String {
        "123456".into()
    }
}

#[derive(Default)]
struct FakeEmailOtpRepo {
    created: Mutex<Vec<Uuid>>,
    discarded: Mutex<Vec<Uuid>>,
    verify_result: Mutex<bool>,
    verify_users: Mutex<Vec<UserId>>,
}

#[async_trait]
impl EmailOtpRepository for FakeEmailOtpRepo {
    async fn create(
        &self,
        id: Uuid,
        _user_id: UserId,
        _email: &str,
        _code: &str,
        _ttl_secs: i64,
    ) -> Result<(), AppError> {
        self.created.lock().expect("created").push(id);
        Ok(())
    }

    async fn verify_latest(
        &self,
        _email: &str,
        user_id: UserId,
        _code: &str,
        _max_attempts: i32,
    ) -> Result<bool, AppError> {
        self.verify_users
            .lock()
            .expect("verify users")
            .push(user_id);
        Ok(*self.verify_result.lock().expect("verify result"))
    }

    async fn discard(&self, id: Uuid) -> Result<(), AppError> {
        self.discarded.lock().expect("discarded").push(id);
        Ok(())
    }

    async fn last_sent_at(&self, _email: &str) -> Result<Option<OffsetDateTime>, AppError> {
        Ok(None)
    }

    async fn sent_since(&self, _email: &str, _window_secs: i64) -> Result<i64, AppError> {
        Ok(0)
    }
}

struct FailingEmailSender;
#[async_trait]
impl EmailSender for FailingEmailSender {
    async fn send_login_otp(
        &self,
        _email: &str,
        _recipient_name: Option<&str>,
        _code: &str,
        _ttl_secs: i64,
    ) -> Result<(), AppError> {
        Err(AppError::Internal("delivery failed".into()))
    }
}

#[test]
fn email_normalization_is_case_insensitive_and_rejects_invalid_values() {
    assert_eq!(
        normalize_email("  Person@Example.COM "),
        Some("person@example.com".into())
    );
    assert_eq!(normalize_email("missing-domain@example"), None);
    assert_eq!(normalize_email("has space@example.com"), None);
}

#[tokio::test]
async fn failed_delivery_discards_the_new_challenge() {
    let users = FakeUserRepo::new();
    let otps = FakeEmailOtpRepo::default();
    let result = request_email_otp(RequestEmailOtpCommand {
        user_repo: &users,
        otp_repo: &otps,
        codegen: &FixedCode,
        sender: &FailingEmailSender,
        platform_org_id: OrgId::new(Uuid::new_v4()),
        resend_cooldown_secs: 30,
        max_sends: 3,
        window_secs: 900,
        otp_ttl_secs: 600,
        email: "new@example.com",
    })
    .await;

    assert!(matches!(result, Err(AppError::Internal(_))));
    assert_eq!(
        *otps.created.lock().expect("created"),
        *otps.discarded.lock().expect("discarded")
    );
}

#[tokio::test]
async fn nonexistent_and_wrong_codes_share_the_unauthorized_result() {
    let users = FakeUserRepo::new();
    let org = OrgId::new(Uuid::new_v4());
    let existing = UserId::new(Uuid::new_v4());
    users.insert(
        "existing@example.com",
        LoginRecord {
            user_id: existing,
            org_id: org,
            role: Role::Prospect,
            status: UserStatus::Active,
            full_name: None,
        },
    );
    let otps = FakeEmailOtpRepo::default();
    let tokens = FakeTokenService::new();
    let refresh = FakeRefreshTokenRepo::new();

    for email in ["existing@example.com", "missing@example.com"] {
        let result = verify_email_otp(VerifyEmailOtpCommand {
            user_repo: &users,
            otp_repo: &otps,
            tokens: &tokens,
            refresh_repo: &refresh,
            refresh_ttl_secs: 1209600,
            otp_max_attempts: 5,
            email,
            code: "000000",
        })
        .await;
        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    let verified_users = otps.verify_users.lock().expect("verify users");
    assert_eq!(verified_users[0], existing);
    assert_eq!(verified_users[1].as_uuid(), Uuid::nil());
}

#[tokio::test]
async fn valid_email_code_issues_and_persists_a_rotating_session() {
    let users = FakeUserRepo::new();
    let user = UserId::new(Uuid::new_v4());
    let org = OrgId::new(Uuid::new_v4());
    users.insert(
        "person@example.com",
        LoginRecord {
            user_id: user,
            org_id: org,
            role: Role::Prospect,
            status: UserStatus::Active,
            full_name: None,
        },
    );
    let otps = FakeEmailOtpRepo::default();
    *otps.verify_result.lock().expect("verify result") = true;
    let tokens = FakeTokenService::new();
    let refresh = FakeRefreshTokenRepo::new();

    let pair = verify_email_otp(VerifyEmailOtpCommand {
        user_repo: &users,
        otp_repo: &otps,
        tokens: &tokens,
        refresh_repo: &refresh,
        refresh_ttl_secs: 1209600,
        otp_max_attempts: 5,
        email: "person@example.com",
        code: "123456",
    })
    .await
    .expect("valid code");

    let hash = pg_application::services::sha256_hex(&pair.refresh_token);
    assert!(refresh.is_live(&hash));
}
