use crate::persistence::audit_util::{select_jsonb, write_audit};
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{
    AuditActor, ResidentDetail, ResidentRepository, ResidentSummary,
};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{BedId, BranchId, PlanKind, ResidentId, UserId};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PgResidentRepository {
    pool: PgPool,
}

impl PgResidentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ResidentRepository for PgResidentRepository {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
        branch_id: BranchId,
        full_name: &str,
        phone: Option<&str>,
        plan_kind: PlanKind,
        rent_paise: i64,
        deposit_paise: i64,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query(
            "INSERT INTO residents (id, org_id, branch_id, full_name, phone, plan_kind, rent_paise, deposit_paise) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id.as_uuid())
        .bind(org)
        .bind(branch_id.as_uuid())
        .bind(full_name)
        .bind(phone)
        .bind(plan_kind.as_str())
        .bind(rent_paise)
        .bind(deposit_paise)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let after = select_jsonb(&mut *tx, "residents", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "create",
            "resident",
            id.as_uuid(),
            None,
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_by_branch(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<Vec<ResidentSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let rows: Vec<(Uuid, Uuid, String, String, String)> = sqlx::query_as(
            "SELECT id, branch_id, full_name, status, plan_kind FROM residents \
             WHERE org_id = $1 AND branch_id = $2 AND deleted_at IS NULL ORDER BY full_name",
        )
        .bind(org)
        .bind(branch_id.as_uuid())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, branch_id, full_name, status, plan_kind)| ResidentSummary {
                    id,
                    branch_id,
                    full_name,
                    status,
                    plan_kind,
                },
            )
            .collect())
    }

    async fn get(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
    ) -> Result<Option<ResidentDetail>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        #[allow(clippy::type_complexity)]
        let row: Option<(
            Uuid,
            Uuid,
            String,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
            String,
            String,
            i64,
            i64,
            OffsetDateTime,
            Option<OffsetDateTime>,
        )> = sqlx::query_as(
            "SELECT r.id, r.branch_id, b.name, rm.number, bed.label, r.full_name, r.phone, r.status, r.plan_kind, r.rent_paise, r.deposit_paise, r.created_at, r.moved_out_at \
             FROM residents r JOIN branches b ON b.id = r.branch_id AND b.org_id = r.org_id \
             LEFT JOIN beds bed ON bed.id = r.bed_id AND bed.org_id = r.org_id AND bed.deleted_at IS NULL \
             LEFT JOIN rooms rm ON rm.id = bed.room_id AND rm.org_id = r.org_id AND rm.deleted_at IS NULL \
             WHERE r.id = $1 AND r.org_id = $2 AND r.deleted_at IS NULL",
        )
        .bind(id.as_uuid())
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(
            |(
                id,
                branch_id,
                branch_name,
                room_number,
                bed_label,
                full_name,
                phone,
                status,
                plan_kind,
                rent_paise,
                deposit_paise,
                joined,
                moved_out_at,
            )| {
                ResidentDetail {
                    id,
                    branch_id,
                    branch_name,
                    room_number,
                    bed_label,
                    full_name,
                    phone,
                    status,
                    plan_kind,
                    rent_paise,
                    deposit_paise,
                    joined,
                    moved_out_at,
                }
            },
        ))
    }

    async fn mark_moved_out(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
        last_day: time::Date,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let moved_out_at = last_day.midnight().assume_utc();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let before = select_jsonb(&mut *tx, "residents", id.as_uuid(), org)
            .await?
            .ok_or(AppError::NotFound)?;
        let membership_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT membership_id FROM residents \
             WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
        )
        .bind(id.as_uuid())
        .bind(org)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "UPDATE residents SET status = 'moved_out', moved_out_at = $3, bed_id = NULL \
             WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
        )
        .bind(id.as_uuid())
        .bind(org)
        .bind(moved_out_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if let Some(membership_id) = membership_id {
            sqlx::query(
                "UPDATE public.user_org_roles \
                 SET membership_status = 'moved_out', ended_at = $2 \
                 WHERE id = $1 AND org_id = $3 AND role = 'tenant'",
            )
            .bind(membership_id)
            .bind(moved_out_at)
            .bind(org)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }
        let after = select_jsonb(&mut *tx, "residents", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "move_out",
            "resident",
            id.as_uuid(),
            Some(before),
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn link_user(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
        user_id: UserId,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let before = select_jsonb(&mut *tx, "residents", id.as_uuid(), org)
            .await?
            .ok_or(AppError::NotFound)?;
        let res = sqlx::query(
            "UPDATE residents SET user_id = $1 WHERE id = $2 AND org_id = $3 AND deleted_at IS NULL",
        )
        .bind(user_id.as_uuid())
        .bind(id.as_uuid())
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        let after = select_jsonb(&mut *tx, "residents", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "link_user",
            "resident",
            id.as_uuid(),
            Some(before),
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn assign_bed(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
        bed_id: Option<BedId>,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let before = select_jsonb(&mut *tx, "residents", id.as_uuid(), org)
            .await?
            .ok_or(AppError::NotFound)?;
        let res = sqlx::query(
            "UPDATE residents SET bed_id = $1 WHERE id = $2 AND org_id = $3 AND deleted_at IS NULL",
        )
        .bind(bed_id.map(|b| b.as_uuid()))
        .bind(id.as_uuid())
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        let after = select_jsonb(&mut *tx, "residents", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "assign_bed",
            "resident",
            id.as_uuid(),
            Some(before),
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn occupant_of_bed(
        &self,
        ctx: &TenantContext,
        bed_id: BedId,
    ) -> Result<Option<ResidentId>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM residents WHERE bed_id = $1 AND org_id = $2 \
             AND deleted_at IS NULL AND status <> 'moved_out' LIMIT 1",
        )
        .bind(bed_id.as_uuid())
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(|(id,)| ResidentId::new(id)))
    }

    async fn resident_id_of_user(
        &self,
        ctx: &TenantContext,
        user_id: UserId,
    ) -> Result<Option<ResidentId>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM residents WHERE user_id = $1 AND org_id = $2 AND deleted_at IS NULL LIMIT 1",
        )
        .bind(user_id.as_uuid())
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(|(id,)| ResidentId::new(id)))
    }
}
