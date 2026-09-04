//! Small helpers shared by handlers: resolve the tenant context, build the
//! audit actor from claims, and shape maker-checker outcomes.
use crate::state::Deps;
use pg_application::context::current_request_id;
use pg_application::ports::auth::Claims;
use pg_application::ports::repositories::AuditActor;
use pg_application::ports::tenancy::TenantContext;
use pg_application::services::MutationOutcome;
use pg_application::AppError;
use pg_domain::{BranchId, OrgId, ResidentId, Role, RoomId};
use serde::Serialize;

pub async fn resolve_ctx(deps: &Deps, org_id: OrgId) -> Result<TenantContext, AppError> {
    use pg_application::ports::tenancy::TenancyError;
    deps.resolver.resolve(org_id).await.map_err(|e| match e {
        TenancyError::Suspended(status) => AppError::OrgSuspended(status),
        other => AppError::Tenancy(other.to_string()),
    })
}

pub fn actor_of(claims: &Claims) -> AuditActor {
    AuditActor {
        user_id: Some(claims.user_id),
        request_id: current_request_id(),
    }
}

/// Platform diagnostics are intentionally separate from tenant management.
/// Only the dedicated platform operator role may access them.
pub fn require_platform_operator(claims: &Claims) -> Result<(), AppError> {
    if claims.role == Role::PlatformOperator {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

/// Management endpoints containing resident PII are branch-scoped for
/// managers. Owners remain org-wide; tenants must use the self-service routes.
pub async fn require_branch_management_scope(
    deps: &Deps,
    claims: &Claims,
    branch_id: BranchId,
) -> Result<(), AppError> {
    match claims.role {
        Role::Owner => Ok(()),
        Role::Manager
            if deps
                .user_repo
                .branch_in_org(claims.user_id, claims.org_id)
                .await?
                == Some(branch_id) =>
        {
            Ok(())
        }
        _ => Err(AppError::Forbidden),
    }
}

/// Resolve a resident before any management read or mutation so a manager
/// cannot substitute an ID belonging to another branch.
pub async fn require_resident_management_scope(
    deps: &Deps,
    ctx: &TenantContext,
    claims: &Claims,
    resident_id: ResidentId,
) -> Result<(), AppError> {
    match claims.role {
        Role::Owner => Ok(()),
        Role::Manager => {
            let assigned = deps
                .user_repo
                .branch_in_org(claims.user_id, claims.org_id)
                .await?;
            let resident = deps
                .resident_repo
                .get(ctx, resident_id)
                .await?
                .ok_or(AppError::NotFound)?;
            if assigned.map(|id| id.as_uuid()) == Some(resident.branch_id) {
                Ok(())
            } else {
                Err(AppError::Forbidden)
            }
        }
        Role::Tenant | Role::PlatformOperator | Role::Prospect => Err(AppError::Forbidden),
    }
}

/// Room and bed mutations are limited to a manager's assigned branch. Other
/// branches may expose a separate availability summary, but never operational
/// room/bed mutation endpoints.
pub async fn require_room_management_scope(
    deps: &Deps,
    ctx: &TenantContext,
    claims: &Claims,
    room_id: RoomId,
) -> Result<(), AppError> {
    match claims.role {
        Role::Owner => Ok(()),
        Role::Manager => {
            let assigned = deps
                .user_repo
                .branch_in_org(claims.user_id, claims.org_id)
                .await?;
            let branch = deps
                .room_repo
                .branch_of(ctx, room_id)
                .await?
                .ok_or(AppError::NotFound)?;
            if assigned == Some(branch) {
                Ok(())
            } else {
                Err(AppError::Forbidden)
            }
        }
        Role::Tenant | Role::PlatformOperator | Role::Prospect => Err(AppError::Forbidden),
    }
}

#[derive(Serialize)]
pub struct ChangeResp {
    pub status: &'static str,
    pub change_request_id: Option<String>,
}

pub fn change_resp(outcome: MutationOutcome) -> ChangeResp {
    match outcome {
        MutationOutcome::Applied => ChangeResp {
            status: "applied",
            change_request_id: None,
        },
        MutationOutcome::Pending { change_request_id } => ChangeResp {
            status: "pending",
            change_request_id: Some(change_request_id.to_string()),
        },
    }
}
