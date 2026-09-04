pub mod admin;
pub mod audit;
pub mod auth;
pub mod away;
pub mod beds;
pub mod branches;
pub mod changes;
pub mod complaints;
pub mod devices;
pub mod enquiries;
pub mod finance;
pub mod floors;
pub mod health;
pub mod listings;
pub mod me;
pub mod moveout;
pub mod notices;
pub mod orgs;
pub mod payments;
pub mod platform_admin;
pub mod receipts;
pub mod residents;
pub mod rooms;
pub mod templates;
pub mod tenant_invitations;

use crate::state::AppState;
use axum::Router;

pub fn api_v1() -> Router<AppState> {
    Router::new()
        .merge(orgs::router())
        .merge(platform_admin::router())
        .merge(auth::router())
        .merge(me::router())
        .merge(away::router())
        .merge(branches::router())
        .merge(floors::router())
        .merge(rooms::router())
        .merge(beds::router())
        .merge(residents::router())
        .merge(changes::router())
        .merge(audit::router())
        .merge(payments::router())
        .merge(receipts::router())
        .merge(moveout::router())
        .merge(templates::router())
        .merge(tenant_invitations::router())
        .merge(complaints::router())
        .merge(notices::router())
        .merge(admin::router())
        .merge(listings::router())
        .merge(enquiries::router())
        .merge(devices::router())
        .merge(finance::router())
}
