use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{
    actor_of, change_resp, require_branch_management_scope, require_resident_management_scope,
    resolve_ctx, ChangeResp,
};
use axum::extract::{Path, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use pg_application::ports::repositories::ChangeTarget;
use pg_application::{services, AppError};
use pg_domain::{BranchId, PlanKind, ResidentId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/branches/{branch_id}/residents", get(list).post(create))
        .route("/residents/{id}", patch(update).delete(remove))
        .route("/residents/{id}/link-user", axum::routing::post(link_user))
        .route(
            "/residents/{id}/assign-bed",
            axum::routing::post(assign_bed),
        )
}

#[derive(Deserialize)]
struct CreateResidentReq {
    full_name: String,
    phone: Option<String>,
    plan_kind: Option<String>,
    rent_paise: Option<i64>,
    deposit_paise: Option<i64>,
}

#[derive(Serialize)]
struct IdResp {
    id: String,
}

#[derive(Serialize)]
struct ResidentOut {
    id: String,
    branch_id: String,
    full_name: String,
    status: String,
    plan_kind: String,
}

async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
    Json(req): Json<CreateResidentReq>,
) -> Result<Json<IdResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_branch_management_scope(deps, &claims, BranchId::new(branch_id)).await?;
    let actor = actor_of(&claims);
    let plan = match req.plan_kind {
        Some(ref s) => {
            PlanKind::parse(s).ok_or(AppError::Validation("invalid plan_kind".into()))?
        }
        None => PlanKind::Monthly,
    };
    let id = services::create_resident(services::CreateResidentCommand {
        repo: deps.resident_repo.as_ref(),
        ctx: &ctx,
        role: claims.role,
        actor: &actor,
        branch_id: BranchId::new(branch_id),
        full_name: req.full_name,
        phone: req.phone,
        plan_kind: plan,
        rent_paise: req.rent_paise.unwrap_or(0),
        deposit_paise: req.deposit_paise.unwrap_or(0),
    })
    .await?;
    Ok(Json(IdResp { id: id.to_string() }))
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<Vec<ResidentOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_branch_management_scope(deps, &claims, BranchId::new(branch_id)).await?;
    let residents = services::list_residents(
        deps.resident_repo.as_ref(),
        &ctx,
        claims.role,
        BranchId::new(branch_id),
    )
    .await?;
    Ok(Json(
        residents
            .into_iter()
            .map(|r| ResidentOut {
                id: r.id.to_string(),
                branch_id: r.branch_id.to_string(),
                full_name: r.full_name,
                status: r.status,
                plan_kind: r.plan_kind,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct UpdateResidentReq {
    full_name: Option<String>,
    phone: Option<String>,
    plan_kind: Option<String>,
    rent_paise: Option<i64>,
    deposit_paise: Option<i64>,
}

async fn update(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateResidentReq>,
) -> Result<Json<ChangeResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_resident_management_scope(deps, &ctx, &claims, ResidentId::new(id)).await?;
    let actor = actor_of(&claims);
    let mut payload = Map::new();
    if let Some(v) = req.full_name {
        payload.insert("full_name".into(), json!(v));
    }
    if let Some(v) = req.phone {
        payload.insert("phone".into(), json!(v));
    }
    if let Some(v) = req.plan_kind {
        payload.insert("plan_kind".into(), json!(v));
    }
    // Rent & deposit revisions pass through maker-checker as well.
    if let Some(v) = req.rent_paise {
        payload.insert("rent_paise".into(), json!(v));
    }
    if let Some(v) = req.deposit_paise {
        payload.insert("deposit_paise".into(), json!(v));
    }
    let target = ChangeTarget {
        entity_type: "resident".into(),
        entity_id: id,
        operation: "update".into(),
        payload: Value::Object(payload),
    };
    let out = services::edit_entity(
        deps.cr_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        claims.user_id,
        target,
    )
    .await?;
    Ok(Json(change_resp(out)))
}

async fn remove(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ChangeResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_resident_management_scope(deps, &ctx, &claims, ResidentId::new(id)).await?;
    let actor = actor_of(&claims);
    let target = ChangeTarget {
        entity_type: "resident".into(),
        entity_id: id,
        operation: "delete".into(),
        payload: Value::Null,
    };
    let out = services::edit_entity(
        deps.cr_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        claims.user_id,
        target,
    )
    .await?;
    Ok(Json(change_resp(out)))
}

#[derive(Deserialize)]
struct AssignBedReq {
    /// Bed to assign; null/omitted clears the assignment.
    bed_id: Option<String>,
}

/// Assign or clear a resident's bed. The bed must be in the resident's branch
/// and free.
async fn assign_bed(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignBedReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_resident_management_scope(deps, &ctx, &claims, ResidentId::new(id)).await?;
    let actor = actor_of(&claims);
    let bed_id = match req.bed_id {
        Some(s) => Some(pg_domain::BedId::new(
            Uuid::parse_str(&s).map_err(|_| AppError::Validation("invalid bed_id".into()))?,
        )),
        None => None,
    };
    services::assign_bed(services::AssignBedCommand {
        resident_repo: deps.resident_repo.as_ref(),
        bed_repo: deps.bed_repo.as_ref(),
        room_repo: deps.room_repo.as_ref(),
        ctx: &ctx,
        role: claims.role,
        actor: &actor,
        resident_id: ResidentId::new(id),
        bed_id,
    })
    .await?;
    Ok(Json(json!({ "status": "assigned" })))
}

#[derive(Deserialize)]
struct LinkUserReq {
    user_id: String,
}

async fn link_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<LinkUserReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_resident_management_scope(deps, &ctx, &claims, ResidentId::new(id)).await?;
    let actor = actor_of(&claims);
    let uid = UserId::new(
        Uuid::parse_str(&req.user_id)
            .map_err(|_| AppError::Validation("invalid user_id".into()))?,
    );
    services::link_resident_user(
        deps.resident_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        ResidentId::new(id),
        uid,
    )
    .await?;
    Ok(Json(json!({ "status": "linked" })))
}
