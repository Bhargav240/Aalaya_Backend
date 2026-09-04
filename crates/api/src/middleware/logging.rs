//! Logs EVERY request and its response -- success or failure -- with method,
//! path, status, latency, and request id.
//!
//! NOTE: capturing response BODIES on failure (with PII redaction of Aadhaar
//! names, phones, receipts) is deliberately deferred to the error boundary in
//! Phase 1, so we never log sensitive bodies at info level here.
use crate::state::{AppState, PlatformLog, PLATFORM_LOG_CAPACITY};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use pg_application::context::current_request_id;
use std::time::Instant;

pub async fn log_requests(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let started = Instant::now();

    let res = next.run(req).await;

    let latency_ms = started.elapsed().as_millis();
    let status = res.status().as_u16();
    let request_id = current_request_id()
        .map(|id| id.to_string())
        .unwrap_or_default();

    if status >= 500 {
        tracing::error!(%method, %uri, status, latency_ms, request_id, "request failed");
    } else if status >= 400 {
        tracing::warn!(%method, %uri, status, latency_ms, request_id, "request rejected");
    } else {
        tracing::info!(%method, %uri, status, latency_ms, request_id, "request completed");
    }

    let entry = PlatformLog {
        timestamp: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
        level: if status >= 500 {
            "ERROR"
        } else if status >= 400 {
            "WARN"
        } else {
            "INFO"
        }
        .to_string(),
        event_type: "request_completed".to_string(),
        route: uri.path().to_string(),
        method: method.to_string(),
        status_code: status,
        latency_ms,
        request_id,
    };
    let mut logs = state
        .platform_logs
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if logs.len() >= PLATFORM_LOG_CAPACITY {
        logs.pop_front();
    }
    logs.push_back(entry);

    res
}
