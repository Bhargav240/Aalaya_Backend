//! Marketplace use cases. Owners/managers PREPARE listings; the Platform
//! Operator is the publish gate. Browse/search/nearby reads are anonymous.
use crate::authz;
use crate::error::AppError;
use crate::ports::discovery::{Listing, ListingInput, ListingRepository, ListingStatus};
use pg_domain::{BranchId, OrgId, Role};

/// Save listing content as a draft (owner/manager prep).
pub async fn prepare_listing(
    repo: &dyn ListingRepository,
    role: Role,
    org_id: OrgId,
    branch_id: BranchId,
    input: ListingInput,
) -> Result<(), AppError> {
    if !authz::can_prepare_listing(role) {
        return Err(AppError::Forbidden);
    }
    if input.title.trim().is_empty() {
        return Err(AppError::Validation("title cannot be empty".into()));
    }
    repo.upsert_draft(org_id, branch_id, &input).await
}

/// "Send for approval": draft/rejected -> pending (owner/manager).
pub async fn submit_listing(
    repo: &dyn ListingRepository,
    role: Role,
    branch_id: BranchId,
) -> Result<(), AppError> {
    if !authz::can_prepare_listing(role) {
        return Err(AppError::Forbidden);
    }
    let listing = repo
        .get(branch_id.as_uuid())
        .await?
        .ok_or(AppError::NotFound)?;
    match listing.status {
        ListingStatus::Draft | ListingStatus::Rejected => {
            repo.set_status(branch_id, ListingStatus::Pending, None)
                .await
        }
        ListingStatus::Pending => Ok(()), // idempotent
        ListingStatus::Live => Err(AppError::Validation("already published".into())),
        ListingStatus::Unlisted => Err(AppError::Validation(
            "unlisted by the platform — contact support".into(),
        )),
    }
}

/// The publish gate: pending -> live (Platform Operator only).
pub async fn operator_publish(
    repo: &dyn ListingRepository,
    role: Role,
    branch_id: BranchId,
) -> Result<(), AppError> {
    if !authz::can_publish_listings(role) {
        return Err(AppError::Forbidden);
    }
    repo.set_status(branch_id, ListingStatus::Live, None).await
}

pub async fn operator_reject(
    repo: &dyn ListingRepository,
    role: Role,
    branch_id: BranchId,
    note: &str,
) -> Result<(), AppError> {
    if !authz::can_publish_listings(role) {
        return Err(AppError::Forbidden);
    }
    repo.set_status(branch_id, ListingStatus::Rejected, Some(note))
        .await
}

pub async fn operator_unlist(
    repo: &dyn ListingRepository,
    role: Role,
    branch_id: BranchId,
) -> Result<(), AppError> {
    if !authz::can_publish_listings(role) {
        return Err(AppError::Forbidden);
    }
    repo.set_status(branch_id, ListingStatus::Unlisted, None)
        .await
}

pub async fn list_org_listings(
    repo: &dyn ListingRepository,
    role: Role,
    org_id: OrgId,
) -> Result<Vec<Listing>, AppError> {
    if !authz::can_prepare_listing(role) {
        return Err(AppError::Forbidden);
    }
    repo.list_by_org(org_id).await
}

pub async fn operator_queue(
    repo: &dyn ListingRepository,
    role: Role,
) -> Result<Vec<Listing>, AppError> {
    if !authz::can_publish_listings(role) {
        return Err(AppError::Forbidden);
    }
    repo.list_by_status(ListingStatus::Pending).await
}

pub async fn discover_list(
    repo: &dyn ListingRepository,
    limit: i64,
) -> Result<Vec<Listing>, AppError> {
    repo.list_published(limit).await
}

pub async fn discover_search(
    repo: &dyn ListingRepository,
    q: Option<String>,
    city: Option<String>,
    limit: i64,
) -> Result<Vec<Listing>, AppError> {
    repo.search(q, city, limit).await
}

pub async fn discover_nearby(
    repo: &dyn ListingRepository,
    lat: f64,
    lng: f64,
    limit: i64,
) -> Result<Vec<Listing>, AppError> {
    repo.nearby(lat, lng, limit).await
}

pub async fn discover_get(
    repo: &dyn ListingRepository,
    branch_id: uuid::Uuid,
) -> Result<Option<Listing>, AppError> {
    Ok(repo
        .get(branch_id)
        .await?
        .filter(|l| l.status == ListingStatus::Live))
}
