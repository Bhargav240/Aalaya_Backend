use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::RefreshTokenRepository;
use pg_application::AppError;
use pg_domain::{OrgId, UserId};
use sqlx::PgPool;

/// Control-plane refresh-token store (hashes only). Enables rotation on every
/// refresh and revocation on logout.
pub struct PgRefreshTokenRepository {
    pool: PgPool,
}

impl PgRefreshTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefreshTokenRepository for PgRefreshTokenRepository {
    async fn store(
        &self,
        token_hash: &str,
        user_id: UserId,
        org_id: OrgId,
        ttl_secs: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.refresh_tokens (token_hash, user_id, org_id, expires_at) \
             VALUES ($1, $2, $3, now() + make_interval(secs => $4::double precision))",
        )
        .bind(token_hash)
        .bind(user_id.as_uuid())
        .bind(org_id.as_uuid())
        .bind(ttl_secs)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn consume(&self, token_hash: &str) -> Result<bool, AppError> {
        let res = sqlx::query(
            "UPDATE public.refresh_tokens SET revoked_at = now() \
             WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > now()",
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(res.rows_affected() == 1)
    }

    async fn revoke_all(&self, user_id: UserId) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE public.refresh_tokens SET revoked_at = now() \
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}

impl PgRefreshTokenRepository {
    /// Periodic sweep of expired/revoked refresh tokens (B18: append-only).
    pub async fn delete_expired(&self) -> Result<u64, AppError> {
        let res = sqlx::query(
            "DELETE FROM public.refresh_tokens \
             WHERE expires_at < now() - interval '7 days' \
                OR (revoked_at IS NOT NULL AND revoked_at < now() - interval '7 days')",
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(res.rows_affected())
    }
}
