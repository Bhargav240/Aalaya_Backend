use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::DeviceTokenRepository;
use pg_application::AppError;
use pg_domain::{BranchId, OrgId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgDeviceTokenRepository {
    pool: PgPool,
}

impl PgDeviceTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeviceTokenRepository for PgDeviceTokenRepository {
    async fn register(
        &self,
        id: Uuid,
        user_id: UserId,
        org_id: OrgId,
        token: &str,
        platform: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.device_tokens (id, user_id, org_id, token, platform, updated_at) \
             VALUES ($1,$2,$3,$4,$5, now()) \
             ON CONFLICT (user_id, token) DO UPDATE SET updated_at=now(), platform=EXCLUDED.platform",
        )
        .bind(id)
        .bind(user_id.as_uuid())
        .bind(org_id.as_uuid())
        .bind(token)
        .bind(platform)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn unregister(&self, user_id: UserId, token: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM public.device_tokens WHERE user_id=$1 AND token=$2")
            .bind(user_id.as_uuid())
            .bind(token)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn tokens_for_recipients(
        &self,
        org_id: OrgId,
        branch_id: Option<BranchId>,
    ) -> Result<Vec<String>, AppError> {
        let rows: Vec<(String,)> = match branch_id {
            Some(b) => sqlx::query_as(
                "SELECT t.token FROM public.device_tokens t \
                 JOIN public.user_org_roles r ON r.user_id = t.user_id AND r.org_id = t.org_id \
                 WHERE t.org_id = $1 AND r.branch_id = $2",
            )
            .bind(org_id.as_uuid())
            .bind(b.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?,
            None => sqlx::query_as("SELECT token FROM public.device_tokens WHERE org_id = $1")
                .bind(org_id.as_uuid())
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx)?,
        };
        Ok(rows.into_iter().map(|(t,)| t).collect())
    }

    async fn tokens_for_user(&self, user_id: UserId) -> Result<Vec<String>, AppError> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT token FROM public.device_tokens WHERE user_id = $1")
                .bind(user_id.as_uuid())
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(t,)| t).collect())
    }
}
