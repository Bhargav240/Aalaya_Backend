use crate::persistence::audit_util::{select_jsonb, write_audit};
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{AuditActor, NoticeRepository, NoticeSummary};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{BranchId, NoticeId, Role, UserId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgNoticeRepository {
    pool: PgPool,
}

impl PgNoticeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NoticeRepository for PgNoticeRepository {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        ctx: &TenantContext,
        id: NoticeId,
        branch_id: Option<BranchId>,
        title: &str,
        body: Option<&str>,
        created_by: UserId,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query(
            "INSERT INTO notices (id, org_id, branch_id, title, body, created_by) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id.as_uuid())
        .bind(org)
        .bind(branch_id.map(|b| b.as_uuid()))
        .bind(title)
        .bind(body)
        .bind(created_by.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let after = select_jsonb(&mut *tx, "notices", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "create",
            "notice",
            id.as_uuid(),
            None,
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_for(
        &self,
        ctx: &TenantContext,
        role: Role,
        branch: Option<BranchId>,
    ) -> Result<Vec<NoticeSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let cols = "id, branch_id, title, body, EXTRACT(EPOCH FROM created_at)::bigint";
        let rows: Vec<(Uuid, Option<Uuid>, String, Option<String>, i64)> = match role {
            Role::Owner => sqlx::query_as(&format!(
                "SELECT {cols} FROM notices WHERE org_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 500"
            ))
            .bind(org)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?,
            _ => match branch {
                Some(b) => sqlx::query_as(&format!(
                    "SELECT {cols} FROM notices WHERE org_id = $1 AND deleted_at IS NULL \
                     AND (branch_id = $2 OR branch_id IS NULL) ORDER BY created_at DESC LIMIT 500"
                ))
                .bind(org)
                .bind(b.as_uuid())
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?,
                None => sqlx::query_as(&format!(
                    "SELECT {cols} FROM notices WHERE org_id = $1 AND deleted_at IS NULL \
                     AND branch_id IS NULL ORDER BY created_at DESC LIMIT 500"
                ))
                .bind(org)
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?,
            },
        };
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, branch_id, title, body, created_epoch)| NoticeSummary {
                    id,
                    branch_id,
                    title,
                    body,
                    created_epoch,
                },
            )
            .collect())
    }
}
