use argon2::password_hash::rand_core::{OsRng, RngCore};
use pg_application::ports::otp::OtpCodeGenerator;

/// Sources randomness the same way `Argon2Hasher` sources its salt, so no new
/// `rand` dependency is needed.
pub struct RandomOtpCodeGenerator;

impl OtpCodeGenerator for RandomOtpCodeGenerator {
    fn generate(&self) -> String {
        let mut buf = [0u8; 4];
        OsRng.fill_bytes(&mut buf);
        let n = u32::from_le_bytes(buf) % 1_000_000;
        format!("{n:06}")
    }
}
