//! Finance rollups. Owner sees org + any branch; manager sees own branch only.
use crate::authz;
use crate::error::AppError;
use crate::ports::repositories::{
    BranchFinance, FinanceReport, FinanceRepository, ResidentDue, UserRepository, VacancyRepository,
};
use crate::ports::tenancy::TenantContext;
use pg_domain::{BranchId, Role, UserId};

/// Bed counts come from a live tenant-schema query (`VacancyRepository::compute_for_branch`),
/// not the resident columns already on `FinanceReport` -- a branch can have
/// beds with nobody assigned yet, which the resident count alone can't show.
async fn with_bed_counts(
    vacancy_repo: &dyn VacancyRepository,
    ctx: &TenantContext,
    branch_id: BranchId,
    mut report: FinanceReport,
) -> Result<FinanceReport, AppError> {
    let (total, available, _price) = vacancy_repo.compute_for_branch(ctx, branch_id).await?;
    report.beds_total = total;
    report.beds_occupied = total - available;
    Ok(report)
}

/// Finance reads for a branch use the same staff scope as every other branch
/// operation: owners may read any branch, managers only their assigned one.
/// Keeping the policy in one helper prevents the report and dues endpoints
/// from drifting apart as roles evolve.
async fn require_branch_finance_access(
    user_repo: &dyn UserRepository,
    ctx: &TenantContext,
    role: Role,
    user_id: UserId,
    branch_id: BranchId,
) -> Result<(), AppError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => {
            if user_repo
                .has_branch_access(user_id, ctx.org_id, branch_id)
                .await?
            {
                Ok(())
            } else {
                Err(AppError::Forbidden)
            }
        }
        Role::Tenant | Role::PlatformOperator | Role::Prospect => {
            Err(AppError::Forbidden)
        }
    }
}

pub async fn finance_branch(
    repo: &dyn FinanceRepository,
    user_repo: &dyn UserRepository,
    vacancy_repo: &dyn VacancyRepository,
    ctx: &TenantContext,
    role: Role,
    user_id: UserId,
    branch_id: BranchId,
) -> Result<FinanceReport, AppError> {
    require_branch_finance_access(user_repo, ctx, role, user_id, branch_id).await?;
    let report = repo.branch_report(ctx, branch_id).await?;
    with_bed_counts(vacancy_repo, ctx, branch_id, report).await
}

/// Batch dues list for the payments screen (A7). Same authorization surface
/// as `finance_branch` (owner or branch-scoped manager); returns every active
/// resident's rollup in one query so the client doesn't fan out N ledger calls.
pub async fn branch_dues(
    repo: &dyn FinanceRepository,
    user_repo: &dyn UserRepository,
    ctx: &TenantContext,
    role: Role,
    user_id: UserId,
    branch_id: BranchId,
) -> Result<Vec<ResidentDue>, AppError> {
    require_branch_finance_access(user_repo, ctx, role, user_id, branch_id).await?;
    repo.branch_dues(ctx, branch_id).await
}

pub async fn finance_org(
    repo: &dyn FinanceRepository,
    vacancy_repo: &dyn VacancyRepository,
    ctx: &TenantContext,
    role: Role,
) -> Result<(FinanceReport, Vec<BranchFinance>), AppError> {
    if !authz::is_owner(role) {
        return Err(AppError::Forbidden);
    }
    let (mut total, branches) = repo.org_report(ctx).await?;
    total.beds_total = 0;
    total.beds_occupied = 0;
    let mut enriched = Vec::with_capacity(branches.len());
    for mut b in branches {
        let (bt, ba, _price) = vacancy_repo
            .compute_for_branch(ctx, BranchId::new(b.branch_id))
            .await?;
        b.report.beds_total = bt;
        b.report.beds_occupied = bt - ba;
        total.beds_total += bt;
        total.beds_occupied += bt - ba;
        enriched.push(b);
    }
    Ok((total, enriched))
}
