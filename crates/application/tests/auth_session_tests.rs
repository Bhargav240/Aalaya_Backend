//! Session: rotating refresh-token use/redemption, org provisioning, phone normalization.
#![allow(non_snake_case)]
mod fakes;

use fakes::*;
use pg_application::error::AppError;
use pg_application::ports::repositories::RefreshTokenRepository;
use pg_application::ports::TokenService;
use pg_application::services::{
    normalize_phone, provision_org, refresh, sha256_hex, switch_organization, ProvisionOrgInput,
    RefreshCommand, SwitchOrganizationCommand,
};
use pg_domain::{OrgId, Role, UserId};
use uuid::Uuid;

fn must<T>(r: Result<T, AppError>, what: &str) -> T {
    r.unwrap_or_else(|e| panic!("{what} failed: {e:?}"))
}
fn must_err<T>(r: Result<T, AppError>, what: &str) -> AppError {
    match r {
        Ok(_) => panic!("{what} unexpectedly succeeded"),
        Err(e) => e,
    }
}

// ---------- normalize_phone ----------

#[test]
fn normalize_phone__strips_to_last_10_digits() {
    assert_eq!(
        normalize_phone("+91 98765 43210"),
        Some("9876543210".into())
    );
    assert_eq!(normalize_phone("09876543210"), Some("9876543210".into()));
    assert_eq!(normalize_phone("9876543210"), Some("9876543210".into()));
}

#[test]
fn normalize_phone__too_few_digits__none() {
    assert_eq!(normalize_phone("98765 4321"), None);
    assert_eq!(normalize_phone(""), None);
}

// ---------- refresh (rotation) ----------

#[tokio::test]
async fn refresh__live_token__rotates_and_returns_fresh_pair() {
    let tokens = FakeTokenService::new();
    let repo = FakeRefreshTokenRepo::new();
    let users = FakeUserRepo::new();
    let owner = UserId::new(Uuid::new_v4());
    let org = OrgId::new(Uuid::new_v4());
    users
        .auth_contexts
        .lock()
        .expect("p")
        .insert(owner, (org, Role::Owner));

    // initial issuance (like a login)
    let pair0 = tokens.issue_pair(owner, org, Role::Owner).expect("issue");
    repo.store(&sha256_hex(&pair0.refresh_token), owner, org, 1209600)
        .await
        .expect("store");

    let pair1 = must(
        refresh(RefreshCommand {
            user_repo: &users,
            tokens: &tokens,
            refresh_repo: &repo,
            refresh_ttl_secs: 1209600,
            refresh_token: &pair0.refresh_token,
        })
        .await,
        "first refresh",
    );
    assert_ne!(pair1.refresh_token, pair0.refresh_token, "rotated");
    assert!(
        !repo.is_live(&sha256_hex(&pair0.refresh_token)),
        "old token revoked atomically"
    );
    assert!(repo.is_live(&sha256_hex(&pair1.refresh_token)));
}

#[tokio::test]
async fn refresh__reused_stale_token__denied() {
    let tokens = FakeTokenService::new();
    let repo = FakeRefreshTokenRepo::new();
    let users = FakeUserRepo::new();
    let first = FakeTokenService::new();
    let owner = UserId::new(Uuid::new_v4());
    let org = OrgId::new(Uuid::new_v4());
    users
        .auth_contexts
        .lock()
        .expect("p")
        .insert(owner, (org, Role::Owner));
    let pair0 = first.issue_pair(owner, org, Role::Owner).expect("issue");
    repo.store(&sha256_hex(&pair0.refresh_token), owner, org, 1209600)
        .await
        .expect("store");
    let _ = refresh(RefreshCommand {
        user_repo: &users,
        tokens: &tokens,
        refresh_repo: &repo,
        refresh_ttl_secs: 1209600,
        refresh_token: &pair0.refresh_token,
    })
    .await; // rotate once

    let err = must_err(
        refresh(RefreshCommand {
            user_repo: &users,
            tokens: &tokens,
            refresh_repo: &repo,
            refresh_ttl_secs: 1209600,
            refresh_token: &pair0.refresh_token,
        })
        .await,
        "replay",
    );
    assert!(
        matches!(err, AppError::Unauthorized),
        "reusing a rotated refresh token is a 401 (replay defence)"
    );
}

#[tokio::test]
async fn refresh__access_token_presented__denied() {
    let tokens = FakeTokenService::new();
    let repo = FakeRefreshTokenRepo::new();
    let users = FakeUserRepo::new();
    let access = tokens
        .issue_access(
            UserId::new(Uuid::new_v4()),
            OrgId::new(Uuid::new_v4()),
            Role::Owner,
        )
        .expect("access");
    let err = must_err(
        refresh(RefreshCommand {
            user_repo: &users,
            tokens: &tokens,
            refresh_repo: &repo,
            refresh_ttl_secs: 1209600,
            refresh_token: &access,
        })
        .await,
        "kind guard",
    );
    assert!(
        matches!(err, AppError::Unauthorized),
        "only Refresh kind accepted"
    );
}

#[tokio::test]
async fn refresh__unknown_token__denied() {
    let tokens = FakeTokenService::new();
    let err = must_err(
        refresh(RefreshCommand {
            user_repo: &FakeUserRepo::new(),
            tokens: &tokens,
            refresh_repo: &FakeRefreshTokenRepo::new(),
            refresh_ttl_secs: 1209600,
            refresh_token: "nonsense",
        })
        .await,
        "unknown token",
    );
    assert!(matches!(err, AppError::Unauthorized));
}

#[tokio::test]
async fn refresh__multi_membership__preserves_selected_context() {
    let tokens = FakeTokenService::new();
    let repo = FakeRefreshTokenRepo::new();
    let users = FakeUserRepo::new();
    let user = UserId::new(Uuid::new_v4());
    let platform = OrgId::new(Uuid::new_v4());
    let property = OrgId::new(Uuid::new_v4());
    users
        .auth_contexts
        .lock()
        .expect("p")
        .insert(user, (platform, Role::Prospect));
    users
        .additional_auth_contexts
        .lock()
        .expect("p")
        .insert(user, vec![(property, Role::Tenant)]);
    let initial = tokens
        .issue_pair(user, platform, Role::Prospect)
        .expect("issue");
    repo.store(&sha256_hex(&initial.refresh_token), user, platform, 1209600)
        .await
        .expect("store");

    let rotated = refresh(RefreshCommand {
        user_repo: &users,
        tokens: &tokens,
        refresh_repo: &repo,
        refresh_ttl_secs: 1209600,
        refresh_token: &initial.refresh_token,
    })
    .await
    .expect("refresh");
    let claims = tokens.verify(&rotated.access_token).expect("claims");
    assert_eq!((claims.org_id, claims.role), (platform, Role::Prospect));
}

#[tokio::test]
async fn switch_organization__active_membership__rotates_into_target_context() {
    let tokens = FakeTokenService::new();
    let repo = FakeRefreshTokenRepo::new();
    let users = FakeUserRepo::new();
    let user = UserId::new(Uuid::new_v4());
    let platform = OrgId::new(Uuid::new_v4());
    let property = OrgId::new(Uuid::new_v4());
    users
        .auth_contexts
        .lock()
        .expect("p")
        .insert(user, (platform, Role::Prospect));
    users
        .additional_auth_contexts
        .lock()
        .expect("p")
        .insert(user, vec![(property, Role::Tenant)]);
    let initial = tokens
        .issue_pair(user, platform, Role::Prospect)
        .expect("issue");
    repo.store(&sha256_hex(&initial.refresh_token), user, platform, 1209600)
        .await
        .expect("store");

    let switched = switch_organization(SwitchOrganizationCommand {
        user_repo: &users,
        tokens: &tokens,
        refresh_repo: &repo,
        refresh_ttl_secs: 1209600,
        actor_user_id: user,
        target_org_id: property,
        target_role: Role::Tenant,
        refresh_token: &initial.refresh_token,
    })
    .await
    .expect("switch");
    let claims = tokens.verify(&switched.access_token).expect("claims");
    assert_eq!((claims.org_id, claims.role), (property, Role::Tenant));
    assert!(!repo.is_live(&sha256_hex(&initial.refresh_token)));
}

#[tokio::test]
async fn switch_organization__inactive_target__is_forbidden_without_consuming_token() {
    let tokens = FakeTokenService::new();
    let repo = FakeRefreshTokenRepo::new();
    let users = FakeUserRepo::new();
    let user = UserId::new(Uuid::new_v4());
    let platform = OrgId::new(Uuid::new_v4());
    let target = OrgId::new(Uuid::new_v4());
    users
        .auth_contexts
        .lock()
        .expect("p")
        .insert(user, (platform, Role::Prospect));
    let initial = tokens
        .issue_pair(user, platform, Role::Prospect)
        .expect("issue");
    repo.store(&sha256_hex(&initial.refresh_token), user, platform, 1209600)
        .await
        .expect("store");

    let error = switch_organization(SwitchOrganizationCommand {
        user_repo: &users,
        tokens: &tokens,
        refresh_repo: &repo,
        refresh_ttl_secs: 1209600,
        actor_user_id: user,
        target_org_id: target,
        target_role: Role::Tenant,
        refresh_token: &initial.refresh_token,
    })
    .await
    .expect_err("inactive context must fail");
    assert!(matches!(error, AppError::Forbidden));
    assert!(repo.is_live(&sha256_hex(&initial.refresh_token)));
}

// ---------- provision_org ----------

#[tokio::test]
async fn provision_org__invalid_owner_email__rejected() {
    let input = ProvisionOrgInput {
        name: "Hamsa Vilas".into(),
        owner_email: "not-an-email".into(),
        owner_phone: None,
        owner_name: "Owner".into(),
    };
    let err = must_err(
        provision_org(
            &OrgRepoOk,
            &FakeUserRepo::new(),
            &FakeBranchRepo::with_count(0),
            &StubResolver::pooled(),
            input,
        )
        .await,
        "invalid owner email",
    );
    assert!(matches!(err, AppError::Validation(_)));
}

#[tokio::test]
async fn provision_org__happy_path__org_and_owner_created() {
    let users = FakeUserRepo::new();
    let out = must(
        provision_org(
            &OrgRepoOk,
            &users,
            &FakeBranchRepo::with_count(0),
            &StubResolver::pooled(),
            ProvisionOrgInput {
                name: "Hamsa Vilas".into(),
                owner_email: "owner@hamsavilas.test".into(),
                owner_phone: Some("9876543210".into()),
                owner_name: "Hamsa Owner".into(),
            },
        )
        .await,
        "provision",
    );
    assert_ne!(out.org_id.as_uuid(), Uuid::nil());
    assert_ne!(out.owner_id.as_uuid(), Uuid::nil());
    // Owner signs in via email OTP — provisioning returns no tokens.
}
