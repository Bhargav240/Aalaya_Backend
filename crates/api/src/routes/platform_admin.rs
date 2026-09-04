//! Read-only platform diagnostics. These endpoints never enter a tenant
//! schema and deliberately return identifiers/status, not secrets or PII.
use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::require_platform_operator;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use pg_domain::OrgId;
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/platform/admin/system/health", get(system_health))
        .route("/platform/admin/organizations", get(organizations))
        .route("/platform/admin/logs", get(logs))
        .route("/platform/admin/logs/summary", get(log_summary))
}

#[derive(Deserialize)]
struct LogQuery {
    level: Option<String>,
    status_code: Option<u16>,
    route: Option<String>,
    page: Option<usize>,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct LogResponse {
    items: Vec<crate::state::PlatformLog>,
    page: usize,
    limit: usize,
    total: usize,
}

async fn logs(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(query): Query<LogQuery>,
) -> Result<Json<LogResponse>, ApiError> {
    require_platform_operator(&claims)?;
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let all = state
        .platform_logs
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let filtered: Vec<_> = all
        .iter()
        .filter(|entry| {
            query
                .level
                .as_ref()
                .is_none_or(|v| entry.level.eq_ignore_ascii_case(v))
        })
        .filter(|entry| query.status_code.is_none_or(|v| entry.status_code == v))
        .filter(|entry| query.route.as_ref().is_none_or(|v| entry.route.contains(v)))
        .cloned()
        .collect();
    let total = filtered.len();
    let start = (page - 1).saturating_mul(limit);
    let items = filtered.into_iter().skip(start).take(limit).collect();
    Ok(Json(LogResponse {
        items,
        page,
        limit,
        total,
    }))
}

#[derive(Serialize)]
struct LogSummary {
    total: usize,
    errors: usize,
    warnings: usize,
    average_latency_ms: u128,
}

async fn log_summary(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<LogSummary>, ApiError> {
    require_platform_operator(&claims)?;
    let logs = state
        .platform_logs
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let total = logs.len();
    let errors = logs.iter().filter(|l| l.status_code >= 500).count();
    let warnings = logs
        .iter()
        .filter(|l| (400..500).contains(&l.status_code))
        .count();
    let average_latency_ms = if total == 0 {
        0
    } else {
        logs.iter().map(|l| l.latency_ms).sum::<u128>() / total as u128
    };
    Ok(Json(LogSummary {
        total,
        errors,
        warnings,
        average_latency_ms,
    }))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    database_ready: bool,
    request_id: Option<String>,
}

async fn system_health(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<HealthResponse>, ApiError> {
    require_platform_operator(&claims)?;
    Ok(Json(HealthResponse {
        status: if state.db_ready() { "ok" } else { "degraded" },
        database_ready: state.db_ready(),
        request_id: pg_application::context::current_request_id().map(|id| id.to_string()),
    }))
}

#[derive(Serialize)]
struct OrganizationInventory {
    organizations: Vec<String>,
    count: usize,
}

async fn organizations(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<OrganizationInventory>, ApiError> {
    require_platform_operator(&claims)?;
    let deps = state.deps()?;
    let ids: Vec<OrgId> = deps.org_repo.list_all_live().await?;
    let organizations = ids.into_iter().map(|id| id.to_string()).collect::<Vec<_>>();
    let count = organizations.len();
    Ok(Json(OrganizationInventory {
        organizations,
        count,
    }))
}
