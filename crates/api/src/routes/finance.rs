use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::resolve_ctx;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use pg_application::ports::repositories::FinanceReport;
use pg_application::services;
use pg_domain::BranchId;
use serde::Serialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/finance/branch/{id}", get(branch))
        .route("/finance/org", get(org))
        .route("/branches/{id}/dues", get(branch_dues))
}

#[derive(Serialize)]
struct ReportOut {
    collected_paise: i64,
    deposits_collected_paise: i64,
    deposits_paise: i64,
    accrued_paise: i64,
    outstanding_paise: i64,
    residents_active: i64,
    residents_total: i64,
    beds_total: i32,
    beds_occupied: i32,
}

fn out(r: FinanceReport) -> ReportOut {
    ReportOut {
        collected_paise: r.collected_paise,
        deposits_collected_paise: r.deposits_collected_paise,
        deposits_paise: r.deposits_paise,
        accrued_paise: r.accrued_paise,
        outstanding_paise: r.outstanding_paise,
        residents_active: r.residents_active,
        residents_total: r.residents_total,
        beds_total: r.beds_total,
        beds_occupied: r.beds_occupied,
    }
}

async fn branch(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ReportOut>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let r = services::finance_branch(
        deps.finance_repo.as_ref(),
        deps.user_repo.as_ref(),
        deps.vacancy_repo.as_ref(),
        &ctx,
        claims.role,
        claims.user_id,
        BranchId::new(id),
    )
    .await?;
    Ok(Json(out(r)))
}

#[derive(Serialize)]
struct BranchOut {
    branch_id: String,
    branch_name: String,
    #[serde(flatten)]
    report: ReportOut,
}

#[derive(Serialize)]
struct OrgOut {
    total: ReportOut,
    branches: Vec<BranchOut>,
}

async fn org(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<OrgOut>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let (total, branches) = services::finance_org(
        deps.finance_repo.as_ref(),
        deps.vacancy_repo.as_ref(),
        &ctx,
        claims.role,
    )
    .await?;
    Ok(Json(OrgOut {
        total: out(total),
        branches: branches
            .into_iter()
            .map(|b| BranchOut {
                branch_id: b.branch_id.to_string(),
                branch_name: b.branch_name,
                report: out(b.report),
            })
            .collect(),
    }))
}

/// A7 batch endpoint — one request returns every active resident's dues in a
/// branch, replacing the N-ledger fan-out the payments screen used to do.
#[derive(Serialize)]
struct ResidentDueOut {
    resident_id: String,
    full_name: String,
    plan_kind: String,
    status: String,
    rent_paise: i64,
    deposit_paise: i64,
    paid_total_paise: i64,
    accrued_paise: i64,
    outstanding_paise: i64,
}

async fn branch_dues(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ResidentDueOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let dues = services::branch_dues(
        deps.finance_repo.as_ref(),
        deps.user_repo.as_ref(),
        &ctx,
        claims.role,
        claims.user_id,
        BranchId::new(id),
    )
    .await?;
    Ok(Json(
        dues.into_iter()
            .map(|d| ResidentDueOut {
                resident_id: d.resident_id.to_string(),
                full_name: d.full_name,
                plan_kind: d.plan_kind,
                status: d.status,
                rent_paise: d.rent_paise,
                deposit_paise: d.deposit_paise,
                paid_total_paise: d.paid_total_paise,
                accrued_paise: d.accrued_paise,
                outstanding_paise: d.outstanding_paise,
            })
            .collect(),
    ))
}
