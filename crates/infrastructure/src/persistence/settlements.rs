use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{AuditActor, SettlementRecord, SettlementRepository};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgSettlementRepository {
    pool: PgPool,
}

impl PgSettlementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettlementRepository for PgSettlementRepository {
    async fn record(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        r: &SettlementRecord,
        _actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query(
            "INSERT INTO move_out_settlements (id, org_id, resident_id, last_day, notice_days, \
             rent_to_date_paise, paid_total_paise, outstanding_paise, deposit_held_paise, \
             deductions_total_paise, refund_paise, forfeited, note) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(id)
        .bind(org)
        .bind(r.resident_id)
        .bind(r.last_day)
        .bind(r.notice_days as i32)
        .bind(r.rent_to_date_paise)
        .bind(r.paid_total_paise)
        .bind(r.outstanding_paise)
        .bind(r.deposit_held_paise)
        .bind(r.deductions_total_paise)
        .bind(r.refund_paise)
        .bind(r.forfeited)
        .bind(&r.note)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        for (reason, amt) in &r.deductions {
            sqlx::query(
                "INSERT INTO move_out_deductions (id, settlement_id, reason, amount_paise) \
                 VALUES ($1,$2,$3,$4)",
            )
            .bind(Uuid::new_v4())
            .bind(id)
            .bind(reason)
            .bind(amt)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }
}
