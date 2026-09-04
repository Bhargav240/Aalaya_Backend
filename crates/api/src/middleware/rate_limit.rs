//! Fixed-window per-IP rate limiter for authentication and other abuse-prone
//! surfaces. Anonymous email delivery has a real external cost, so it is
//! protected independently from per-address database throttles. Deliberately
//! not a general-purpose crate (governor et al.): the
//! whole need is "cap requests per IP per window," and a `DashMap` gives that
//! in a dozen lines with no new dependency surface.

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use dashmap::DashMap;
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<DashMap<IpAddr, (Instant, u32)>>,
    limit: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            limit,
            window,
        }
    }

    /// True if this call is within budget for `ip`'s current window.
    fn allow(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut entry = self.buckets.entry(ip).or_insert((now, 0));
        let (window_start, count) = *entry;
        if now.duration_since(window_start) >= self.window {
            *entry = (now, 1);
            true
        } else if count < self.limit {
            entry.1 = count + 1;
            true
        } else {
            false
        }
    }
}

pub async fn enforce(
    State(limiter): State<RateLimiter>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    if limiter.allow(addr.ip()) {
        next.run(request).await
    } else {
        tracing::warn!(ip = %addr.ip(), "rate limit exceeded");
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "type": "about:blank",
                "title": "too many requests",
                "status": 429,
            })),
        )
            .into_response()
    }
}
