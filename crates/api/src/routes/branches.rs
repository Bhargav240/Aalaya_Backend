use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{
    actor_of, change_resp, require_branch_management_scope, resolve_ctx, ChangeResp,
};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use pg_application::ports::repositories::ChangeTarget;
use pg_application::services;
use pg_domain::Role;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/branches", get(list).post(create))
        .route(
            "/branches/with-layout",
            axum::routing::post(create_with_layout),
        )
        .route(
            "/branches/{id}",
            axum::routing::patch(update).delete(remove),
        )
}

#[derive(Serialize)]
struct BranchOut {
    id: String,
    name: String,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<BranchOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    // Branches expose operational structure, so the route is staff-only even
    // if a client manually calls it outside the role-aware Android navigation.
    // Managers see their assigned branch only; owners retain the existing
    // organization-wide list.
    let items = match claims.role {
        Role::Owner => deps.branch_repo.list(&ctx).await?,
        Role::Manager => {
            let assigned = deps
                .user_repo
                .branch_in_org(claims.user_id, claims.org_id)
                .await?
                .map(|branch| branch.as_uuid());
            deps.branch_repo
                .list(&ctx)
                .await?
                .into_iter()
                .filter(|branch| Some(branch.id) == assigned)
                .collect()
        }
        Role::Tenant | Role::PlatformOperator | Role::Prospect => {
            return Err(pg_application::AppError::Forbidden.into())
        }
    };
    Ok(Json(
        items
            .into_iter()
            .map(|b| BranchOut {
                id: b.id.to_string(),
                name: b.name,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct CreateBranchReq {
    name: String,
}

#[derive(Deserialize)]
struct CreateBranchWithLayoutReq {
    name: String,
    floor_count: u16,
    rooms_per_floor: u16,
    beds_per_room: u16,
}

#[derive(Serialize)]
struct CreateBranchWithLayoutResp {
    status: &'static str,
    id: String,
    name: String,
}

async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateBranchReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if claims.role == Role::Owner || claims.role == Role::Manager {
        return Err(pg_application::AppError::Forbidden.into());
    }
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let actor = actor_of(&claims);
    let branch_id = services::create_branch(
        deps.branch_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        req.name,
    )
    .await?;
    if let Some(layout) = &deps.onboarding_layout {
        services::reconcile_onboarding_layout(
            layout,
            deps.room_repo.as_ref(),
            deps.bed_repo.as_ref(),
            deps.floor_repo.as_ref(),
            &ctx,
            branch_id,
            &actor,
        )
        .await?;
    }
    Ok(Json(json!({"status":"created"})))
}

async fn create_with_layout(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateBranchWithLayoutReq>,
) -> Result<Json<CreateBranchWithLayoutResp>, ApiError> {
    if claims.role == Role::Owner || claims.role == Role::Manager {
        return Err(pg_application::AppError::Forbidden.into());
    }
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    let actor = actor_of(&claims);
    let name = req.name.trim().to_string();
    let branch_id = services::create_branch_with_layout(
        deps.branch_repo.as_ref(),
        deps.floor_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        name.clone(),
        req.floor_count,
        req.rooms_per_floor,
        req.beds_per_room,
    )
    .await?;
    Ok(Json(CreateBranchWithLayoutResp {
        status: "created",
        id: branch_id.to_string(),
        name,
    }))
}

#[derive(Deserialize)]
struct UpdateBranchReq {
    name: Option<String>,
    address: Option<String>,
}

async fn update(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBranchReq>,
) -> Result<Json<ChangeResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_branch_management_scope(deps, &claims, pg_domain::BranchId::new(id)).await?;
    let actor = actor_of(&claims);
    let mut payload = Map::new();
    if let Some(v) = req.name {
        payload.insert("name".into(), json!(v));
    }
    if let Some(v) = req.address {
        payload.insert("address".into(), json!(v));
    }
    let target = ChangeTarget {
        entity_type: "branch".into(),
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
    require_branch_management_scope(deps, &claims, pg_domain::BranchId::new(id)).await?;
    let actor = actor_of(&claims);
    let target = ChangeTarget {
        entity_type: "branch".into(),
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
