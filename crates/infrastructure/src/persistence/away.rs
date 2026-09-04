use crate::persistence::audit_util::{select_jsonb, write_audit};
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{
    AuditActor, AwayPeriod, AwayPeriodDetail, AwayRepository,
};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{BranchId, ResidentId};
use sqlx::PgPool;
use time::Date;
use uuid::Uuid;

pub struct PgAwayRepository {
    pool: PgPool,
}

impl PgAwayRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn row_to_period(row: sqlx::postgres::PgRow) -> AwayPeriod {
    use sqlx::Row;
    AwayPeriod {
        id: row.get("id"),
        resident_id: row.get("resident_id"),
        from_date: row.get("from_date"),
        to_date: row.get("to_date"),
        note: row.get("note"),
        status: row.get("status"),
        created_at: row.get("created_at"),
    }
}

#[async_trait]
impl AwayRepository for PgAwayRepository {
    async fn create(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        resident_id: ResidentId,
        from_date: Date,
        to_date: Date,
        note: Option<&str>,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query(
            "INSERT INTO away_periods (id, org_id, resident_id, from_date, to_date, note) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(org)
        .bind(resident_id.as_uuid())
        .bind(from_date)
        .bind(to_date)
        .bind(note)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let after = select_jsonb(&mut *tx, "away_periods", id, org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "create",
            "away_period",
            id,
            None,
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_active_for_resident(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
    ) -> Result<Vec<AwayPeriod>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let rows = sqlx::query(
            "SELECT id, resident_id, from_date, to_date, note, status, created_at \
             FROM away_periods \
             WHERE org_id=$1 AND resident_id=$2 AND status='active' AND deleted_at IS NULL \
             ORDER BY from_date DESC",
        )
        .bind(org)
        .bind(resident_id.as_uuid())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(row_to_period).collect())
    }

    async fn list_active_detailed(
        &self,
        ctx: &TenantContext,
        branch: Option<BranchId>,
    ) -> Result<Vec<AwayPeriodDetail>, AppError> {
        use sqlx::Row;
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        // `$2 IS NULL OR ...` keeps one statement for both the owner's
        // org-wide list and a manager's single branch.
        let rows = sqlx::query(
            "SELECT ap.id, ap.resident_id, ap.from_date, ap.to_date, ap.note, ap.status, \
                    ap.created_at, res.full_name, res.branch_id, b.name AS branch_name, \
                    rm.number AS room_number \
             FROM away_periods ap \
             JOIN residents res ON res.id = ap.resident_id AND res.org_id = ap.org_id \
                  AND res.deleted_at IS NULL \
             JOIN branches b ON b.id = res.branch_id AND b.org_id = res.org_id \
                  AND b.deleted_at IS NULL \
             LEFT JOIN beds bd ON bd.id = res.bed_id AND bd.org_id = res.org_id \
             LEFT JOIN rooms rm ON rm.id = bd.room_id AND rm.org_id = res.org_id \
             WHERE ap.org_id=$1 AND ap.status='active' AND ap.deleted_at IS NULL \
               AND ($2::uuid IS NULL OR res.branch_id = $2) \
             ORDER BY ap.from_date DESC",
        )
        .bind(org)
        .bind(branch.map(|b| b.as_uuid().to_owned()))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|row| AwayPeriodDetail {
                resident_name: row.get("full_name"),
                room_number: row.get("room_number"),
                branch_id: row.get("branch_id"),
                branch_name: row.get("branch_name"),
                period: row_to_period(row),
            })
            .collect())
    }

    async fn update(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        resident_id: ResidentId,
        from_date: Date,
        to_date: Date,
        note: Option<&str>,
        actor: &AuditActor,
    ) -> Result<bool, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let before = select_jsonb(&mut *tx, "away_periods", id, org).await?;
        // Matching on resident_id as well as id is the authorization: one
        // tenant cannot rewrite another tenant's dates by guessing a uuid.
        let res = sqlx::query(
            "UPDATE away_periods SET from_date=$4, to_date=$5, note=$6 \
             WHERE id=$1 AND org_id=$2 AND resident_id=$3 AND status='active' \
               AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(org)
        .bind(resident_id.as_uuid())
        .bind(from_date)
        .bind(to_date)
        .bind(note)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            tx.rollback().await.map_err(map_sqlx)?;
            return Ok(false);
        }
        let after = select_jsonb(&mut *tx, "away_periods", id, org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "update",
            "away_period",
            id,
            before,
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(true)
    }

    async fn cancel(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        resident_id: ResidentId,
        actor: &AuditActor,
    ) -> Result<bool, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let before = select_jsonb(&mut *tx, "away_periods", id, org).await?;
        let res = sqlx::query(
            "UPDATE away_periods SET status='cancelled', cancelled_at=now() \
             WHERE id=$1 AND org_id=$2 AND resident_id=$3 AND status='active' AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(org)
        .bind(resident_id.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            tx.rollback().await.map_err(map_sqlx)?;
            return Ok(false);
        }
        let after = select_jsonb(&mut *tx, "away_periods", id, org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "cancel",
            "away_period",
            id,
            before,
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(true)
    }

    async fn overlaps(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
        from: Date,
        to: Date,
        exclude: Option<Uuid>,
    ) -> Result<Vec<AwayPeriod>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let rows = sqlx::query(
            "SELECT id, resident_id, from_date, to_date, note, status, created_at \
             FROM away_periods \
             WHERE org_id=$1 AND resident_id=$2 AND status='active' AND deleted_at IS NULL \
               AND from_date <= $4 AND to_date >= $3 \
               AND ($5::uuid IS NULL OR id <> $5)",
        )
        .bind(org)
        .bind(resident_id.as_uuid())
        .bind(from)
        .bind(to)
        .bind(exclude)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(row_to_period).collect())
    }
}
