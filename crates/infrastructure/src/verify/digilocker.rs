use async_trait::async_trait;
use pg_application::ports::verification::{
    DigiLockerClient, DigiLockerIdentity, IdentityVerifier, VerificationOutcome,
    VerificationRequest, VerificationStatus,
};
use pg_application::AppError;
use serde::Deserialize;

/// Compatibility adapter for the legacy synchronous identity endpoint. New
/// tenant onboarding uses `HttpDigiLockerClient` and the OAuth callback flow.
pub struct DigiLockerVerifier;

#[async_trait]
impl IdentityVerifier for DigiLockerVerifier {
    async fn verify(&self, _req: &VerificationRequest) -> Result<VerificationOutcome, AppError> {
        Ok(VerificationOutcome {
            status: VerificationStatus::Failed,
            verified_name: None,
            reason: Some("use invitation-scoped DigiLocker onboarding".to_string()),
        })
    }
}

pub struct HttpDigiLockerClient {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    scopes: String,
    client: reqwest::Client,
}

impl HttpDigiLockerClient {
    pub fn new(config: DigiLockerClientConfig) -> Self {
        Self {
            client_id: config.client_id,
            client_secret: config.client_secret,
            redirect_uri: config.redirect_uri,
            authorization_endpoint: config.authorization_endpoint,
            token_endpoint: config.token_endpoint,
            userinfo_endpoint: config.userinfo_endpoint,
            scopes: config.scopes,
            client: reqwest::Client::new(),
        }
    }
}

pub struct DigiLockerClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub scopes: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
}

#[derive(Deserialize)]
struct UserInfoResponse {
    sub: String,
    nonce: String,
    document_type: Option<String>,
}

#[async_trait]
impl DigiLockerClient for HttpDigiLockerClient {
    fn authorization_url(
        &self,
        state: &str,
        nonce: &str,
        code_challenge: &str,
    ) -> Result<String, AppError> {
        Ok(format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",
            self.authorization_endpoint,
            encode(&self.client_id),
            encode(&self.redirect_uri),
            encode(&self.scopes),
            encode(state),
            encode(nonce),
            encode(code_challenge),
        ))
    }

    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<DigiLockerIdentity, AppError> {
        let response = self
            .client
            .post(&self.token_endpoint)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", self.redirect_uri.as_str()),
                ("code_verifier", pkce_verifier),
            ])
            .send()
            .await
            .map_err(|_| AppError::Internal("identity provider unavailable".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Unauthorized);
        }
        let token: TokenResponse = response.json().await.map_err(|_| AppError::Unauthorized)?;
        if !token.token_type.eq_ignore_ascii_case("bearer") || token.access_token.is_empty() {
            return Err(AppError::Unauthorized);
        }

        let response = self
            .client
            .get(&self.userinfo_endpoint)
            .bearer_auth(&token.access_token)
            .send()
            .await
            .map_err(|_| AppError::Internal("identity provider unavailable".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Unauthorized);
        }
        let identity: UserInfoResponse =
            response.json().await.map_err(|_| AppError::Unauthorized)?;
        if identity.sub.trim().is_empty() || identity.nonce.trim().is_empty() {
            return Err(AppError::Unauthorized);
        }
        Ok(DigiLockerIdentity {
            provider_reference: identity.sub,
            document_type: identity.document_type,
            nonce: identity.nonce,
        })
    }
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                vec![char::from(byte)]
            } else {
                format!("%{byte:02X}").chars().collect()
            }
        })
        .collect()
}
