use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::TrustedDeviceRepository;
use pg_application::AppError;
use pg_domain::UserId;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgTrustedDeviceRepository {
    pool: PgPool,
}

impl PgTrustedDeviceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TrustedDeviceRepository for PgTrustedDeviceRepository {
    async fn is_trusted(&self, user_id: UserId, device_id: &str) -> Result<bool, AppError> {
        let row: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT id FROM public.trusted_devices WHERE user_id=$1 AND device_id=$2",
        )
        .bind(user_id.as_uuid())
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.is_some())
    }

    async fn trust(
        &self,
        id: Uuid,
        user_id: UserId,
        device_id: &str,
        name: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.trusted_devices (id, user_id, device_id, name) VALUES ($1,$2,$3,$4) \
             ON CONFLICT (user_id, device_id) DO UPDATE SET last_seen_at=now(), name=COALESCE(EXCLUDED.name, trusted_devices.name)",
        )
        .bind(id)
        .bind(user_id.as_uuid())
        .bind(device_id)
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}
