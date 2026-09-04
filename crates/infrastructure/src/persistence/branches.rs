use crate::persistence::audit_util::{select_jsonb, write_audit};
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{AuditActor, BranchRepository, BranchSummary};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::BranchId;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgBranchRepository {
    pool: PgPool,
}

impl PgBranchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BranchRepository for PgBranchRepository {
    async fn create(
        &self,
        ctx: &TenantContext,
        id: BranchId,
        name: &str,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query("INSERT INTO branches (id, org_id, name) VALUES ($1,$2,$3)")
            .bind(id.as_uuid())
            .bind(org)
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        let after = select_jsonb(&mut *tx, "branches", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "create",
            "branch",
            id.as_uuid(),
            None,
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn list(&self, ctx: &TenantContext) -> Result<Vec<BranchSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, name FROM branches WHERE org_id = $1 AND deleted_at IS NULL ORDER BY name",
        )
        .bind(org)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|(id, name)| BranchSummary { id, name })
            .collect())
    }

    async fn count(&self, ctx: &TenantContext) -> Result<i64, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM branches WHERE org_id = $1 AND deleted_at IS NULL",
        )
        .bind(org)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(n)
    }
}
