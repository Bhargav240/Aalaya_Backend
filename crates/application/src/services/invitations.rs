//! Tenant invitation use cases.
//!
//! HTTP handlers only translate requests and responses. Authorization,
//! validation, visibility, and lifecycle rules live here so every transport
//! observes the same behaviour.
use super::auth::normalize_email;
use crate::error::AppError;
use crate::ports::email::{InvitationEmail, InvitationEmailKind, InvitationEmailSender};
use crate::ports::repositories::{
    ActivateTenantRecord, TenantActivationRepository, TenantActivationResult, TenantInvitation,
    TenantInvitationRepository, UserRepository,
};
use crate::ports::tenancy::TenantContext;
use pg_domain::{BedId, BranchId, InvitationStatus, OrgId, Role, RoomId, UserId};
use time::{Date, Duration, OffsetDateTime};
use uuid::Uuid;

pub struct ListTenantInvitationsCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub actor_id: UserId,
    pub actor_org_id: OrgId,
    pub actor_role: Role,
}

pub async fn list_tenant_invitations(
    command: ListTenantInvitationsCommand<'_>,
) -> Result<Vec<TenantInvitation>, AppError> {
    if !matches!(command.actor_role, Role::Owner | Role::Manager) {
        return Err(AppError::Forbidden);
    }

    let assigned_branch = if command.actor_role == Role::Manager {
        command
            .users
            .branch_in_org(command.actor_id, command.actor_org_id)
            .await?
    } else {
        None
    };

    Ok(command
        .invitations
        .list_for_org(command.actor_org_id)
        .await?
        .into_iter()
        .filter(|invitation| {
            command.actor_role == Role::Owner || assigned_branch == Some(invitation.branch_id)
        })
        .collect())
}

pub struct CreateTenantInvitationCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub email_sender: &'a dyn InvitationEmailSender,
    pub actor_id: UserId,
    pub actor_org_id: OrgId,
    pub actor_role: Role,
    pub email: String,
    pub full_name: String,
    pub plan_kind: String,
    pub branch_id: BranchId,
    pub intended_room_id: Option<RoomId>,
    pub intended_bed_id: Option<BedId>,
    pub monthly_rent_paise: Option<i64>,
    pub deposit_paise: Option<i64>,
    pub planned_move_in_date: Option<Date>,
    pub require_identity_verification: bool,
    pub expires_in_days: Option<i64>,
}

pub async fn create_tenant_invitation(
    command: CreateTenantInvitationCommand<'_>,
) -> Result<TenantInvitation, AppError> {
    require_staff_scope(
        command.users,
        command.actor_id,
        command.actor_org_id,
        command.actor_role,
        command.branch_id,
    )
    .await?;

    let email = normalize_email(&command.email)
        .ok_or_else(|| AppError::Validation("invalid email".into()))?;
    let full_name = command.full_name.trim().to_string();
    if full_name.is_empty() {
        return Err(AppError::Validation("full_name required".into()));
    }
    if command.intended_room_id.is_none() {
        return Err(AppError::Validation("intended_room_id required".into()));
    }
    if !matches!(command.plan_kind.as_str(), "daily" | "weekly" | "monthly") {
        return Err(AppError::Validation(
            "plan_kind must be daily, weekly, or monthly".into(),
        ));
    }
    if command.monthly_rent_paise.is_some_and(|amount| amount < 0) {
        return Err(AppError::Validation(
            "monthly_rent_paise cannot be negative".into(),
        ));
    }
    if command.deposit_paise.is_some_and(|amount| amount < 0) {
        return Err(AppError::Validation(
            "deposit_paise cannot be negative".into(),
        ));
    }

    let now = OffsetDateTime::now_utc();
    let invitation = TenantInvitation {
        id: Uuid::new_v4(),
        org_id: command.actor_org_id,
        branch_id: command.branch_id,
        intended_room_id: command.intended_room_id,
        intended_bed_id: command.intended_bed_id,
        email,
        full_name,
        plan_kind: command.plan_kind,
        monthly_rent_paise: command.monthly_rent_paise,
        deposit_paise: command.deposit_paise,
        planned_move_in_date: command.planned_move_in_date,
        require_identity_verification: command.require_identity_verification,
        status: InvitationStatus::Invited,
        accepted_by_user_id: None,
        expires_at: now + Duration::days(command.expires_in_days.unwrap_or(7).clamp(1, 30)),
        last_sent_at: now,
        accepted_at: None,
        approved_at: None,
        approved_by_user_id: None,
    };
    command
        .invitations
        .create(&invitation, command.actor_id)
        .await?;
    command
        .email_sender
        .send(InvitationEmail {
            invitation_id: invitation.id,
            recipient: invitation.email.clone(),
            kind: InvitationEmailKind::Created,
        })
        .await?;
    Ok(invitation)
}

pub struct GetTenantInvitationCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub actor_id: UserId,
    pub actor_org_id: OrgId,
    pub actor_role: Role,
    pub invitation_id: Uuid,
}

pub async fn get_tenant_invitation(
    command: GetTenantInvitationCommand<'_>,
) -> Result<TenantInvitation, AppError> {
    if command.actor_role == Role::PlatformOperator {
        return Err(AppError::Forbidden);
    }
    let invitation = command
        .invitations
        .get(command.invitation_id)
        .await?
        .ok_or(AppError::NotFound)?;
    authorize_invitation_view(
        command.users,
        command.actor_id,
        command.actor_org_id,
        command.actor_role,
        &invitation,
    )
    .await?;
    Ok(invitation)
}

pub struct AcceptTenantInvitationCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub email_sender: &'a dyn InvitationEmailSender,
    pub actor_id: UserId,
    pub actor_role: Role,
    pub invitation_id: Uuid,
}

pub async fn accept_tenant_invitation(
    command: AcceptTenantInvitationCommand<'_>,
) -> Result<InvitationStatus, AppError> {
    if command.actor_role == Role::PlatformOperator {
        return Err(AppError::Forbidden);
    }
    let email = verified_email(command.users, command.actor_id).await?;
    let invitation =
        invitation_for_email(command.invitations, command.invitation_id, &email).await?;
    ensure_not_expired(&invitation)?;
    ensure_transition(invitation.status, InvitationStatus::EmailVerified)?;

    if !command
        .invitations
        .accept(command.invitation_id, command.actor_id, &email)
        .await?
    {
        return Err(AppError::Conflict(
            "invitation changed while it was being accepted".into(),
        ));
    }
    let target = if invitation.require_identity_verification {
        InvitationStatus::KycPending
    } else {
        InvitationStatus::ApprovalPending
    };
    if !command
        .invitations
        .transition(
            command.invitation_id,
            InvitationStatus::EmailVerified,
            target,
            command.actor_id,
        )
        .await?
    {
        return Err(AppError::Conflict(
            "invitation changed while onboarding was being started".into(),
        ));
    }
    if target == InvitationStatus::KycPending {
        command
            .email_sender
            .send(InvitationEmail {
                invitation_id: invitation.id,
                recipient: invitation.email,
                kind: InvitationEmailKind::KycRequired,
            })
            .await?;
    }
    Ok(target)
}

pub struct DeclineTenantInvitationCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub actor_id: UserId,
    pub actor_role: Role,
    pub invitation_id: Uuid,
}

pub async fn decline_tenant_invitation(
    command: DeclineTenantInvitationCommand<'_>,
) -> Result<InvitationStatus, AppError> {
    if command.actor_role == Role::PlatformOperator {
        return Err(AppError::Forbidden);
    }
    let email = verified_email(command.users, command.actor_id).await?;
    let invitation =
        invitation_for_email(command.invitations, command.invitation_id, &email).await?;
    ensure_transition(invitation.status, InvitationStatus::Cancelled)?;

    if !command
        .invitations
        .decline(command.invitation_id, &email, invitation.status)
        .await?
    {
        return Err(AppError::Conflict(
            "invitation changed while it was being declined".into(),
        ));
    }
    Ok(InvitationStatus::Cancelled)
}

pub struct ReviewTenantInvitationCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub email_sender: &'a dyn InvitationEmailSender,
    pub actor_id: UserId,
    pub actor_org_id: OrgId,
    pub actor_role: Role,
    pub invitation_id: Uuid,
    pub target: InvitationStatus,
}

pub async fn review_tenant_invitation(
    command: ReviewTenantInvitationCommand<'_>,
) -> Result<InvitationStatus, AppError> {
    if !matches!(
        command.target,
        InvitationStatus::Rejected | InvitationStatus::Cancelled
    ) {
        return Err(AppError::Validation(
            "unsupported staff invitation action".into(),
        ));
    }

    let invitation = command
        .invitations
        .get(command.invitation_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if invitation.org_id != command.actor_org_id {
        return Err(AppError::Forbidden);
    }
    require_staff_scope(
        command.users,
        command.actor_id,
        command.actor_org_id,
        command.actor_role,
        invitation.branch_id,
    )
    .await?;
    ensure_transition(invitation.status, command.target)?;
    if command.target == InvitationStatus::Approved
        && invitation.accepted_by_user_id == Some(command.actor_id)
    {
        return Err(AppError::Forbidden);
    }

    if !command
        .invitations
        .transition(
            command.invitation_id,
            invitation.status,
            command.target,
            command.actor_id,
        )
        .await?
    {
        return Err(AppError::Conflict(
            "invitation changed while it was being reviewed".into(),
        ));
    }
    let notification = match command.target {
        InvitationStatus::Approved => Some(InvitationEmailKind::Approved),
        InvitationStatus::Rejected => Some(InvitationEmailKind::Rejected),
        _ => None,
    };
    if let Some(kind) = notification {
        command
            .email_sender
            .send(InvitationEmail {
                invitation_id: invitation.id,
                recipient: invitation.email,
                kind,
            })
            .await?;
    }
    Ok(command.target)
}

pub struct ApproveTenantInvitationCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub activation: &'a dyn TenantActivationRepository,
    pub users: &'a dyn UserRepository,
    pub email_sender: &'a dyn InvitationEmailSender,
    pub ctx: &'a TenantContext,
    pub actor_id: UserId,
    pub actor_org_id: OrgId,
    pub actor_role: Role,
    pub invitation_id: Uuid,
    pub request_id: Option<Uuid>,
}

pub async fn approve_tenant_invitation(
    command: ApproveTenantInvitationCommand<'_>,
) -> Result<TenantActivationResult, AppError> {
    let invitation = command
        .invitations
        .get(command.invitation_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if invitation.org_id != command.actor_org_id {
        return Err(AppError::Forbidden);
    }
    require_staff_scope(
        command.users,
        command.actor_id,
        command.actor_org_id,
        command.actor_role,
        invitation.branch_id,
    )
    .await?;
    ensure_transition(invitation.status, InvitationStatus::Approved)?;
    if invitation.accepted_by_user_id == Some(command.actor_id) {
        return Err(AppError::Forbidden);
    }

    let result = command
        .activation
        .activate(
            command.ctx,
            &ActivateTenantRecord {
                invitation_id: command.invitation_id,
                approved_by_user_id: command.actor_id,
                request_id: command.request_id,
            },
        )
        .await?;

    if let Err(error) = command
        .email_sender
        .send(InvitationEmail {
            invitation_id: invitation.id,
            recipient: invitation.email,
            kind: InvitationEmailKind::Approved,
        })
        .await
    {
        tracing::warn!(
            invitation_id = %command.invitation_id,
            error = %error,
            "tenant activated but approval email delivery failed"
        );
    }
    Ok(result)
}

pub struct ListMyTenantInvitationsCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub actor_id: UserId,
    pub actor_role: Role,
}

pub async fn list_my_tenant_invitations(
    command: ListMyTenantInvitationsCommand<'_>,
) -> Result<Vec<TenantInvitation>, AppError> {
    if command.actor_role == Role::PlatformOperator {
        return Err(AppError::Forbidden);
    }
    let email = verified_email(command.users, command.actor_id).await?;
    command.invitations.list_for_email(&email).await
}

pub struct ResendTenantInvitationCommand<'a> {
    pub invitations: &'a dyn TenantInvitationRepository,
    pub users: &'a dyn UserRepository,
    pub email_sender: &'a dyn InvitationEmailSender,
    pub actor_id: UserId,
    pub actor_org_id: OrgId,
    pub actor_role: Role,
    pub invitation_id: Uuid,
    pub cooldown_secs: i64,
}

pub async fn resend_tenant_invitation(
    command: ResendTenantInvitationCommand<'_>,
) -> Result<TenantInvitation, AppError> {
    let mut invitation = command
        .invitations
        .get(command.invitation_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if invitation.org_id != command.actor_org_id {
        return Err(AppError::Forbidden);
    }
    require_staff_scope(
        command.users,
        command.actor_id,
        command.actor_org_id,
        command.actor_role,
        invitation.branch_id,
    )
    .await?;
    if matches!(
        invitation.status,
        InvitationStatus::Approved
            | InvitationStatus::Active
            | InvitationStatus::Cancelled
            | InvitationStatus::Rejected
            | InvitationStatus::Expired
    ) {
        return Err(AppError::Conflict(
            "this invitation can no longer be resent".into(),
        ));
    }

    let now = OffsetDateTime::now_utc();
    let cooldown = command.cooldown_secs.max(0);
    let available_at = invitation.last_sent_at + Duration::seconds(cooldown);
    if now < available_at {
        return Err(AppError::Conflict(format!(
            "invitation can be resent in {} seconds",
            (available_at - now).whole_seconds().max(1)
        )));
    }
    let expires_at = now + Duration::days(7);
    if !command
        .invitations
        .resend(
            invitation.id,
            invitation.status,
            expires_at,
            now - Duration::seconds(cooldown),
        )
        .await?
    {
        return Err(AppError::Conflict(
            "invitation changed while it was being resent".into(),
        ));
    }

    command
        .email_sender
        .send(InvitationEmail {
            invitation_id: invitation.id,
            recipient: invitation.email.clone(),
            kind: InvitationEmailKind::Resent,
        })
        .await?;
    invitation.expires_at = expires_at;
    invitation.last_sent_at = now;
    Ok(invitation)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantOnboardingStatus {
    pub invitation_id: Uuid,
    pub status: InvitationStatus,
    pub require_identity_verification: bool,
    pub next_action: &'static str,
}

pub async fn tenant_onboarding_status(
    command: GetTenantInvitationCommand<'_>,
) -> Result<TenantOnboardingStatus, AppError> {
    let invitation = get_tenant_invitation(command).await?;
    let status = effective_status(&invitation);
    let next_action = match status {
        InvitationStatus::Invited => "accept_or_decline",
        InvitationStatus::EmailVerified => "continue_onboarding",
        InvitationStatus::KycPending => "complete_identity_verification",
        InvitationStatus::KycFailed => "contact_property",
        InvitationStatus::KycVerified | InvitationStatus::ApprovalPending => "wait_for_approval",
        InvitationStatus::Approved => "activation_in_progress",
        InvitationStatus::Active => "open_tenant_home",
        InvitationStatus::Rejected | InvitationStatus::Cancelled | InvitationStatus::Expired => {
            "none"
        }
    };
    Ok(TenantOnboardingStatus {
        invitation_id: invitation.id,
        status,
        require_identity_verification: invitation.require_identity_verification,
        next_action,
    })
}

fn effective_status(invitation: &TenantInvitation) -> InvitationStatus {
    if invitation.status == InvitationStatus::Invited
        && invitation.expires_at <= OffsetDateTime::now_utc()
    {
        InvitationStatus::Expired
    } else {
        invitation.status
    }
}

async fn require_staff_scope(
    users: &dyn UserRepository,
    actor_id: UserId,
    actor_org_id: OrgId,
    actor_role: Role,
    invitation_branch: BranchId,
) -> Result<(), AppError> {
    match actor_role {
        Role::Owner => Ok(()),
        Role::Manager
            if users.branch_in_org(actor_id, actor_org_id).await? == Some(invitation_branch) =>
        {
            Ok(())
        }
        Role::Manager | Role::Tenant | Role::Prospect | Role::PlatformOperator => {
            Err(AppError::Forbidden)
        }
    }
}

async fn authorize_invitation_view(
    users: &dyn UserRepository,
    actor_id: UserId,
    actor_org_id: OrgId,
    actor_role: Role,
    invitation: &TenantInvitation,
) -> Result<(), AppError> {
    if actor_role == Role::Owner && actor_org_id == invitation.org_id {
        return Ok(());
    }
    if actor_role == Role::Manager && actor_org_id == invitation.org_id {
        return require_staff_scope(
            users,
            actor_id,
            actor_org_id,
            actor_role,
            invitation.branch_id,
        )
        .await;
    }
    let email = verified_email(users, actor_id).await?;
    if email == invitation.email {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

async fn verified_email(users: &dyn UserRepository, actor_id: UserId) -> Result<String, AppError> {
    users
        .verified_email_of(actor_id)
        .await?
        .and_then(|email| normalize_email(&email))
        .ok_or(AppError::Forbidden)
}

async fn invitation_for_email(
    invitations: &dyn TenantInvitationRepository,
    invitation_id: Uuid,
    verified_email: &str,
) -> Result<TenantInvitation, AppError> {
    let invitation = invitations
        .get(invitation_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if invitation.email != verified_email {
        return Err(AppError::Forbidden);
    }
    Ok(invitation)
}

fn ensure_not_expired(invitation: &TenantInvitation) -> Result<(), AppError> {
    if invitation.expires_at <= OffsetDateTime::now_utc() {
        Err(AppError::Conflict("invitation has expired".into()))
    } else {
        Ok(())
    }
}

fn ensure_transition(from: InvitationStatus, to: InvitationStatus) -> Result<(), AppError> {
    if from.can_transition_to(to) {
        Ok(())
    } else {
        Err(AppError::Conflict(format!(
            "invalid invitation transition: {} -> {}",
            from.as_str(),
            to.as_str()
        )))
    }
}
