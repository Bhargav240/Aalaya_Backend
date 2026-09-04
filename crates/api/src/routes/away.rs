//! Away days: tenant self-reports; org views everything.
use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{actor_of, resolve_ctx};
use axum::extract::{Path, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use pg_application::services;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/away-periods", post(report).get(list_all))
        .route("/me/away-periods", get(list_mine))
        // DELETE keeps its existing meaning; PATCH is the new correction path.
        .route("/me/away-periods/{id}", patch(update).delete(cancel))
}

#[derive(Deserialize)]
struct ReportReq {
    from_date: String, // YYYY-MM-DD
    to_date: String,
    note: Option<String>,
}

#[derive(Serialize)]
struct AwayOut {
    id: String,
    resident_id: Option<String>,
    from_date: String,
    to_date: String,
    note: Option<String>,
}

/// The staff view: who is away, which bed, and where.
#[derive(Serialize)]
struct AwayStaffOut {
    id: String,
    resident_id: String,
    resident_name: String,
    room_number: Option<String>,
    branch_id: String,
    branch_name: String,
    from_date: String,
    to_date: String,
    note: Option<String>,
}

fn parse_date(s: &str, field: &str) -> Result<time::Date, pg_application::AppError> {
    let err = || pg_application::AppError::Validation(format!("{field} must be YYYY-MM-DD"));
    let p: Vec<&str> = s.split('-').collect();
    if p.len() != 3 {
        return Err(err());
    }
    let (y, m, d) = (
        p[0].parse::<i32>().map_err(|_| err())?,
        p[1].parse::<u8>().map_err(|_| err())?,
        p[2].parse::<u8>().map_err(|_| err())?,
    );
    let month = time::Month::try_from(m).map_err(|_| err())?;
    time::Date::from_calendar_date(y, month, d).map_err(|_| err())
}

async fn report(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ReportReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let actor = actor_of(&claims);
    let id = services::away::report(services::away::ReportAwayCommand {
        away_repo: deps.away_repo.as_ref(),
        resident_repo: deps.resident_repo.as_ref(),
        clock: deps.clock.as_ref(),
        ctx: &ctx,
        user_id: claims.user_id,
        actor: &actor,
        input: services::away::AwayPeriodInput {
            from_date: parse_date(&req.from_date, "from_date")?,
            to_date: parse_date(&req.to_date, "to_date")?,
            note: req.note,
        },
    })
    .await?;
    Ok(Json(serde_json::json!({ "id": id.to_string() })))
}

async fn list_mine(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<AwayOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let items = services::away::list_mine(
        deps.away_repo.as_ref(),
        deps.resident_repo.as_ref(),
        &ctx,
        claims.user_id,
    )
    .await?;
    Ok(Json(
        items
            .into_iter()
            .map(|p| AwayOut {
                id: p.id.to_string(),
                resident_id: None,
                from_date: p.from_date.to_string(),
                to_date: p.to_date.to_string(),
                note: p.note,
            })
            .collect(),
    ))
}

async fn list_all(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<AwayStaffOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let items = services::away::list_all(
        deps.away_repo.as_ref(),
        deps.user_repo.as_ref(),
        &ctx,
        claims.role,
        claims.user_id,
    )
    .await?;
    Ok(Json(
        items
            .into_iter()
            .map(|d| AwayStaffOut {
                id: d.period.id.to_string(),
                resident_id: d.period.resident_id.to_string(),
                resident_name: d.resident_name,
                room_number: d.room_number,
                branch_id: d.branch_id.to_string(),
                branch_name: d.branch_name,
                from_date: d.period.from_date.to_string(),
                to_date: d.period.to_date.to_string(),
                note: d.period.note,
            })
            .collect(),
    ))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ReportReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let actor = actor_of(&claims);
    services::away::update(
        deps.away_repo.as_ref(),
        deps.resident_repo.as_ref(),
        deps.clock.as_ref(),
        &ctx,
        claims.user_id,
        &actor,
        id,
        services::away::AwayPeriodInput {
            from_date: parse_date(&req.from_date, "from_date")?,
            to_date: parse_date(&req.to_date, "to_date")?,
            note: req.note,
        },
    )
    .await?;
    Ok(Json(serde_json::json!({ "status": "updated" })))
}

async fn cancel(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let actor = actor_of(&claims);
    services::away::cancel(
        deps.away_repo.as_ref(),
        deps.resident_repo.as_ref(),
        &ctx,
        claims.user_id,
        &actor,
        id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}
