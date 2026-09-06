//! Marketplace enquiry funnel for authenticated email-OTP explorers.
use crate::error::AppError;
use crate::ports::repositories::{EnquiryRecord, EnquiryRepository, UserRepository};
use pg_domain::{BranchId, OrgId, Role, UserId};
use uuid::Uuid;

pub struct AuthenticatedEnquiryCommand<'a> {
    pub enquiry_repo: &'a dyn EnquiryRepository,
    pub user_repo: &'a dyn UserRepository,
    pub branch_id: BranchId,
    pub user_id: UserId,
    pub note: Option<&'a str>,
    pub phone: Option<&'a str>,
}

/// Enquiries are submitted by an authenticated explorer. The contact phone/email
/// is captured and stored on the lead and the explorer's profile.
pub async fn create_authenticated_enquiry(
    command: AuthenticatedEnquiryCommand<'_>,
) -> Result<(), AppError> {
    let org_id = command
        .enquiry_repo
        .org_of_branch(command.branch_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if let Some(p) = command.phone.filter(|s| !s.trim().is_empty()) {
        let _ = command.user_repo.update_profile(command.user_id, None, Some(p)).await;
    }

    let contact = if let Some(p) = command.phone.filter(|s| !s.trim().is_empty()) {
        p.to_string()
    } else if let Ok(Some(email)) = command.user_repo.verified_email_of(command.user_id).await {
        email
    } else {
        "email-authenticated".to_string()
    };

    command
        .enquiry_repo
        .create(
            Uuid::new_v4(),
            org_id,
            command.branch_id,
            command.user_id,
            &contact,
            command.note,
        )
        .await
}

/// The manager sees only their own branch's leads; the owner sees the org.
pub async fn list_enquiries(
    enquiry_repo: &dyn EnquiryRepository,
    user_repo: &dyn UserRepository,
    role: Role,
    actor: UserId,
    org_id: OrgId,
) -> Result<Vec<EnquiryRecord>, AppError> {
    use crate::authz;
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    if role == Role::Manager {
        let branch = user_repo
            .branch_in_org(actor, org_id)
            .await?
            .ok_or(AppError::Forbidden)?;
        enquiry_repo.list_for_branch(branch).await
    } else {
        enquiry_repo.list_for_org(org_id).await
    }
}

pub async fn set_enquiry_status(
    enquiry_repo: &dyn EnquiryRepository,
    role: Role,
    id: Uuid,
    status: &str,
) -> Result<(), AppError> {
    if !crate::authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    if !matches!(status, "new" | "contacted" | "closed") {
        return Err(AppError::Validation("invalid enquiry status".into()));
    }
    enquiry_repo.set_status(id, status).await
}
