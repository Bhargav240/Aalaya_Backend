use crate::state::AppState;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    db_ready: bool,
    request_id: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn health(State(state): State<AppState>) -> Json<Health> {
    let request_id = pg_application::context::current_request_id().map(|id| id.to_string());
    Json(Health {
        status: "ok",
        db_ready: state.db_ready(),
        request_id,
    })
}
