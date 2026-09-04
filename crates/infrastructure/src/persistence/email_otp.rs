use crate::persistence::errmap::map_sqlx;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use async_trait::async_trait;
use pg_application::ports::repositories::EmailOtpRepository;
use pg_application::AppError;
use pg_domain::UserId;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PgEmailOtpRepository {
    pool: PgPool,
    dummy_hash: String,
}
impl PgEmailOtpRepository {
    pub fn new(pool: PgPool) -> Self {
        let salt = SaltString::generate(&mut OsRng);
        let dummy_hash = Argon2::default()
            .hash_password(b"000000", &salt)
            .expect("static email OTP dummy hash must be constructible")
            .to_string();
        Self { pool, dummy_hash }
    }

    pub async fn delete_expired(&self) -> Result<u64, AppError> {
        sqlx::query(
            "DELETE FROM public.email_otps \
             WHERE expires_at < now() - interval '1 day' OR consumed_at < now() - interval '1 day'",
        )
        .execute(&self.pool)
        .await
        .map(|result| result.rows_affected())
        .map_err(map_sqlx)
    }
}

#[async_trait]
impl EmailOtpRepository for PgEmailOtpRepository {
    async fn create(
        &self,
        id: Uuid,
        user_id: UserId,
        email: &str,
        code: &str,
        ttl_secs: i64,
    ) -> Result<(), AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(code.as_bytes(), &salt)
            .map_err(|_| AppError::Internal("failed to hash email otp".into()))?
            .to_string();
        sqlx::query("INSERT INTO public.email_otps (id,user_id,email,code_hash,expires_at) VALUES ($1,$2,$3,$4,now()+make_interval(secs=>$5::double precision))")
            .bind(id).bind(user_id.as_uuid()).bind(email).bind(hash).bind(ttl_secs)
            .execute(&self.pool).await.map_err(map_sqlx)?;
        Ok(())
    }
    async fn verify_latest(
        &self,
        email: &str,
        user_id: UserId,
        code: &str,
        max_attempts: i32,
    ) -> Result<bool, AppError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
        let row: Option<(Uuid, Uuid, String, i32, bool, bool)> = sqlx::query_as(
            "SELECT id, user_id, code_hash, attempts, expires_at > now(), consumed_at IS NULL \
             FROM public.email_otps WHERE lower(email)=lower($1) \
             ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
        )
        .bind(email)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let encoded_hash = row
            .as_ref()
            .map(|(_, _, hash, _, _, _)| hash.as_str())
            .unwrap_or(&self.dummy_hash);
        // Always perform one Argon2 verification, including nonexistent,
        // expired, exhausted, consumed, and email-mismatch cases.
        let hash_matches = PasswordHash::new(encoded_hash)
            .map(|hash| {
                Argon2::default()
                    .verify_password(code.as_bytes(), &hash)
                    .is_ok()
            })
            .unwrap_or(false);
        let valid =
            row.as_ref()
                .is_some_and(|(_, row_user_id, _, attempts, unexpired, unconsumed)| {
                    *row_user_id == user_id.as_uuid()
                        && *attempts < max_attempts
                        && *unexpired
                        && *unconsumed
                        && hash_matches
                });
        if valid {
            let id = row.as_ref().expect("valid OTP has a database row").0;
            sqlx::query("UPDATE public.email_otps SET consumed_at=now() WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx)?;
        } else if let Some((id, _, _, attempts, _, unconsumed)) = row {
            if unconsumed && attempts < max_attempts {
                sqlx::query("UPDATE public.email_otps SET attempts=attempts+1 WHERE id=$1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(map_sqlx)?;
            }
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(valid)
    }
    async fn discard(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM public.email_otps WHERE id=$1 AND consumed_at IS NULL")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }
    async fn last_sent_at(&self, email: &str) -> Result<Option<OffsetDateTime>, AppError> {
        sqlx::query_as("SELECT created_at FROM public.email_otps WHERE lower(email)=lower($1) ORDER BY created_at DESC LIMIT 1")
            .bind(email).fetch_optional(&self.pool).await.map(|r: Option<(OffsetDateTime,)>| r.map(|v| v.0)).map_err(map_sqlx)
    }
    async fn sent_since(&self, email: &str, window_secs: i64) -> Result<i64, AppError> {
        sqlx::query_scalar("SELECT count(*) FROM public.email_otps WHERE lower(email)=lower($1) AND created_at > now()-make_interval(secs=>$2::double precision)")
            .bind(email).bind(window_secs).fetch_one(&self.pool).await.map_err(map_sqlx)
    }
}
