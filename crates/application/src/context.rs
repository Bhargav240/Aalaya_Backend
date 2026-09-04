//! Ambient, per-request context. Set ONCE by middleware and read anywhere in
//! the async task -- so cross-cutting values (request id, actor, org) never
//! have to be threaded through every function signature.
//!
//! This is a `tokio::task_local!`, not a thread-local: Tokio moves tasks across
//! worker threads at every `.await`, so a thread-local could read back another
//! task's value. A task-local is bound to the task and follows it.
//!
//! GUARDRAIL: this is only for ambient cross-cutting data. Real dependencies
//! (repositories, config, storage) stay explicitly injected through their
//! traits, so the code remains testable.
use pg_domain::{OrgId, Role, UserId};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Actor {
    pub user_id: UserId,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: Uuid,
    pub actor: Option<Actor>,
    pub org_id: Option<OrgId>,
}

impl RequestContext {
    pub fn new(request_id: Uuid) -> Self {
        Self {
            request_id,
            actor: None,
            org_id: None,
        }
    }
}

tokio::task_local! {
    pub static REQUEST_CONTEXT: RequestContext;
}

/// The current request id, if a context is in scope.
pub fn current_request_id() -> Option<Uuid> {
    REQUEST_CONTEXT.try_with(|c| c.request_id).ok()
}
