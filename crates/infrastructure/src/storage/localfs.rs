//! Local-filesystem ObjectStore for dev. Same trait as the future Supabase/S3
//! adapters, so swapping is a wiring change only.
use async_trait::async_trait;
use pg_application::ports::storage::ObjectStore;
use pg_application::AppError;
use std::path::PathBuf;

pub struct LocalFsObjectStore {
    base: PathBuf,
}

impl LocalFsObjectStore {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }
}

#[async_trait]
impl ObjectStore for LocalFsObjectStore {
    async fn put(&self, key: &str, bytes: Vec<u8>, _content_type: &str) -> Result<(), AppError> {
        let path = self.base.join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Internal(format!("mkdir: {e}")))?;
        }
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|e| AppError::Internal(format!("write: {e}")))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let path = self.base.join(key);
        tokio::fs::read(&path).await.map_err(|_| AppError::NotFound)
    }

    async fn signed_url(&self, _key: &str, _ttl_secs: i64) -> Result<Option<String>, AppError> {
        Ok(None) // local FS can't sign -> caller streams the bytes
    }
}
