//! Credential and session use cases.
//!
//! Email OTP is the only first-party sign-in mechanism. Phone numbers are
//! optional contact data and never authentication credentials.

use crate::error::AppError;
use crate::ports::auth::{TokenKind, TokenPair, TokenService};
use crate::ports::email::EmailSender;
use crate::ports::otp::OtpCodeGenerator;
use crate::ports::repositories::{
    AuditActor, BranchRepository, EmailOtpRepository, OrganizationRepository,
    RefreshTokenRepository, UserRepository,
};
use crate::ports::tenancy::TenantResolver;
use pg_domain::{BranchId, OrgId, Organization, Role, UserId, UserStatus};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Normalizes the currently supported Indian phone-number identity format.
/// Keeping this policy at the auth boundary isolates it from generic domain
/// logic, so country-aware identity rules can evolve independently later.
pub fn normalize_phone(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    (digits.len() >= 10).then(|| digits[digits.len() - 10..].to_string())
}

pub fn normalize_email(raw: &str) -> Option<String> {
    let email = raw.trim().to_ascii_lowercase();
    let (local, domain) = email.split_once('@')?;
    (!local.is_empty() && domain.contains('.') && !email.contains(char::is_whitespace))
        .then_some(email)
}

pub struct RequestEmailOtpCommand<'a> {
    pub user_repo: &'a dyn UserRepository,
    pub otp_repo: &'a dyn EmailOtpRepository,
    pub codegen: &'a dyn OtpCodeGenerator,
    pub sender: &'a dyn EmailSender,
    pub platform_org_id: OrgId,
    pub resend_cooldown_secs: i64,
    pub max_sends: i64,
    pub window_secs: i64,
    pub otp_ttl_secs: i64,
    pub email: &'a str,
    pub full_name: Option<&'a str>,
    pub phone: Option<&'a str>,
}

pub async fn request_email_otp(command: RequestEmailOtpCommand<'_>) -> Result<(), AppError> {
    let email = normalize_email(command.email)
        .ok_or_else(|| AppError::Validation("invalid email".into()))?;
    if let Some(last_sent) = command.otp_repo.last_sent_at(&email).await? {
        let elapsed = (time::OffsetDateTime::now_utc() - last_sent).whole_seconds();
        if elapsed < command.resend_cooldown_secs {
            return Err(AppError::Validation(format!(
                "please wait {}s before requesting another code",
                command.resend_cooldown_secs - elapsed
            )));
        }
    }
    if command
        .otp_repo
        .sent_since(&email, command.window_secs)
        .await?
        >= command.max_sends
    {
        return Err(AppError::Validation(
            "too many code requests; try again later".into(),
        ));
    }
    let user = match command.user_repo.find_by_email(&email).await? {
        Some(user) if user.status == UserStatus::Active => {
            if command.full_name.is_some() || command.phone.is_some() {
                let _ = command
                    .user_repo
                    .update_profile(user.user_id, command.full_name, command.phone)
                    .await;
            }
            user
        }
        Some(_) => return Ok(()),
        None => {
            let record = command
                .user_repo
                .create_email_explorer(UserId::new(Uuid::new_v4()), command.platform_org_id, &email)
                .await?;
            if command.full_name.is_some() || command.phone.is_some() {
                let _ = command
                    .user_repo
                    .update_profile(record.user_id, command.full_name, command.phone)
                    .await;
            }
            record
        }
    };
    let code = command.codegen.generate();
    let challenge_id = Uuid::new_v4();
    command
        .otp_repo
        .create(
            challenge_id,
            user.user_id,
            &email,
            &code,
            command.otp_ttl_secs,
        )
        .await?;
    if let Err(error) = command
        .sender
        .send_login_otp(
            &email,
            user.full_name.as_deref(),
            &code,
            command.otp_ttl_secs,
        )
        .await
    {
        command.otp_repo.discard(challenge_id).await?;
        return Err(error);
    }
    Ok(())
}

pub struct VerifyEmailOtpCommand<'a> {
    pub user_repo: &'a dyn UserRepository,
    pub otp_repo: &'a dyn EmailOtpRepository,
    pub tokens: &'a dyn TokenService,
    pub refresh_repo: &'a dyn RefreshTokenRepository,
    pub refresh_ttl_secs: i64,
    pub otp_max_attempts: i32,
    pub email: &'a str,
    pub code: &'a str,
    pub full_name: Option<&'a str>,
    pub phone: Option<&'a str>,
}

pub async fn verify_email_otp(command: VerifyEmailOtpCommand<'_>) -> Result<TokenPair, AppError> {
    let email = normalize_email(command.email).ok_or(AppError::Unauthorized)?;
    let user = command
        .user_repo
        .find_by_email(&email)
        .await?
        .filter(|u| u.status == UserStatus::Active);
    let verification_user_id = user
        .as_ref()
        .map(|record| record.user_id)
        .unwrap_or_else(|| UserId::new(Uuid::nil()));
    let valid = command
        .otp_repo
        .verify_latest(
            &email,
            verification_user_id,
            command.code,
            command.otp_max_attempts,
        )
        .await?;
    if !valid {
        return Err(AppError::Unauthorized);
    }
    let user = user.ok_or(AppError::Unauthorized)?;
    if command.full_name.is_some() || command.phone.is_some() {
        let _ = command
            .user_repo
            .update_profile(user.user_id, command.full_name, command.phone)
            .await;
    }
    command.user_repo.mark_email_verified(user.user_id).await?;
    issue_and_store(
        command.tokens,
        command.refresh_repo,
        command.refresh_ttl_secs,
        user.user_id,
        user.org_id,
        user.role,
    )
    .await
}

async fn issue_and_store(
    tokens: &dyn TokenService,
    refresh_repo: &dyn RefreshTokenRepository,
    refresh_ttl_secs: i64,
    user_id: UserId,
    org_id: OrgId,
    role: Role,
) -> Result<TokenPair, AppError> {
    let pair = tokens.issue_pair(user_id, org_id, role)?;
    refresh_repo
        .store(
            &sha256_hex(&pair.refresh_token),
            user_id,
            org_id,
            refresh_ttl_secs,
        )
        .await?;
    Ok(pair)
}

pub struct ProvisionOrgInput {
    pub name: String,
    pub owner_email: String,
    pub owner_phone: Option<String>,
    pub owner_name: String,
}

pub struct ProvisionOrgOutput {
    pub org_id: OrgId,
    pub owner_id: UserId,
    pub branch_id: BranchId,
}

pub async fn provision_org(
    org_repo: &dyn OrganizationRepository,
    user_repo: &dyn UserRepository,
    branch_repo: &dyn BranchRepository,
    resolver: &dyn TenantResolver,
    input: ProvisionOrgInput,
) -> Result<ProvisionOrgOutput, AppError> {
    let email = normalize_email(&input.owner_email)
        .ok_or_else(|| AppError::Validation("valid owner_email required".into()))?;
    let phone = match input.owner_phone.as_deref() {
        Some(raw) => Some(normalize_phone(raw).ok_or_else(|| {
            AppError::Validation("owner_phone must contain at least 10 digits".into())
        })?),
        None => None,
    };
    let org_id = OrgId::new(Uuid::new_v4());
    org_repo
        .create(&Organization {
            id: org_id,
            name: input.name,
        })
        .await?;

    let owner_id = UserId::new(Uuid::new_v4());
    user_repo
        .create_active_owner(
            owner_id,
            org_id,
            &email,
            phone.as_deref(),
            &input.owner_name,
        )
        .await?;

    let ctx = resolver
        .resolve(org_id)
        .await
        .map_err(|error| AppError::Tenancy(error.to_string()))?;
    let branch_id = BranchId::new(Uuid::new_v4());
    branch_repo
        .create(
            &ctx,
            branch_id,
            "Main Branch",
            &AuditActor {
                user_id: Some(owner_id),
                request_id: None,
            },
        )
        .await?;

    Ok(ProvisionOrgOutput {
        org_id,
        owner_id,
        branch_id,
    })
}

pub async fn google_sign_in(
    verifier: &dyn crate::ports::GoogleVerifier,
    user_repo: &dyn UserRepository,
    tokens: &dyn TokenService,
    refresh_repo: &dyn RefreshTokenRepository,
    refresh_ttl_secs: i64,
    id_token: &str,
) -> Result<TokenPair, AppError> {
    let google_identity = verifier.verify(id_token).await?;
    let (user_id, org_id, role) = user_repo
        .find_by_google_sub(&google_identity.sub)
        .await?
        .ok_or(AppError::Unauthorized)?;
    issue_and_store(
        tokens,
        refresh_repo,
        refresh_ttl_secs,
        user_id,
        org_id,
        role,
    )
    .await
}

pub struct RefreshCommand<'a> {
    pub user_repo: &'a dyn UserRepository,
    pub tokens: &'a dyn TokenService,
    pub refresh_repo: &'a dyn RefreshTokenRepository,
    pub refresh_ttl_secs: i64,
    pub refresh_token: &'a str,
}
pub async fn refresh(command: RefreshCommand<'_>) -> Result<TokenPair, AppError> {
    let RefreshCommand {
        user_repo,
        tokens,
        refresh_repo,
        refresh_ttl_secs,
        refresh_token,
    } = command;
    let claims = tokens.verify(refresh_token)?;
    if claims.kind != TokenKind::Refresh {
        return Err(AppError::Unauthorized);
    }
    // A refresh token is bound to the organization and role selected when it
    // was issued. Never fall back to the user's highest-precedence role: that
    // would silently jump contexts after every background refresh.
    if !user_repo
        .has_active_membership(claims.user_id, claims.org_id, claims.role)
        .await?
    {
        return Err(AppError::Unauthorized);
    }
    if !refresh_repo.consume(&sha256_hex(refresh_token)).await? {
        return Err(AppError::Unauthorized);
    }
    issue_and_store(
        tokens,
        refresh_repo,
        refresh_ttl_secs,
        claims.user_id,
        claims.org_id,
        claims.role,
    )
    .await
}

pub struct SwitchOrganizationCommand<'a> {
    pub user_repo: &'a dyn UserRepository,
    pub tokens: &'a dyn TokenService,
    pub refresh_repo: &'a dyn RefreshTokenRepository,
    pub refresh_ttl_secs: i64,
    pub actor_user_id: UserId,
    pub target_org_id: OrgId,
    pub target_role: Role,
    pub refresh_token: &'a str,
}

/// Rotate the current refresh token into a newly selected active membership.
/// Requiring the refresh credential prevents a stolen access token alone from
/// minting a longer-lived session in another organization.
pub async fn switch_organization(
    command: SwitchOrganizationCommand<'_>,
) -> Result<TokenPair, AppError> {
    let claims = command.tokens.verify(command.refresh_token)?;
    if claims.kind != TokenKind::Refresh || claims.user_id != command.actor_user_id {
        return Err(AppError::Unauthorized);
    }
    if !command
        .user_repo
        .has_active_membership(
            command.actor_user_id,
            command.target_org_id,
            command.target_role,
        )
        .await?
    {
        return Err(AppError::Forbidden);
    }
    if !command
        .refresh_repo
        .consume(&sha256_hex(command.refresh_token))
        .await?
    {
        return Err(AppError::Unauthorized);
    }
    issue_and_store(
        command.tokens,
        command.refresh_repo,
        command.refresh_ttl_secs,
        command.actor_user_id,
        command.target_org_id,
        command.target_role,
    )
    .await
}

pub async fn logout(
    refresh_repo: &dyn RefreshTokenRepository,
    user_id: UserId,
) -> Result<(), AppError> {
    refresh_repo.revoke_all(user_id).await
}
