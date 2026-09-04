//! Establishes the ambient `RequestContext` (task-local) for the whole request,
//! reusing an inbound `x-request-id` if present, and echoing it on the response.
use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use pg_application::context::{RequestContext, REQUEST_CONTEXT};
use uuid::Uuid;

const REQUEST_ID_HEADER: &str = "x-request-id";

pub async fn with_request_context(req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::new_v4);

    let ctx = RequestContext::new(request_id);

    // The rest of the request runs INSIDE the task-local scope.
    let mut res = REQUEST_CONTEXT.scope(ctx, next.run(req)).await;

    if let Ok(val) = HeaderValue::from_str(&request_id.to_string()) {
        res.headers_mut().insert(REQUEST_ID_HEADER, val);
    }
    res
}
