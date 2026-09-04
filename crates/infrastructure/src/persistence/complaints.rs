use crate::persistence::audit_util::{select_jsonb, write_audit};
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{
    AuditActor, ComplaintRepository, ComplaintStateRow, ComplaintSummary,
};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{BranchId, ComplaintId, Role, UserId};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PgComplaintRepository {
    pool: PgPool,
}

impl PgComplaintRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type Row = (
    Uuid,
    Option<Uuid>,
    Uuid,
    String,
    Option<String>,
    String,
    bool,
    OffsetDateTime,
    OffsetDateTime,
);

fn to_summary(r: Row) -> ComplaintSummary {
    ComplaintSummary {
        id: r.0,
        branch_id: r.1,
        raised_by: r.2,
        title: r.3,
        description: r.4,
        state: r.5,
        is_business_level: r.6,
        created_at: r.7,
        updated_at: r.8,
    }
}

#[async_trait]
impl ComplaintRepository for PgComplaintRepository {
    #[allow(clippy::too_many_arguments)]
    async fn raise(
        &self,
        ctx: &TenantContext,
        id: ComplaintId,
        branch_id: Option<BranchId>,
        raised_by: UserId,
        title: &str,
        description: Option<&str>,
        is_business_level: bool,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query(
            "INSERT INTO complaints (id, org_id, branch_id, raised_by, title, description, state, is_business_level) \
             VALUES ($1,$2,$3,$4,$5,$6,'open',$7)",
        )
        .bind(id.as_uuid())
        .bind(org)
        .bind(branch_id.map(|b| b.as_uuid()))
        .bind(raised_by.as_uuid())
        .bind(title)
        .bind(description)
        .bind(is_business_level)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO complaint_events (org_id, complaint_id, from_state, to_state, by_user) \
             VALUES ($1,$2,NULL,'open',$3)",
        )
        .bind(org)
        .bind(id.as_uuid())
        .bind(raised_by.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let after = select_jsonb(&mut *tx, "complaints", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "create",
            "complaint",
            id.as_uuid(),
            None,
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_visible(
        &self,
        ctx: &TenantContext,
        role: Role,
        user_id: UserId,
        branch: Option<BranchId>,
    ) -> Result<Vec<ComplaintSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let cols = "id, branch_id, raised_by, title, description, state, is_business_level, \
                    created_at, updated_at";
        let rows: Vec<Row> = match role {
            Role::Owner => {
                sqlx::query_as(&format!(
                    "SELECT {cols} FROM complaints WHERE org_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 500"
                ))
                .bind(org)
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?
            }
            Role::Manager => match branch {
                Some(b) => sqlx::query_as(&format!(
                    "SELECT {cols} FROM complaints WHERE org_id = $1 AND deleted_at IS NULL \
                     AND is_business_level = false AND branch_id = $2 ORDER BY created_at DESC LIMIT 500"
                ))
                .bind(org)
                .bind(b.as_uuid())
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?,
                None => Vec::new(),
            },
            Role::PlatformOperator | Role::Prospect => {
                return Err(AppError::Forbidden);
            }
            Role::Tenant => sqlx::query_as(&format!(
                "SELECT {cols} FROM complaints WHERE org_id = $1 AND deleted_at IS NULL \
                 AND raised_by = $2 ORDER BY created_at DESC LIMIT 500"
            ))
            .bind(org)
            .bind(user_id.as_uuid())
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?,
        };
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(to_summary).collect())
    }

    async fn get_state(
        &self,
        ctx: &TenantContext,
        id: ComplaintId,
    ) -> Result<Option<ComplaintStateRow>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let row: Option<(String, Option<Uuid>, Uuid, bool)> = sqlx::query_as(
            "SELECT state, branch_id, raised_by, is_business_level FROM complaints \
             WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
        )
        .bind(id.as_uuid())
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(
            |(state, branch_id, raised_by, is_business_level)| ComplaintStateRow {
                state,
                branch_id,
                raised_by,
                is_business_level,
            },
        ))
    }

    async fn transition(
        &self,
        ctx: &TenantContext,
        id: ComplaintId,
        from: &str,
        to: &str,
        by: UserId,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let before = select_jsonb(&mut *tx, "complaints", id.as_uuid(), org)
            .await?
            .ok_or(AppError::NotFound)?;
        // Guard on `from` for optimistic concurrency.
        let res = sqlx::query(
            "UPDATE complaints SET state = $1, updated_at = now() \
             WHERE id = $2 AND org_id = $3 AND state = $4 AND deleted_at IS NULL",
        )
        .bind(to)
        .bind(id.as_uuid())
        .bind(org)
        .bind(from)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            return Err(AppError::Conflict("complaint state changed".into()));
        }
        sqlx::query(
            "INSERT INTO complaint_events (org_id, complaint_id, from_state, to_state, by_user) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(org)
        .bind(id.as_uuid())
        .bind(from)
        .bind(to)
        .bind(by.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let after = select_jsonb(&mut *tx, "complaints", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "complaint_transition",
            "complaint",
            id.as_uuid(),
            Some(before),
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }
}
