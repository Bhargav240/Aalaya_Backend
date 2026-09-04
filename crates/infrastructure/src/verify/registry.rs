//! Provider registry. Dispatches to the verifier for `req.method`, falling back
//! to the default. THIS is the worldwide-extensibility hook: register more
//! adapters (passport, other national IDs, KYC vendors) keyed by method/region.
use async_trait::async_trait;
use pg_application::ports::verification::{
    IdentityVerifier, VerificationOutcome, VerificationRequest,
};
use pg_application::AppError;
use std::collections::HashMap;
use std::sync::Arc;

pub struct CompositeVerifier {
    providers: HashMap<String, Arc<dyn IdentityVerifier>>,
    default_method: String,
}

impl CompositeVerifier {
    pub fn new(default_method: impl Into<String>) -> Self {
        Self {
            providers: HashMap::new(),
            default_method: default_method.into(),
        }
    }

    pub fn register(mut self, method: impl Into<String>, v: Arc<dyn IdentityVerifier>) -> Self {
        self.providers.insert(method.into(), v);
        self
    }
}

#[async_trait]
impl IdentityVerifier for CompositeVerifier {
    async fn verify(&self, req: &VerificationRequest) -> Result<VerificationOutcome, AppError> {
        let method = if req.method.is_empty() {
            &self.default_method
        } else {
            &req.method
        };
        let verifier = self
            .providers
            .get(method)
            .or_else(|| self.providers.get(&self.default_method))
            .ok_or_else(|| AppError::Internal("no identity verifier registered".into()))?;
        verifier.verify(req).await
    }
}
