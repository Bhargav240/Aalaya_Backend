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
        let rows: Vec<(i64, String, Option<String>, Option<Uuid>, Option<Uuid>, Option<String>, i64)> =
            sqlx::query_as(
                "SELECT a.id, a.action, a.entity_type, a.entity_id, a.actor_id, \
                 u.full_name, \
                 EXTRACT(EPOCH FROM a.created_at)::bigint \
                 FROM audit_log a \
                 LEFT JOIN public.platform_users u ON u.id = a.actor_id \
                 WHERE a.org_id = $1 ORDER BY a.id DESC LIMIT $2",
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
                |(id, action, entity_type, entity_id, actor_id, actor_name, epoch)| AuditSummary {
                    id,
                    action,
                    entity_type,
                    entity_id,
                    actor_id,
                    actor_name,
                    created_at_epoch: epoch,
                },
            )
            .collect())
    }
}
