use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::VerificationRepository;
use pg_application::AppError;
use pg_domain::{OrgId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgVerificationRepository {
    pool: PgPool,
}

impl PgVerificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VerificationRepository for PgVerificationRepository {
    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        id: Uuid,
        user_id: UserId,
        org_id: OrgId,
        method: &str,
        status: &str,
        verified_name: Option<&str>,
        phone: Option<&str>,
        emergency_contact: Option<&str>,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.verifications \
             (id, user_id, org_id, method, status, verified_name, phone, emergency_contact, reason) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(user_id.as_uuid())
        .bind(org_id.as_uuid())
        .bind(method)
        .bind(status)
        .bind(verified_name)
        .bind(phone)
        .bind(emergency_contact)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}
