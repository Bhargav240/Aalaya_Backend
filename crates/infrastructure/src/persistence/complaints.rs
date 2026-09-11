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
    Option<String>,
    Option<String>,
    Option<String>,
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
        tenant_name: r.9,
        floor_number: r.10,
        room_number: r.11,
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
            "INSERT INTO complaints (id, org_id, branch_id, raised_by, title, description, is_business_level) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id.as_uuid())
        .bind(org)
        .bind(branch_id.map(|b| b.as_uuid().to_owned()))
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
        let select_clause = "SELECT c.id, c.branch_id, c.raised_by, c.title, c.description, c.state, c.is_business_level, \
                    c.created_at, c.updated_at, \
                    COALESCE(res.full_name, u.full_name) AS tenant_name, \
                    COALESCE(fl.name, CASE WHEN rm.number ~ '^[0-9]' THEN 'Floor ' || LEFT(rm.number, 1) WHEN rm.number ILIKE 'G%' THEN 'Ground floor' ELSE NULL END) AS floor_number, \
                    rm.number AS room_number \
              FROM complaints c \
              LEFT JOIN public.platform_users u ON u.id = c.raised_by \
              LEFT JOIN LATERAL ( \
                  SELECT r.full_name, r.bed_id \
                  FROM residents r \
                  WHERE r.user_id = c.raised_by AND r.org_id = c.org_id AND r.deleted_at IS NULL \
                  ORDER BY (r.status = 'active') DESC, r.created_at DESC \
                  LIMIT 1 \
              ) res ON true \
              LEFT JOIN beds bd ON bd.id = res.bed_id AND bd.org_id = c.org_id AND bd.deleted_at IS NULL \
              LEFT JOIN rooms rm ON rm.id = bd.room_id AND rm.org_id = c.org_id AND rm.deleted_at IS NULL \
              LEFT JOIN floors fl ON fl.id = rm.floor_id AND fl.org_id = c.org_id AND fl.deleted_at IS NULL";
        let rows: Vec<Row> = match role {
            Role::Owner => match branch {
                Some(b) => sqlx::query_as(&format!(
                    "{select_clause} WHERE c.org_id = $1 AND c.deleted_at IS NULL AND c.branch_id = $2 ORDER BY c.created_at DESC LIMIT 500"
                ))
                .bind(org)
                .bind(b.as_uuid())
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?,
                None => sqlx::query_as(&format!(
                    "{select_clause} WHERE c.org_id = $1 AND c.deleted_at IS NULL ORDER BY c.created_at DESC LIMIT 500"
                ))
                .bind(org)
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?,
            },
            Role::Manager => match branch {
                Some(b) => sqlx::query_as(&format!(
                    "{select_clause} WHERE c.org_id = $1 AND c.deleted_at IS NULL \
                     AND c.is_business_level = false AND c.branch_id = $2 ORDER BY c.created_at DESC LIMIT 500"
                ))
                .bind(org)
                .bind(b.as_uuid())
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?,
                None => sqlx::query_as(&format!(
                    "{select_clause} WHERE c.org_id = $1 AND c.deleted_at IS NULL \
                     AND c.is_business_level = false ORDER BY c.created_at DESC LIMIT 500"
                ))
                .bind(org)
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx)?,
            },
            Role::PlatformOperator | Role::Prospect => {
                return Err(AppError::Forbidden);
            }
            Role::Tenant => sqlx::query_as(&format!(
                "{select_clause} WHERE c.org_id = $1 AND c.deleted_at IS NULL \
                 AND c.raised_by = $2 ORDER BY c.created_at DESC LIMIT 500"
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
