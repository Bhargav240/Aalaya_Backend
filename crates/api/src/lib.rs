//! HTTP layer (Axum). Owns request/response DTOs, middleware, and the single
//! error boundary. Depends on `application`, never on `infrastructure`.
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(dead_code)]

pub mod error;
pub mod extractors;
pub mod middleware;
pub mod routes;
pub mod state;
pub mod support;

pub use error::ApiError;
pub use state::{AppState, Deps};

use axum::middleware::{from_fn, from_fn_with_state};
use axum::Router;

/// Build the router. Middleware order matters: the context layer is outermost
/// so the logging layer (and every handler) can read the request id.
///
/// Explore is an authenticated app feature. It deliberately reads live
/// listings across organizations, but only after `AuthUser` verifies a session.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(routes::health::router())
        .nest("/api/v1", routes::api_v1())
        .layer(from_fn_with_state(
            state.clone(),
            middleware::logging::log_requests,
        ))
        .layer(from_fn(middleware::context::with_request_context))
        .with_state(state)
}
