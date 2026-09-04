//! Use cases and the PORTS (traits) they depend on. Depends only on `domain`.
#![allow(clippy::too_many_arguments)]

pub mod authz;
pub mod context;
pub mod error;
pub mod ports;
pub mod services;

pub use context::{current_request_id, Actor, RequestContext, REQUEST_CONTEXT};
pub use error::AppError;
