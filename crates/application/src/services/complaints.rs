//! Complaints (state machine + visibility) and notices.
use crate::authz;
use crate::error::AppError;
use crate::ports::notifier::Notifier;
use crate::ports::repositories::{
    AuditActor, ComplaintRepository, ComplaintSummary, NoticeRepository, NoticeSummary,
    ResidentRepository, UserRepository,
};
use crate::ports::tenancy::TenantContext;
use pg_domain::{BranchId, ComplaintId, ComplaintState, DomainError, NoticeId, Role, UserId};
use uuid::Uuid;

/// Anyone signed in may raise a complaint. Tenants/managers file against their
/// own branch; owners may file a branch or a business-level complaint.
pub struct RaiseComplaintCommand<'a> {
    pub complaint_repo: &'a dyn ComplaintRepository,
    pub user_repo: &'a dyn UserRepository,
    pub ctx: &'a TenantContext,
    pub role: Role,
    pub actor: &'a AuditActor,
    pub user_id: UserId,
    pub title: String,
    pub description: Option<String>,
    pub business_level: bool,
    pub branch_id_req: Option<BranchId>,
}
pub async fn raise_complaint(command: RaiseComplaintCommand<'_>) -> Result<Uuid, AppError> {
    let RaiseComplaintCommand {
        complaint_repo,
        user_repo,
        ctx,
        role,
        actor,
        user_id,
        title,
        description,
        business_level,
        branch_id_req,
    } = command;
    if title.trim().is_empty() {
        return Err(AppError::Validation("title required".into()));
    }
    let (branch, is_biz) = match role {
        Role::Owner => (branch_id_req, business_level),
        Role::Manager => {
            let b = match branch_id_req {
                Some(bid) => {
                    if user_repo.has_branch_access(user_id, ctx.org_id, bid).await? {
                        Some(bid)
                    } else {
                        return Err(AppError::Forbidden);
                    }
                }
                None => user_repo.branch_in_org(user_id, ctx.org_id).await?,
            };
            (b, false)
        }
        Role::Tenant => {
            (user_repo.branch_in_org(user_id, ctx.org_id).await?, false)
        }
        Role::PlatformOperator | Role::Prospect => return Err(AppError::Forbidden),
    };
    let id = ComplaintId::new(Uuid::new_v4());
    complaint_repo
        .raise(
            ctx,
            id,
            branch,
            user_id,
            &title,
            description.as_deref(),
            is_biz,
            actor,
        )
        .await?;
    Ok(id.as_uuid())
}

/// Visibility: tenant -> own; manager -> own branch(es), non-business; owner -> all (or selected branch).
pub async fn list_complaints(
    complaint_repo: &dyn ComplaintRepository,
    user_repo: &dyn UserRepository,
    ctx: &TenantContext,
    role: Role,
    user_id: UserId,
    branch_req: Option<BranchId>,
) -> Result<Vec<ComplaintSummary>, AppError> {
    let branch = match role {
        Role::Owner => branch_req,
        Role::Manager => {
            if let Some(req) = branch_req {
                if user_repo.has_branch_access(user_id, ctx.org_id, req).await? {
                    Some(req)
                } else {
                    return Err(AppError::Forbidden);
                }
            } else {
                user_repo.branch_in_org(user_id, ctx.org_id).await?
            }
        }
        Role::Tenant | Role::PlatformOperator | Role::Prospect => None,
    };
    complaint_repo
        .list_visible(ctx, role, user_id, branch)
        .await
}

pub struct TransitionComplaintCommand<'a> {
    pub complaint_repo: &'a dyn ComplaintRepository,
    pub user_repo: &'a dyn UserRepository,
    pub ctx: &'a TenantContext,
    pub role: Role,
    pub actor: &'a AuditActor,
    pub user_id: UserId,
    pub complaint_id: ComplaintId,
    pub to: ComplaintState,
}
pub async fn transition_complaint(command: TransitionComplaintCommand<'_>) -> Result<(), AppError> {
    let TransitionComplaintCommand {
        complaint_repo,
        user_repo,
        ctx,
        role,
        actor,
        user_id,
        complaint_id,
        to,
    } = command;
    let row = complaint_repo
        .get_state(ctx, complaint_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let from = ComplaintState::parse(&row.state)
        .ok_or_else(|| AppError::Internal("bad complaint state".into()))?;

    match role {
        Role::Owner => {}
        Role::Manager => {
            if row.is_business_level {
                return Err(AppError::Forbidden);
            }
            if let Some(bid) = row.branch_id {
                if !user_repo
                    .has_branch_access(user_id, ctx.org_id, BranchId::new(bid))
                    .await?
                {
                    return Err(AppError::Forbidden);
                }
            } else {
                return Err(AppError::Forbidden);
            }
        }
        Role::PlatformOperator | Role::Prospect => return Err(AppError::Forbidden),
        Role::Tenant => {
            if row.raised_by != user_id.as_uuid() {
                return Err(AppError::Forbidden);
            }
            // A tenant may only bounce their own resolved complaint back.
            if !(from == ComplaintState::Resolved && to == ComplaintState::InProgress) {
                return Err(AppError::Forbidden);
            }
        }
    }

    if !from.can_transition_to(to) {
        return Err(AppError::Domain(DomainError::InvalidTransition(format!(
            "{} -> {}",
            from.as_str(),
            to.as_str()
        ))));
    }
    complaint_repo
        .transition(
            ctx,
            complaint_id,
            from.as_str(),
            to.as_str(),
            user_id,
            actor,
        )
        .await
}

// ---- Notices ----
pub struct CreateNoticeCommand<'a> {
    pub notice_repo: &'a dyn NoticeRepository,
    pub user_repo: &'a dyn UserRepository,
    pub notifier: &'a dyn Notifier,
    pub ctx: &'a TenantContext,
    pub role: Role,
    pub actor: &'a AuditActor,
    pub user_id: UserId,
    pub branch_id_req: Option<BranchId>,
    pub title: String,
    pub body: Option<String>,
}
pub async fn create_notice(command: CreateNoticeCommand<'_>) -> Result<Uuid, AppError> {
    let CreateNoticeCommand {
        notice_repo,
        user_repo,
        notifier,
        ctx,
        role,
        actor,
        user_id,
        branch_id_req,
        title,
        body,
    } = command;
    if title.trim().is_empty() {
        return Err(AppError::Validation("title required".into()));
    }
    let branch = match role {
        Role::Owner => branch_id_req,
        Role::Manager => user_repo.branch_in_org(user_id, ctx.org_id).await?,
        Role::Tenant | Role::PlatformOperator | Role::Prospect => return Err(AppError::Forbidden),
    };
    let id = NoticeId::new(Uuid::new_v4());
    notice_repo
        .create(ctx, id, branch, &title, body.as_deref(), user_id, actor)
        .await?;
    // Best-effort fan-out; failure to notify must not fail the write.
    let _ = notifier.notify(ctx.org_id, branch, &title).await;
    Ok(id.as_uuid())
}

/// Tenants/managers see their branch's notices + org-wide; owner sees all.
pub async fn list_notices(
    notice_repo: &dyn NoticeRepository,
    user_repo: &dyn UserRepository,
    resident_repo: &dyn ResidentRepository,
    ctx: &TenantContext,
    role: Role,
    user_id: UserId,
) -> Result<Vec<NoticeSummary>, AppError> {
    let branch = if authz::is_owner(role) {
        None
    } else if role == Role::Tenant {
        // A tenant's branch lives on their `residents` row, not
        // `user_org_roles` (that table only carries a branch_id for staff).
        match resident_repo.resident_id_of_user(ctx, user_id).await? {
            Some(rid) => resident_repo
                .get(ctx, rid)
                .await?
                .map(|d| BranchId::new(d.branch_id)),
            None => None,
        }
    } else {
        user_repo.branch_in_org(user_id, ctx.org_id).await?
    };
    notice_repo.list_for(ctx, role, branch).await
}
