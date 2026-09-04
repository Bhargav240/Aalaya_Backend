//! Live vacancy snapshot: derived counts of beds per branch. Written to the
//! control plane by the vacancy projector so anonymous marketplace reads
//! never touch tenant schemas at query time.
use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::{VacancyRepository, VacancySnapshot};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{BranchId, OrgId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgVacancyRepository {
    pool: PgPool,
}

impl PgVacancyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VacancyRepository for PgVacancyRepository {
    /// Run inside the TENANT schema — beds + active residents are tenant data.
    /// A bed is "available" when no active resident is assigned to it.
    async fn compute_for_branch(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<(i32, i32, Option<i64>), AppError> {
        let mut tx = crate::persistence::scope::scoped_tx(&self.pool, ctx).await?;
        let (total, avail): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*)::bigint, \
                    COUNT(*) FILTER (WHERE occ.id IS NULL)::bigint \
             FROM beds b \
             JOIN rooms r ON r.id = b.room_id AND r.org_id = b.org_id \
             LEFT JOIN residents occ ON occ.bed_id = b.id AND occ.org_id = b.org_id \
                AND occ.moved_out_at IS NULL AND occ.deleted_at IS NULL AND occ.status <> 'moved_out' \
             WHERE r.branch_id = $1 AND b.org_id = $2 AND b.deleted_at IS NULL AND r.deleted_at IS NULL",
        )
        .bind(branch_id.as_uuid())
        .bind(ctx.org_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let price: Option<i64> = sqlx::query_scalar(
            "SELECT MIN(res.rent_paise) FROM residents res \
             WHERE res.branch_id = $1 AND res.org_id = $2 AND res.rent_paise > 0 \
             AND res.moved_out_at IS NULL AND res.deleted_at IS NULL AND res.status <> 'moved_out'",
        )
        .bind(branch_id.as_uuid())
        .bind(ctx.org_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok((total as i32, avail as i32, price))
    }

    async fn upsert_snapshot(
        &self,
        org_id: OrgId,
        branch_id: BranchId,
        beds_total: i32,
        beds_available: i32,
        starting_price_paise: Option<i64>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.listing_vacancy (branch_id, org_id, beds_total, beds_available, starting_price_paise, updated_at, stale) \
             VALUES ($1,$2,$3,$4,$5, now(), false) \
             ON CONFLICT (branch_id) DO UPDATE SET \
             org_id=EXCLUDED.org_id, beds_total=EXCLUDED.beds_total, \
             beds_available=EXCLUDED.beds_available, starting_price_paise=EXCLUDED.starting_price_paise, \
             updated_at=now(), stale=false",
        )
        .bind(branch_id.as_uuid())
        .bind(org_id.as_uuid())
        .bind(beds_total)
        .bind(beds_available)
        .bind(starting_price_paise)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn snapshot_for(&self, branch_id: BranchId) -> Result<Option<VacancySnapshot>, AppError> {
        let row: Option<(i32, i32, Option<i64>, bool)> = sqlx::query_as(
            "SELECT beds_total, beds_available, starting_price_paise, stale \
             FROM public.listing_vacancy WHERE branch_id = $1",
        )
        .bind(branch_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(|(t, a, p, s)| VacancySnapshot {
            beds_total: t,
            beds_available: a,
            starting_price_paise: p,
            stale: s,
        }))
    }

    async fn mark_stale(&self, branch_id: BranchId) -> Result<(), AppError> {
        sqlx::query("UPDATE public.listing_vacancy SET stale = true WHERE branch_id = $1")
            .bind(branch_id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn live_branch_ids(&self) -> Result<Vec<(OrgId, BranchId)>, AppError> {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT org_id, branch_id FROM public.public_listings WHERE status = 'live'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|(o, b)| (OrgId::new(o), BranchId::new(b)))
            .collect())
    }
}
