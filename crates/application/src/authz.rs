//! Authorization policy helpers. The PRD's who-can-do-what matrix lives here.
use pg_domain::Role;

/// Owners and managers can run a branch's daily work.
pub fn can_manage(role: Role) -> bool {
    matches!(role, Role::Owner | Role::Manager)
}

pub fn is_owner(role: Role) -> bool {
    role == Role::Owner
}

pub fn is_operator(role: Role) -> bool {
    role == Role::PlatformOperator
}

/// The platform operator is the gate on public listings.
pub fn can_publish_listings(role: Role) -> bool {
    role == Role::PlatformOperator
}

/// Owners and managers may PREPARE a listing (never publish it).
pub fn can_prepare_listing(role: Role) -> bool {
    matches!(role, Role::Owner | Role::Manager)
}
