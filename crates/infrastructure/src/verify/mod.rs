pub mod digilocker;
pub mod mock;
pub mod registry;

pub use digilocker::{DigiLockerClientConfig, DigiLockerVerifier, HttpDigiLockerClient};
pub use mock::MockIdentityVerifier;
pub use registry::CompositeVerifier;
