use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{
    BranchFinance, FinanceReport, FinanceRepository, ResidentDue,
};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::BranchId;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgFinanceRepository {
    pool: PgPool,
}

impl PgFinanceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// Day-based accrual, bit-for-bit the same formula as SimpleBillingEngine
// (the single source of truth): accrue from `created_at` up to the day the
// resident moved out (never past), then prorate by plan. The `GREATEST(...,0)`
// mirrors the engine's `.max(0)` guard.
const ACCRUED_EXPR: &str = "COALESCE(SUM((CASE plan_kind \
     WHEN 'weekly' THEN rent_paise * GREATEST(LEAST(CURRENT_DATE, COALESCE(moved_out_at::date, CURRENT_DATE)) - created_at::date, 0) / 7 \
     WHEN 'daily'  THEN rent_paise * GREATEST(LEAST(CURRENT_DATE, COALESCE(moved_out_at::date, CURRENT_DATE)) - created_at::date, 0) \
     ELSE rent_paise * GREATEST(LEAST(CURRENT_DATE, COALESCE(moved_out_at::date, CURRENT_DATE)) - created_at::date, 0) / 30 END)), 0)::bigint";

#[async_trait]
impl FinanceRepository for PgFinanceRepository {
    async fn branch_report(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<FinanceReport, AppError> {
        let org = ctx.org_id.as_uuid();
        let branch = branch_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;

        // Two figures from one pass: everything collected (what actually came
        // in), and the rent share of it. Outstanding rent must be measured
        // against rent only -- a deposit is refundable and cannot settle a due.
        let (collected, rent_collected): (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(p.amount_paise),0)::bigint, \
                    COALESCE(SUM(p.amount_paise) FILTER (WHERE p.kind = 'rent'),0)::bigint \
             FROM payments p \
             JOIN residents r ON r.id = p.resident_id \
             WHERE p.org_id = $1 AND r.branch_id = $2 AND p.deleted_at IS NULL",
        )
        .bind(org)
        .bind(branch)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        let (deposits, accrued, active, total): (i64, i64, i64, i64) = sqlx::query_as(&format!(
            "SELECT \
             COALESCE(SUM(deposit_paise) FILTER (WHERE status <> 'moved_out'),0)::bigint, \
             {ACCRUED_EXPR}, \
             COUNT(*) FILTER (WHERE status <> 'moved_out')::bigint, \
             COUNT(*)::bigint \
             FROM residents WHERE org_id = $1 AND branch_id = $2 AND deleted_at IS NULL"
        ))
        .bind(org)
        .bind(branch)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;

        Ok(FinanceReport {
            collected_paise: collected,
            deposits_collected_paise: collected - rent_collected,
            deposits_paise: deposits,
            accrued_paise: accrued,
            outstanding_paise: (accrued - rent_collected).max(0),
            residents_active: active,
            residents_total: total,
            beds_total: 0,
            beds_occupied: 0,
        })
    }

    async fn branch_dues(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<Vec<ResidentDue>, AppError> {
        let org = ctx.org_id.as_uuid();
        let branch = branch_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let rows: Vec<(Uuid, String, String, String, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT r.id, r.full_name, r.plan_kind, r.status, r.rent_paise, r.deposit_paise, \
                    COALESCE((SELECT SUM(p.amount_paise) FROM payments p \
                              WHERE p.org_id = r.org_id AND p.resident_id = r.id \
                                AND p.deleted_at IS NULL AND p.kind = 'rent'), 0)::bigint AS paid, \
                    (CASE r.plan_kind \
                        WHEN 'weekly' THEN r.rent_paise * GREATEST(LEAST(CURRENT_DATE, COALESCE(r.moved_out_at::date, CURRENT_DATE)) - r.created_at::date, 0) / 7 \
                        WHEN 'daily'  THEN r.rent_paise * GREATEST(LEAST(CURRENT_DATE, COALESCE(r.moved_out_at::date, CURRENT_DATE)) - r.created_at::date, 0) \
                        ELSE r.rent_paise * GREATEST(LEAST(CURRENT_DATE, COALESCE(r.moved_out_at::date, CURRENT_DATE)) - r.created_at::date, 0) / 30 \
                     END)::bigint AS accrued \
             FROM residents r \
             WHERE r.org_id = $1 AND r.branch_id = $2 AND r.deleted_at IS NULL \
             ORDER BY r.full_name"
        )
        .bind(org)
        .bind(branch)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, full_name, plan_kind, status, rent_paise, deposit_paise, paid, accrued)| {
                    ResidentDue {
                        resident_id: id,
                        full_name,
                        plan_kind,
                        status,
                        rent_paise,
                        deposit_paise,
                        paid_total_paise: paid,
                        accrued_paise: accrued,
                        outstanding_paise: (accrued - paid).max(0),
                    }
                },
            )
            .collect())
    }

    async fn org_report(
        &self,
        ctx: &TenantContext,
    ) -> Result<(FinanceReport, Vec<BranchFinance>), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let branches: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, name FROM branches WHERE org_id = $1 AND deleted_at IS NULL ORDER BY name",
        )
        .bind(org)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;

        let mut list = Vec::new();
        let mut total = FinanceReport {
            collected_paise: 0,
            deposits_collected_paise: 0,
            deposits_paise: 0,
            accrued_paise: 0,
            outstanding_paise: 0,
            residents_active: 0,
            residents_total: 0,
            beds_total: 0,
            beds_occupied: 0,
        };
        for (bid, name) in branches {
            let r = self.branch_report(ctx, BranchId::new(bid)).await?;
            total.collected_paise += r.collected_paise;
            total.deposits_collected_paise += r.deposits_collected_paise;
            total.deposits_paise += r.deposits_paise;
            total.accrued_paise += r.accrued_paise;
            total.outstanding_paise += r.outstanding_paise;
            total.residents_active += r.residents_active;
            total.residents_total += r.residents_total;
            total.beds_total += r.beds_total;
            total.beds_occupied += r.beds_occupied;
            list.push(BranchFinance {
                branch_id: bid,
                branch_name: name,
                report: r,
            });
        }
        Ok((total, list))
    }
}
