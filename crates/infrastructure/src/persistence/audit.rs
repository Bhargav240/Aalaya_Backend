use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{AuditRepository, AuditSummary};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgAuditRepository {
    pool: PgPool,
}

impl PgAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditRepository for PgAuditRepository {
    async fn list_recent(
        &self,
        ctx: &TenantContext,
        limit: i64,
    ) -> Result<Vec<AuditSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let rows: Vec<(i64, String, Option<String>, Option<Uuid>, Option<Uuid>, i64)> =
            sqlx::query_as(
                "SELECT id, action, entity_type, entity_id, actor_id, \
                 EXTRACT(EPOCH FROM created_at)::bigint \
                 FROM audit_log WHERE org_id = $1 ORDER BY id DESC LIMIT $2",
            )
            .bind(org)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, action, entity_type, entity_id, actor_id, epoch)| AuditSummary {
                    id,
                    action,
                    entity_type,
                    entity_id,
                    actor_id,
                    created_at_epoch: epoch,
                },
            )
            .collect())
    }
}
