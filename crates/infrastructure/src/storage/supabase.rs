//! Supabase Storage adapter (S3-compatible object store) via its REST API.
//! put/get use the service-role key; signed_url mints a short-lived link so the
//! client downloads directly from Supabase instead of streaming through us.
use async_trait::async_trait;
use pg_application::ports::storage::ObjectStore;
use pg_application::AppError;
use reqwest::Client;

pub struct SupabaseStorage {
    base: String,
    service_key: String,
    bucket: String,
    client: Client,
}

impl SupabaseStorage {
    pub fn new(
        base: impl Into<String>,
        service_key: impl Into<String>,
        bucket: impl Into<String>,
    ) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_string(),
            service_key: service_key.into(),
            bucket: bucket.into(),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl ObjectStore for SupabaseStorage {
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), AppError> {
        let url = format!("{}/storage/v1/object/{}/{}", self.base, self.bucket, key);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.service_key)
            .header("x-upsert", "true")
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(bytes)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("storage put: {e}")))?;
        if !resp.status().is_success() {
            return Err(AppError::Internal(format!(
                "storage put status {}",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let url = format!(
            "{}/storage/v1/object/authenticated/{}/{}",
            self.base, self.bucket, key
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.service_key)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("storage get: {e}")))?;
        if !resp.status().is_success() {
            return Err(AppError::NotFound);
        }
        Ok(resp
            .bytes()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .to_vec())
    }

    async fn signed_url(&self, key: &str, ttl_secs: i64) -> Result<Option<String>, AppError> {
        let url = format!(
            "{}/storage/v1/object/sign/{}/{}",
            self.base, self.bucket, key
        );
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.service_key)
            .json(&serde_json::json!({ "expiresIn": ttl_secs }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("storage sign: {e}")))?;
        if !resp.status().is_success() {
            return Err(AppError::Internal(format!(
                "storage sign status {}",
                resp.status()
            )));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let signed = body
            .get("signedURL")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Internal("no signedURL in response".into()))?;
        Ok(Some(format!("{}/storage/v1{}", self.base, signed)))
    }
}
