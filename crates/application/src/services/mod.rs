//! Application use cases. Each module exposes plain functions that receive
//! only the ports it needs, keeping HTTP and infrastructure concerns outside
//! business logic.

pub mod archive;
pub mod auth;
pub mod away;
pub mod complaints;
pub mod core;
pub mod discovery;
pub mod enquiries;
pub mod finance;
pub mod invitations;
pub mod layout;
pub mod money;
pub mod provisioning;
pub mod tenant_kyc;
pub mod vacancy;
pub mod verification;

pub use archive::*;
pub use auth::*;
pub use away::*;
pub use complaints::*;
pub use core::*;
pub use discovery::*;
pub use enquiries::*;
pub use finance::*;
pub use invitations::*;
pub use layout::*;
pub use money::*;
pub use provisioning::*;
pub use tenant_kyc::*;
pub use vacancy::*;
pub use verification::*;
