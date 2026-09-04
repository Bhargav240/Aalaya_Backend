use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{
    actor_of, require_branch_management_scope, require_room_management_scope, resolve_ctx,
};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use pg_application::ports::repositories::ChangeTarget;
use pg_application::services;
use pg_application::services::MutationOutcome;
use pg_application::AppError;
use pg_domain::{BranchId, Role, RoomId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/rooms/{room_id}/beds", get(list).post(create))
        // Every bed in a branch in one call — the payments screen's room->bed
        // fan-out collapses into a single fetch.
        .route("/branches/{branch_id}/beds", get(list_by_branch))
}

#[derive(Deserialize)]
struct CreateBedReq {
    label: String,
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
struct IdResp {
    id: String,
}

#[derive(Serialize)]
struct BedOut {
    id: String,
    room_id: String,
    label: String,
    occupant_resident_id: Option<String>,
    occupant_name: Option<String>,
}

async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(room_id): Path<Uuid>,
    Json(req): Json<CreateBedReq>,
) -> Result<Json<CreateResp>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_room_management_scope(deps, &ctx, &claims, RoomId::new(room_id)).await?;
    let actor = actor_of(&claims);

    // Owner adds a bed outright; a manager's addition waits for approval.
    if claims.role == Role::Owner {
        let id = services::create_bed(
            deps.bed_repo.as_ref(),
            &ctx,
            claims.role,
            &actor,
            RoomId::new(room_id),
            req.label,
        )
        .await?;
        return Ok(Json(CreateResp {
            id: Some(id.to_string()),
            status: "applied",
            change_request_id: None,
        }));
    }

    let label = req.label.trim().to_string();
    if label.is_empty() {
        return Err(AppError::Validation("bed label is required".into()).into());
    }
    let id = Uuid::new_v4();
    let out = services::edit_entity(
        deps.cr_repo.as_ref(),
        &ctx,
        claims.role,
        &actor,
        claims.user_id,
        ChangeTarget {
            entity_type: "bed".into(),
            entity_id: id,
            operation: "create".into(),
            payload: json!({ "room_id": room_id.to_string(), "label": label }),
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

async fn list_by_branch(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<Vec<BedOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_branch_management_scope(deps, &claims, BranchId::new(branch_id)).await?;
    let beds = services::list_beds_by_branch(
        deps.bed_repo.as_ref(),
        &ctx,
        claims.role,
        BranchId::new(branch_id),
    )
    .await?;
    Ok(Json(
        beds.into_iter()
            .map(|b| BedOut {
                id: b.id.to_string(),
                room_id: b.room_id.to_string(),
                label: b.label,
                occupant_resident_id: b.occupant_resident_id.map(|id| id.to_string()),
                occupant_name: b.occupant_name,
            })
            .collect(),
    ))
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(room_id): Path<Uuid>,
) -> Result<Json<Vec<BedOut>>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    require_room_management_scope(deps, &ctx, &claims, RoomId::new(room_id)).await?;
    let beds = services::list_beds(
        deps.bed_repo.as_ref(),
        &ctx,
        claims.role,
        RoomId::new(room_id),
    )
    .await?;
    Ok(Json(
        beds.into_iter()
            .map(|b| BedOut {
                id: b.id.to_string(),
                room_id: b.room_id.to_string(),
                label: b.label,
                occupant_resident_id: b.occupant_resident_id.map(|id| id.to_string()),
                occupant_name: b.occupant_name,
            })
            .collect(),
    ))
}
