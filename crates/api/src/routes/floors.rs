use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{
    actor_of, change_resp, require_branch_management_scope, resolve_ctx, ChangeResp,
};
use axum::extract::{Path, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use pg_application::authz;
use pg_application::ports::repositories::{ChangeTarget, NewRoomSpec};
use pg_application::services;
use pg_application::services::MutationOutcome;
use pg_application::AppError;
use pg_domain::{BranchId, FloorId, Role};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/branches/{branch_id}/floors", get(list).post(create))
        .route("/floors/{id}", patch(update).delete(remove))
}

#[derive(Deserialize)]
struct FloorReq {
    name: String,
    #[serde(default)]
    sort_order: i32,
    /// Rooms to create with the floor. Empty means a bare floor, which is
    /// still valid — rooms can be added to it later.
    #[serde(default)]
    rooms: Vec<NewRoomReq>,
}

#[derive(Deserialize, Clone)]
struct NewRoomReq {
    number: String,
    #[serde(default = "default_cooling")]
    cooling_type: String,
    /// How many people share the room; beds are labelled A, B, C… from this.
    #[serde(default)]
    beds: u16,
}

fn default_cooling() -> String {
    "non_ac".into()
}

/// Validates the rooms attached to an add-floor request.
///
/// Room numbers must be unique *within the request* as well as in the branch —
/// the database catches the second case, but two identical numbers typed into
/// the same form would otherwise fail deep in a transaction with a confusing
/// duplicate-key error instead of a clear one here.
fn clean_rooms(rooms: &[NewRoomReq]) -> Result<Vec<NewRoomSpec>, AppError> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(rooms.len());
    for room in rooms {
        let number = room.number.trim();
        if number.is_empty() {
            return Err(AppError::Validation("every room needs a number".into()));
        }
        if number.chars().count() > 20 {
            return Err(AppError::Validation(format!(
                "room number {number} is too long"
            )));
        }
        if !seen.insert(number.to_ascii_lowercase()) {
            return Err(AppError::Validation(format!(
                "room {number} is listed twice"
            )));
        }
        let cooling = room.cooling_type.trim().to_ascii_lowercase();
        if !matches!(cooling.as_str(), "ac" | "non_ac") {
            return Err(AppError::Validation(
                "cooling_type must be ac or non_ac".into(),
            ));
        }
        if room.beds > 26 {
            return Err(AppError::Validation(
                "a room cannot have more than 26 beds".into(),
            ));
        }
        out.push(NewRoomSpec {
            number: number.to_string(),
            cooling_type: cooling,
            beds: room.beds,
        });
    }
    Ok(out)
}

#[derive(Serialize)]
struct FloorOut {
    id: String,
    branch_id: String,
    name: String,
    sort_order: i32,
    room_count: i64,
}

fn clean_name(raw: &str) -> Result<String, AppError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(AppError::Validation("floor name is required".into()));
    }
    if name.chars().count() > 60 {
        return Err(AppError::Validation("floor name is too long".into()));
    }
    Ok(name.to_string())
}

async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
    Json(req): Json<FloorReq>,
) -> Result<Json<ChangeResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    if !authz::can_manage(claims.role) {
        return Err(AppError::Forbidden.into());
    }
    require_branch_management_scope(deps, &claims, BranchId::new(branch_id)).await?;
    let name = clean_name(&req.name)?;
    let rooms = clean_rooms(&req.rooms)?;
    let actor = actor_of(&claims);
    // The id is allocated here, before the request is raised, so an approved
    // floor is the same floor the pending request referred to.
    let id = FloorId::new(Uuid::new_v4());

    if claims.role == Role::Owner {
        deps.floor_repo
            .create(
                &ctx,
                id,
                BranchId::new(branch_id),
                &name,
                req.sort_order,
                &rooms,
                &actor,
            )
            .await?;
        return Ok(Json(change_resp(MutationOutcome::Applied)));
    }

    let out = services::edit_entity(
        deps.cr_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        claims.user_id,
        ChangeTarget {
            entity_type: "floor".into(),
            entity_id: id.as_uuid(),
            operation: "create".into(),
            // The rooms travel with the request, so approving the floor
            // creates them too — one decision, not one per room.
            payload: json!({
                "branch_id": branch_id.to_string(),
                "name": name,
                "sort_order": req.sort_order,
                "rooms": rooms.iter().map(|r| json!({
                    "number": r.number,
                    "cooling_type": r.cooling_type,
                    "beds": r.beds,
                })).collect::<Vec<_>>(),
            }),
        },
    )
    .await?;
    Ok(Json(change_resp(out)))
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<Vec<FloorOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_branch_management_scope(deps, &claims, BranchId::new(branch_id)).await?;
    let floors = deps
        .floor_repo
        .list_by_branch(&ctx, BranchId::new(branch_id))
        .await?;
    Ok(Json(
        floors
            .into_iter()
            .map(|f| FloorOut {
                id: f.id.to_string(),
                branch_id: f.branch_id.to_string(),
                name: f.name,
                sort_order: f.sort_order,
                room_count: f.room_count,
            })
            .collect(),
    ))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<FloorReq>,
) -> Result<Json<ChangeResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    if !authz::can_manage(claims.role) {
        return Err(AppError::Forbidden.into());
    }
    let floor_id = FloorId::new(id);
    let branch = deps
        .floor_repo
        .branch_of(&ctx, floor_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_branch_management_scope(deps, &claims, branch).await?;
    let name = clean_name(&req.name)?;
    let actor = actor_of(&claims);

    if claims.role == Role::Owner {
        deps.floor_repo
            .rename(&ctx, floor_id, &name, req.sort_order, &actor)
            .await?;
        return Ok(Json(change_resp(MutationOutcome::Applied)));
    }

    let out = services::edit_entity(
        deps.cr_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        claims.user_id,
        ChangeTarget {
            entity_type: "floor".into(),
            entity_id: id,
            operation: "update".into(),
            payload: json!({ "name": name, "sort_order": req.sort_order }),
        },
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
    if !authz::can_manage(claims.role) {
        return Err(AppError::Forbidden.into());
    }
    let floor_id = FloorId::new(id);
    let branch = deps
        .floor_repo
        .branch_of(&ctx, floor_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_branch_management_scope(deps, &claims, branch).await?;
    let actor = actor_of(&claims);

    if claims.role == Role::Owner {
        deps.floor_repo.delete(&ctx, floor_id, &actor).await?;
        return Ok(Json(change_resp(MutationOutcome::Applied)));
    }

    let out = services::edit_entity(
        deps.cr_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        claims.user_id,
        ChangeTarget {
            entity_type: "floor".into(),
            entity_id: id,
            operation: "delete".into(),
            payload: Value::Null,
        },
    )
    .await?;
    Ok(Json(change_resp(out)))
}
