//! Marketplace: owners/managers PREPARE listings, the Platform Operator is
//! the publish gate, authenticated explorers read live listings only.
use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::require_branch_management_scope;
use axum::extract::{Path, Query, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use pg_application::ports::discovery::{Listing, ListingInput, ListingStatus};
use pg_application::services;
use pg_domain::BranchId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Owner/manager/operator routes. Every handler here takes `AuthUser`.
pub fn router() -> Router<AppState> {
    Router::new()
        // Owner/manager prep
        .route("/listings/{branch_id}", put(prepare))
        .route("/listings/{branch_id}/submit", axum::routing::post(submit))
        .route("/listings", get(my_listings))
        // Authenticated platform-wide Explore surface. It intentionally does
        // not use the caller's org context, but it does require a session.
        .route("/discover/listings", get(list))
        .route("/discover/search", get(search))
        .route("/discover/nearby", get(nearby))
        .route("/discover/listings/{branch_id}", get(get_one))
        // Platform operator gates
        .route("/operator/listings", get(operator_queue))
        .route(
            "/operator/listings/{branch_id}/publish",
            axum::routing::post(operator_publish),
        )
        .route(
            "/operator/listings/{branch_id}/reject",
            axum::routing::post(operator_reject),
        )
        .route(
            "/operator/listings/{branch_id}/unlist",
            axum::routing::post(operator_unlist),
        )
}

#[derive(Deserialize)]
struct PrepareReq {
    title: String,
    area: Option<String>,
    city: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    description: Option<String>,
    amenities: Option<String>,
}

/// Owner/manager saves a listing's content as a draft.
async fn prepare(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
    Json(req): Json<PrepareReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    let ctx = {
        let org = claims.org_id;
        deps.resolver
            .resolve(org)
            .await
            .map_err(|e| pg_application::AppError::Tenancy(e.to_string()))?
    };
    let known = deps.branch_repo.list(&ctx).await?;
    if !known.iter().any(|b| b.id == branch_id) {
        return Err(pg_application::AppError::NotFound.into());
    }
    require_branch_management_scope(deps, &claims, BranchId::new(branch_id)).await?;
    services::prepare_listing(
        deps.listing_repo.as_ref(),
        claims.role,
        claims.org_id,
        BranchId::new(branch_id),
        ListingInput {
            title: req.title,
            area: req.area,
            city: req.city,
            lat: req.lat,
            lng: req.lng,
            description: req.description,
            amenities: req.amenities,
        },
    )
    .await?;
    Ok(Json(json!({ "status": "draft" })))
}

/// "Send for approval" — a draft/rejected listing goes to the operator queue.
async fn submit(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    services::submit_listing(
        deps.listing_repo.as_ref(),
        claims.role,
        BranchId::new(branch_id),
    )
    .await?;
    Ok(Json(json!({ "status": "pending" })))
}

#[derive(Serialize)]
struct ListingOut {
    branch_id: String,
    title: String,
    area: Option<String>,
    city: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    description: Option<String>,
    amenities: Option<String>,
    status: String,
    beds_available: Option<i32>,
    starting_price_paise: Option<i64>,
    distance_km: Option<f64>,
}

fn out(l: Listing) -> ListingOut {
    ListingOut {
        branch_id: l.branch_id.to_string(),
        title: l.title,
        area: l.area,
        city: l.city,
        lat: l.lat,
        lng: l.lng,
        description: l.description,
        amenities: l.amenities,
        status: l.status.as_str().to_string(),
        beds_available: l.beds_available,
        starting_price_paise: l.starting_price_paise,
        distance_km: l.distance_km,
    }
}

/// Owner/manager: their org's listings with status (draft/pending/live/...).
async fn my_listings(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<ListingOut>>, ApiError> {
    let deps = state.deps()?;
    let items =
        services::list_org_listings(deps.listing_repo.as_ref(), claims.role, claims.org_id).await?;
    Ok(Json(items.into_iter().map(out).collect()))
}

/// Platform operator: the pending-approval queue across every org.
async fn operator_queue(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<ListingOut>>, ApiError> {
    let deps = state.deps()?;
    let items = services::operator_queue(deps.listing_repo.as_ref(), claims.role).await?;
    Ok(Json(items.into_iter().map(out).collect()))
}

async fn operator_publish(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    services::operator_publish(
        deps.listing_repo.as_ref(),
        claims.role,
        BranchId::new(branch_id),
    )
    .await?;
    Ok(Json(json!({ "status": "live" })))
}

#[derive(Deserialize)]
struct RejectReq {
    note: String,
}

async fn operator_reject(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
    Json(req): Json<RejectReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    services::operator_reject(
        deps.listing_repo.as_ref(),
        claims.role,
        BranchId::new(branch_id),
        &req.note,
    )
    .await?;
    Ok(Json(json!({ "status": "rejected" })))
}

async fn operator_unlist(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deps = state.deps()?;
    services::operator_unlist(
        deps.listing_repo.as_ref(),
        claims.role,
        BranchId::new(branch_id),
    )
    .await?;
    Ok(Json(json!({ "status": "unlisted" })))
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<i64>,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<ListingOut>>, ApiError> {
    let deps = state.deps()?;
    let items = services::discover_list(
        deps.listing_repo.as_ref(),
        q.limit.unwrap_or(50).clamp(1, 200),
    )
    .await?;
    Ok(Json(items.into_iter().map(out).collect()))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    city: Option<String>,
    limit: Option<i64>,
}

async fn search(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Query(sq): Query<SearchQuery>,
) -> Result<Json<Vec<ListingOut>>, ApiError> {
    let deps = state.deps()?;
    let items = services::discover_search(
        deps.listing_repo.as_ref(),
        sq.q,
        sq.city,
        sq.limit.unwrap_or(50).clamp(1, 200),
    )
    .await?;
    Ok(Json(items.into_iter().map(out).collect()))
}

#[derive(Deserialize)]
struct NearbyQuery {
    lat: f64,
    lng: f64,
    limit: Option<i64>,
}

async fn nearby(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Query(nq): Query<NearbyQuery>,
) -> Result<Json<Vec<ListingOut>>, ApiError> {
    let deps = state.deps()?;
    let items = services::discover_nearby(
        deps.listing_repo.as_ref(),
        nq.lat,
        nq.lng,
        nq.limit.unwrap_or(20).clamp(1, 50),
    )
    .await?;
    Ok(Json(items.into_iter().map(out).collect()))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(branch_id): Path<Uuid>,
) -> Result<Json<ListingOut>, ApiError> {
    let deps = state.deps()?;
    let l = services::discover_get(deps.listing_repo.as_ref(), branch_id)
        .await?
        .filter(|l| l.status == ListingStatus::Live)
        .ok_or(pg_application::AppError::NotFound)?;
    Ok(Json(out(l)))
}
