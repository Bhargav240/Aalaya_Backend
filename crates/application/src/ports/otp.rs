//! One-time-code generation port used by email authentication.

/// Generates the one-time code (infrastructure sources the randomness).
pub trait OtpCodeGenerator: Send + Sync {
    fn generate(&self) -> String;
}
