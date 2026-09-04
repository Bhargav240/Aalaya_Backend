use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::OrganizationRepository;
use pg_application::AppError;
use pg_domain::{OrgId, Organization};
use sqlx::PgPool;

pub struct PgOrganizationRepository {
    pool: PgPool,
}

impl PgOrganizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrganizationRepository for PgOrganizationRepository {
    async fn create(&self, org: &Organization) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
        sqlx::query("INSERT INTO public.organizations (id, name) VALUES ($1, $2)")
            .bind(org.id.as_uuid())
            .bind(&org.name)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO public.tenant_placement (org_id, mode, schema_name) VALUES ($1, 'pooled', 'pool')",
        )
        .bind(org.id.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_all_live(&self) -> Result<Vec<OrgId>, AppError> {
        let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT id FROM public.organizations WHERE deleted_at IS NULL ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(id,)| OrgId::new(id)).collect())
    }

    async fn exists(&self, org_id: OrgId) -> Result<bool, AppError> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM public.organizations WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(org_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.is_some())
    }
}
