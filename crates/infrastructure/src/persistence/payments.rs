use crate::persistence::audit_util::write_audit;
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{
    AuditActor, PaymentRepository, ReceiptFile, ReceiptMeta, RecordedPayment, VoidedPayment,
};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{PaymentId, ReceiptId, ResidentId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgPaymentRepository {
    pool: PgPool,
}

impl PgPaymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PaymentRepository for PgPaymentRepository {
    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        ctx: &TenantContext,
        payment_id: PaymentId,
        receipt_id: ReceiptId,
        resident_id: ResidentId,
        amount_paise: i64,
        _receipt_number: &str,
        recorded_by: Option<UserId>,
        method: &str,
        reference: Option<&str>,
        idempotency_key: Option<&str>,
        actor: &AuditActor,
    ) -> Result<RecordedPayment, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        // Idempotency: when the client supplies a key, a retried POST hits the
        // (org, key) partial unique index; catch that and return the ORIGINAL
        // row's receipt number instead of a 500 / duplicate charge (B9).
        // Idempotency (B9): a retried POST with the same key must return the
        // ORIGINAL receipt, not a duplicate charge. A uniqueness-violation
        // would poison the transaction in Postgres, so check-then-insert inside
        // the (default READ COMMITTED) tx, relying on the partial unique index
        // to serialize concurrent first-writes.
        if let Some(key) = idempotency_key {
            let original: Option<(String, i64, Uuid, Uuid, Uuid)> = sqlx::query_as(
                "SELECT r.receipt_number, p.amount_paise, p.resident_id, p.id, r.id FROM payments p \
                 JOIN receipts r ON r.org_id = p.org_id AND r.payment_id = p.id \
                 WHERE p.org_id = $1 AND p.idempotency_key = $2",
            )
            .bind(org)
            .bind(key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            if let Some((n, orig_amount, orig_resident, orig_payment, orig_receipt)) = original {
                tx.commit().await.map_err(map_sqlx)?;
                // Same key + different payload = client error, NOT a silent replay.
                if orig_amount != amount_paise || orig_resident != resident_id.as_uuid() {
                    return Err(AppError::Conflict("payments_idempotency_uidx".to_string()));
                }
                return Ok(RecordedPayment {
                    payment_id: orig_payment,
                    receipt_id: orig_receipt,
                    receipt_number: n,
                    replayed: true,
                });
            }
        }
        let sequence: Option<(String, i32)> = sqlx::query_as(
            "INSERT INTO invoice_sequences (org_id, period, next_number) \
             VALUES ($1, to_char(now(), 'YYMM'), 2) \
             ON CONFLICT (org_id, period) DO UPDATE SET next_number = invoice_sequences.next_number + 1 \
             WHERE invoice_sequences.next_number <= 999 \
             RETURNING period, next_number - 1",
        )
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let (period, sequence) = sequence
            .ok_or_else(|| AppError::Conflict("monthly invoice sequence exhausted".into()))?;
        let invoice_number = format!("INV-{period}-{sequence:03}");

        sqlx::query(
            "INSERT INTO payments (id, org_id, resident_id, amount_paise, recorded_by, method, \
                                   upi_ref, idempotency_key) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(payment_id.as_uuid())
        .bind(org)
        .bind(resident_id.as_uuid())
        .bind(amount_paise)
        .bind(recorded_by.map(|u| u.as_uuid()))
        .bind(method)
        .bind(reference)
        .bind(idempotency_key)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO receipts (id, org_id, resident_id, payment_id, receipt_number, invoice_number, amount_paise, status) \
             VALUES ($1,$2,$3,$4,$5,$5,$6,'pending')",
        )
        .bind(receipt_id.as_uuid())
        .bind(org)
        .bind(resident_id.as_uuid())
        .bind(payment_id.as_uuid())
        .bind(&invoice_number)
        .bind(amount_paise)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "payment",
            "payment",
            payment_id.as_uuid(),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(RecordedPayment {
            payment_id: payment_id.as_uuid(),
            receipt_id: receipt_id.as_uuid(),
            receipt_number: invoice_number,
            replayed: false,
        })
    }

    /// Undo the resident's latest live payment.
    ///
    /// Soft-deletes the payment and the receipt issued for it, inside one
    /// transaction, and records the reversal in the audit log. Nothing is
    /// erased: `deleted_at` is what every balance query already filters on, so
    /// the resident's outstanding total simply recomputes. `FOR UPDATE` keeps
    /// two concurrent reversals from voiding two different payments.
    async fn void_last_payment(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
        actor: &AuditActor,
    ) -> Result<Option<VoidedPayment>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;

        let latest: Option<(Uuid, i64)> = sqlx::query_as(
            "SELECT id, amount_paise FROM payments \
             WHERE org_id = $1 AND resident_id = $2 AND deleted_at IS NULL \
             ORDER BY paid_at DESC, created_at DESC LIMIT 1 FOR UPDATE",
        )
        .bind(org)
        .bind(resident_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        let Some((payment_id, amount_paise)) = latest else {
            tx.commit().await.map_err(map_sqlx)?;
            return Ok(None);
        };

        // Withdraw the receipt too, otherwise the resident keeps a downloadable
        // receipt for money the ledger no longer counts as received.
        let invoice_number: Option<String> = sqlx::query_scalar(
            "UPDATE receipts SET deleted_at = now() \
             WHERE org_id = $1 AND payment_id = $2 AND deleted_at IS NULL \
             RETURNING invoice_number",
        )
        .bind(org)
        .bind(payment_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .flatten();

        let voided = sqlx::query(
            "UPDATE payments SET deleted_at = now() \
             WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
        )
        .bind(payment_id)
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if voided.rows_affected() != 1 {
            return Err(AppError::Conflict(
                "payment was already reversed".to_string(),
            ));
        }

        write_audit(
            &mut *tx,
            org,
            actor,
            "payment_void",
            "payment",
            payment_id,
            None,
            Some(serde_json::json!({
                "resident_id": resident_id.as_uuid(),
                "amount_paise": amount_paise,
                "invoice_number": invoice_number,
            })),
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;

        Ok(Some(VoidedPayment {
            payment_id,
            amount_paise,
            invoice_number,
        }))
    }

    async fn set_receipt_ready(
        &self,
        ctx: &TenantContext,
        receipt_id: ReceiptId,
        storage_key: &str,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let res = sqlx::query(
            "UPDATE receipts SET storage_key = $1, status = 'ready' WHERE id = $2 AND org_id = $3",
        )
        .bind(storage_key)
        .bind(receipt_id.as_uuid())
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    async fn paid_total(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
    ) -> Result<i64, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let total: i64 = sqlx::query_scalar(
            // Rent only. A deposit is money held on the resident's behalf and
            // owed back at move-out, so it must never satisfy a rent due.
            "SELECT COALESCE(SUM(amount_paise),0)::bigint FROM payments \
             WHERE org_id = $1 AND resident_id = $2 AND deleted_at IS NULL \
               AND kind = 'rent'",
        )
        .bind(org)
        .bind(resident_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(total)
    }

    async fn list_receipts(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
    ) -> Result<Vec<ReceiptMeta>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let rows: Vec<(Uuid, String, i64, String, i64)> = sqlx::query_as(
            "SELECT id, receipt_number, amount_paise, status, EXTRACT(EPOCH FROM issued_at)::bigint \
             FROM receipts WHERE org_id = $1 AND resident_id = $2 AND deleted_at IS NULL \
             ORDER BY issued_at DESC",
        )
        .bind(org)
        .bind(resident_id.as_uuid())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, receipt_number, amount_paise, status, issued_epoch)| ReceiptMeta {
                    id,
                    receipt_number,
                    amount_paise,
                    status,
                    issued_epoch,
                },
            )
            .collect())
    }

    async fn get_receipt(
        &self,
        ctx: &TenantContext,
        receipt_id: ReceiptId,
    ) -> Result<Option<ReceiptFile>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let row: Option<(Option<String>, String, Uuid)> = sqlx::query_as(
            "SELECT storage_key, status, resident_id FROM receipts \
             WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
        )
        .bind(receipt_id.as_uuid())
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(|(storage_key, status, resident_id)| ReceiptFile {
            storage_key,
            status,
            resident_id,
        }))
    }
}
