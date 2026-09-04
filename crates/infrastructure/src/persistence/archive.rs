use crate::persistence::audit_util::write_audit;
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{ArchiveRepository, AuditActor};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PgArchiveRepository {
    pool: PgPool,
}

impl PgArchiveRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Only these soft-deletable tables may be archived. The name is a fixed
/// literal (never user input), so interpolation is safe.
fn table_ok(table: &str) -> bool {
    matches!(
        table,
        "branches"
            | "rooms"
            | "beds"
            | "residents"
            | "payments"
            | "receipts"
            | "complaints"
            | "notices"
    )
}

#[async_trait]
impl ArchiveRepository for PgArchiveRepository {
    async fn fetch_deleted(
        &self,
        ctx: &TenantContext,
        table: &str,
        cutoff: OffsetDateTime,
    ) -> Result<(serde_json::Value, i64), AppError> {
        if !table_ok(table) {
            return Err(AppError::Validation("not archivable".into()));
        }
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let row: (serde_json::Value, i64) = sqlx::query_as(&format!(
            "SELECT COALESCE(jsonb_agg(t), '[]'::jsonb), COUNT(*)::bigint FROM {table} t \
             WHERE org_id = $1 AND deleted_at IS NOT NULL AND deleted_at < $2"
        ))
        .bind(org)
        .bind(cutoff)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row)
    }

    async fn purge(
        &self,
        ctx: &TenantContext,
        table: &str,
        cutoff: OffsetDateTime,
        object_key: &str,
        row_count: i64,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        if !table_ok(table) {
            return Err(AppError::Validation("not archivable".into()));
        }
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query(&format!(
            "DELETE FROM {table} WHERE org_id = $1 AND deleted_at IS NOT NULL AND deleted_at < $2"
        ))
        .bind(org)
        .bind(cutoff)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO archive_manifest (id, org_id, table_name, object_key, row_count) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(org)
        .bind(table)
        .bind(object_key)
        .bind(row_count)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "archive",
            table,
            Uuid::nil(),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }
}
