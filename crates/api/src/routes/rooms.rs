use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{
    actor_of, change_resp, require_branch_management_scope, require_room_management_scope,
    resolve_ctx, ChangeResp,
};
use axum::extract::{Path, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use pg_application::ports::repositories::ChangeTarget;
use pg_application::services;
use pg_application::services::MutationOutcome;
use pg_application::AppError;
use pg_domain::{BranchId, FloorId, Role, RoomId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/branches/{branch_id}/rooms", get(list).post(create))
        .route("/rooms/{id}", patch(update).delete(remove))
}

#[derive(Deserialize)]
struct CreateRoomReq {
    number: String,
    #[serde(default = "default_cooling_type")]
    cooling_type: String,
    /// Optional floor to file the room under at creation.
    #[serde(default)]
    floor_id: Option<Uuid>,
    /// How many people share the room; beds are labelled A, B, C… from this.
    #[serde(default)]
    beds: u16,
}

fn default_cooling_type() -> String {
    "non_ac".into()
}

/// A create either lands immediately (owner) or becomes a pending request
/// (manager), so the response has to be able to say which.
#[derive(Serialize)]
struct CreateResp {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    change_request_id: Option<String>,
}

#[derive(Serialize)]
struct RoomOut {
    id: String,
    branch_id: String,
    number: String,
    cooling_type: String,
    floor_id: Option<String>,
}

async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
    Json(req): Json<CreateRoomReq>,
) -> Result<Json<CreateResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_branch_management_scope(deps, &claims, BranchId::new(branch_id)).await?;
    let actor = actor_of(&claims);

    // An owner's room appears at once; a manager's waits for the owner, the
    // same rule a rename already follows.
    if claims.role == Role::Owner {
        let id = services::create_room(
            deps.room_repo.as_ref(),
            &ctx,
            claims.role,
            &actor,
            BranchId::new(branch_id),
            req.number,
            req.cooling_type,
            req.floor_id.map(FloorId::new),
            req.beds,
        )
        .await?;
        return Ok(Json(CreateResp {
            id: Some(id.to_string()),
            status: "applied",
            change_request_id: None,
        }));
    }

    let number = req.number.trim().to_string();
    if number.is_empty() {
        return Err(AppError::Validation("room number is required".into()).into());
    }
    let cooling = req.cooling_type.trim().to_ascii_lowercase();
    if !matches!(cooling.as_str(), "ac" | "non_ac") {
        return Err(AppError::Validation("cooling_type must be ac or non_ac".into()).into());
    }
    let id = Uuid::new_v4();
    let out = services::edit_entity(
        deps.cr_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        claims.user_id,
        ChangeTarget {
            entity_type: "room".into(),
            entity_id: id,
            operation: "create".into(),
            payload: json!({
                "branch_id": branch_id.to_string(),
                "number": number,
                "cooling_type": cooling,
                "floor_id": req.floor_id.map(|f| f.to_string()),
                "beds": req.beds,
            }),
        },
    )
    .await?;
    Ok(Json(match out {
        MutationOutcome::Applied => CreateResp {
            id: Some(id.to_string()),
            status: "applied",
            change_request_id: None,
        },
        MutationOutcome::Pending { change_request_id } => CreateResp {
            id: None,
            status: "pending",
            change_request_id: Some(change_request_id.to_string()),
        },
    }))
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<Vec<RoomOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_branch_management_scope(deps, &claims, BranchId::new(branch_id)).await?;
    let rooms = services::list_rooms(
        deps.room_repo.as_ref(),
        &ctx,
        claims.role,
        BranchId::new(branch_id),
    )
    .await?;
    Ok(Json(
        rooms
            .into_iter()
            .map(|r| RoomOut {
                id: r.id.to_string(),
                branch_id: r.branch_id.to_string(),
                number: r.number,
                cooling_type: r.cooling_type,
                floor_id: r.floor_id.map(|f| f.to_string()),
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct UpdateRoomReq {
    number: Option<String>,
    cooling_type: Option<String>,
    /// Present-and-null clears the floor; absent leaves it unchanged. Serde
    /// cannot tell those apart on a plain Option, so the raw value is kept.
    #[serde(default)]
    floor_id: Option<Value>,
}

async fn update(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRoomReq>,
) -> Result<Json<ChangeResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_room_management_scope(deps, &ctx, &claims, RoomId::new(id)).await?;
    let actor = actor_of(&claims);
    let mut payload = Map::new();
    if let Some(v) = req.number {
        payload.insert("number".into(), json!(v));
    }
    if let Some(v) = req.cooling_type {
        let normalized = v.trim().to_ascii_lowercase();
        if !matches!(normalized.as_str(), "ac" | "non_ac") {
            return Err(AppError::Validation("cooling_type must be ac or non_ac".into()).into());
        }
        payload.insert("cooling_type".into(), json!(normalized));
    }
    if let Some(v) = req.floor_id {
        // A room can only be filed under a floor of its own branch.
        if let Some(raw) = v.as_str() {
            let floor_id = Uuid::parse_str(raw)
                .map_err(|_| AppError::Validation("floor_id must be a uuid".into()))?;
            let floor_branch = deps
                .floor_repo
                .branch_of(&ctx, FloorId::new(floor_id))
                .await?
                .ok_or(AppError::NotFound)?;
            let room_branch = deps
                .room_repo
                .branch_of(&ctx, RoomId::new(id))
                .await?
                .ok_or(AppError::NotFound)?;
            if floor_branch.as_uuid() != room_branch.as_uuid() {
                return Err(AppError::Validation(
                    "that floor belongs to a different branch".into(),
                )
                .into());
            }
        } else if !v.is_null() {
            return Err(AppError::Validation("floor_id must be a uuid or null".into()).into());
        }
        payload.insert("floor_id".into(), v);
    }
    let target = ChangeTarget {
        entity_type: "room".into(),
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
    require_room_management_scope(deps, &ctx, &claims, RoomId::new(id)).await?;
    let actor = actor_of(&claims);
    let target = ChangeTarget {
        entity_type: "room".into(),
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
