use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{actor_of, require_resident_management_scope, resolve_ctx};
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use pg_application::services;
use pg_application::AppError;
use pg_domain::ResidentId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/residents/{id}/move-out", post(move_out))
}

#[derive(Deserialize)]
struct DeductionReq {
    reason: String,
    amount_paise: i64,
}

#[derive(Deserialize)]
struct MoveOutReq {
    last_day: String, // YYYY-MM-DD
    notice_days: i64,
    deductions: Option<Vec<DeductionReq>>,
}

#[derive(Serialize)]
struct SettlementOut {
    rent_to_date_paise: i64,
    paid_total_paise: i64,
    outstanding_paise: i64,
    deposit_held_paise: i64,
    deductions_total_paise: i64,
    refund_paise: i64,
    forfeited: bool,
    note: String,
}

fn parse_date(s: &str) -> Option<time::Date> {
    let mut it = s.split('-');
    let y: i32 = it.next()?.parse().ok()?;
    let m: u8 = it.next()?.parse().ok()?;
    let d: u8 = it.next()?.parse().ok()?;
    let month = time::Month::try_from(m).ok()?;
    time::Date::from_calendar_date(y, month, d).ok()
}

async fn move_out(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(resident_id): Path<Uuid>,
    Json(req): Json<MoveOutReq>,
) -> Result<Json<SettlementOut>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_resident_management_scope(deps, &ctx, &claims, ResidentId::new(resident_id)).await?;
    let actor = actor_of(&claims);
    let last_day = parse_date(&req.last_day)
        .ok_or(AppError::Validation("last_day must be YYYY-MM-DD".into()))?;
    let deductions = req
        .deductions
        .unwrap_or_default()
        .into_iter()
        .map(|d| services::Deduction {
            reason: d.reason,
            amount_paise: d.amount_paise,
        })
        .collect();
    let s = services::move_out(services::MoveOutCommand {
        payment_repo: deps.payment_repo.as_ref(),
        resident_repo: deps.resident_repo.as_ref(),
        settlement_repo: deps.settlement_repo.as_ref(),
        billing: deps.billing.as_ref(),
        ctx: &ctx,
        role: claims.role,
        actor: &actor,
        resident_id: ResidentId::new(resident_id),
        last_day,
        notice_days: req.notice_days,
        deductions,
    })
    .await?;
    Ok(Json(SettlementOut {
        rent_to_date_paise: s.rent_to_date_paise,
        paid_total_paise: s.paid_total_paise,
        outstanding_paise: s.outstanding_paise,
        deposit_held_paise: s.deposit_held_paise,
        deductions_total_paise: s.deductions_total_paise,
        refund_paise: s.refund_paise,
        forfeited: s.forfeited,
        note: s.note,
    }))
}
