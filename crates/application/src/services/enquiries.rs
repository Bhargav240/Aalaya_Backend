//! Marketplace enquiry funnel for authenticated email-OTP explorers.
use crate::error::AppError;
use crate::ports::repositories::{EnquiryRecord, EnquiryRepository, UserRepository};
use pg_domain::{BranchId, OrgId, Role, UserId};
use uuid::Uuid;

pub struct AuthenticatedEnquiryCommand<'a> {
    pub enquiry_repo: &'a dyn EnquiryRepository,
    pub branch_id: BranchId,
    pub user_id: UserId,
    pub note: Option<&'a str>,
}

/// Enquiries are now submitted by an authenticated explorer. The legacy
/// `phone` field remains the contact column until the broader contact-data
/// migration; storing the stable user ID is the authorization boundary.
pub async fn create_authenticated_enquiry(
    command: AuthenticatedEnquiryCommand<'_>,
) -> Result<(), AppError> {
    let org_id = command
        .enquiry_repo
        .org_of_branch(command.branch_id)
        .await?
        .ok_or(AppError::NotFound)?;
    command
        .enquiry_repo
        .create(
            Uuid::new_v4(),
            org_id,
            command.branch_id,
            command.user_id,
            "email-authenticated",
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
