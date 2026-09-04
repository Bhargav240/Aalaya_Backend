//! Google sign-in: verifies Google identity tokens (RS256) against Google's
//! public JWKS, keeping account claims on our side of the fence.
use async_trait::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use pg_application::ports::{GoogleVerifier, VerifiedGoogleUser};
use pg_application::AppError;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
/// Our Android + any web client ids accepted as `aud`.
const JWKS_TTL: Duration = Duration::from_secs(6 * 60 * 60); // 6h, keys rotate daily-ish

#[derive(Debug, Deserialize)]
struct GoogleClaims {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Deserialize, Clone)]
struct Jwk {
    kty: String,
    kid: String,
    n: String,
    e: String,
}

/// Fetches and verifies against Google's JWKS, caching the key set in memory.
pub struct HttpGoogleVerifier {
    client_id: String,
    http: reqwest::Client,
    cache: Mutex<Option<(Instant, HashMap<String, DecodingKey>)>>,
}

impl HttpGoogleVerifier {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            http: reqwest::Client::new(),
            cache: Mutex::new(None),
        }
    }

    async fn jwks(&self) -> Result<HashMap<String, DecodingKey>, AppError> {
        {
            let cache = self.cache.lock().expect("poisoned");
            if let Some((fetched_at, keys)) = &*cache {
                if fetched_at.elapsed() < JWKS_TTL {
                    return Ok(keys.clone());
                }
            }
        }
        let jwks: Jwks = self
            .http
            .get(GOOGLE_JWKS_URL)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("google jwks fetch: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Internal(format!("google jwks status: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("google jwks parse: {e}")))?;

        let mut map = HashMap::new();
        for j in jwks.keys {
            if j.kty != "RSA" {
                continue;
            }
            let key = DecodingKey::from_rsa_components(&j.n, &j.e)
                .map_err(|e| AppError::Internal(format!("google jwk: {e}")))?;
            map.insert(j.kid, key);
        }
        *self.cache.lock().expect("poisoned") = Some((Instant::now(), map.clone()));
        Ok(map)
    }
}

/// Returned when the app is not configured with GOOGLE_CLIENT_ID. Always 401.
pub struct NullGoogleVerifier;

#[async_trait]
impl GoogleVerifier for NullGoogleVerifier {
    async fn verify(&self, _id_token: &str) -> Result<VerifiedGoogleUser, AppError> {
        Err(AppError::Unauthorized)
    }
}

#[async_trait]
impl GoogleVerifier for HttpGoogleVerifier {
    async fn verify(&self, id_token: &str) -> Result<VerifiedGoogleUser, AppError> {
        let header = jsonwebtoken::decode_header(id_token).map_err(|_| AppError::Unauthorized)?;
        let kid = header.kid.ok_or(AppError::Unauthorized)?;
        let keys = self.jwks().await?;
        let key = keys.get(&kid).ok_or(AppError::Unauthorized)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[self.client_id.as_str()]);
        validation.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);
        validation.validate_exp = true;

        let data = jsonwebtoken::decode::<GoogleClaims>(id_token, key, &validation)
            .map_err(|_| AppError::Unauthorized)?;

        Ok(VerifiedGoogleUser {
            sub: data.claims.sub,
            email: data.claims.email,
            email_verified: data.claims.email_verified.unwrap_or(false),
            name: data.claims.name,
        })
    }
}
