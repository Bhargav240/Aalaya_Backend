//! Google sign-in service: token verification is behind a port, so the
//! sub-bound happy path / unknown-user cases are testable without network.
//! Google sign-in only looks up an already-bound `sub`; it never matches
//! or binds by email.
#![allow(non_snake_case)]
mod fakes;

use async_trait::async_trait;
use fakes::*;
use pg_application::error::AppError;
use pg_application::ports::{GoogleVerifier, LoginRecord, VerifiedGoogleUser};
use pg_application::services::google_sign_in;
use pg_domain::{OrgId, Role, UserId, UserStatus};
use uuid::Uuid;

struct StubGoogle {
    outcome: VerifiedGoogleUser,
}

#[async_trait]
impl GoogleVerifier for StubGoogle {
    async fn verify(&self, _token: &str) -> Result<VerifiedGoogleUser, AppError> {
        Ok(self.outcome.clone())
    }
}

fn must_err<T>(r: Result<T, AppError>, what: &str) -> AppError {
    match r {
        Ok(_) => panic!("{what} unexpectedly succeeded"),
        Err(e) => e,
    }
}

#[tokio::test]
async fn google_sign_in__unverified_token__propagates_unauthorized() {
    struct Deny;
    #[async_trait]
    impl GoogleVerifier for Deny {
        async fn verify(&self, _t: &str) -> Result<VerifiedGoogleUser, AppError> {
            Err(AppError::Unauthorized)
        }
    }
    let err = must_err(
        google_sign_in(
            &Deny,
            &FakeUserRepo::new(),
            &FakeTokenService::new(),
            &FakeRefreshTokenRepo::new(),
            1209600,
            "garbage",
        )
        .await,
        "bad token",
    );
    assert!(matches!(err, AppError::Unauthorized));
}

#[tokio::test]
async fn google_sign_in__sub_bound__issues_pair() {
    let users = FakeUserRepo::new();
    let uid = UserId::new(Uuid::new_v4());
    let org = OrgId::new(Uuid::new_v4());
    users.bind_google("google|sub123", (uid, org, Role::Owner));

    let pair = google_sign_in(
        &StubGoogle {
            outcome: VerifiedGoogleUser {
                sub: "google|sub123".into(),
                email: Some("o@x.in".into()),
                email_verified: true,
                name: Some("Owner".into()),
            },
        },
        &users,
        &FakeTokenService::new(),
        &FakeRefreshTokenRepo::new(),
        1209600,
        "irrelevant",
    )
    .await
    .expect("sign in");
    assert!(!pair.access_token.is_empty());
    assert!(users.roles_of(uid).contains(&Role::Owner));
}

#[tokio::test]
async fn google_sign_in__unbound_sub__unauthorized() {
    // Only a pre-bound sub may sign in: an unbound sub is rejected, even
    // when the verified token carries a verified email for a known user.
    let users = FakeUserRepo::new();
    users.insert(
        "9876543210",
        LoginRecord {
            user_id: UserId::new(Uuid::new_v4()),
            org_id: OrgId::new(Uuid::new_v4()),
            role: Role::Tenant,
            status: UserStatus::Active,
            full_name: None,
        },
    );

    let err = must_err(
        google_sign_in(
            &StubGoogle {
                outcome: VerifiedGoogleUser {
                    sub: "google|subX".into(),
                    email: Some("tenant@x.in".into()),
                    email_verified: true,
                    name: None,
                },
            },
            &users,
            &FakeTokenService::new(),
            &FakeRefreshTokenRepo::new(),
            1209600,
            "t",
        )
        .await,
        "unbound sub",
    );
    assert!(matches!(err, AppError::Unauthorized));
}
