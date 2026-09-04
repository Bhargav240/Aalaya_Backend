use crate::persistence::audit_util::{select_jsonb, write_audit};
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{AuditActor, BedRepository, BedSummary};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{BedId, BranchId, RoomId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgBedRepository {
    pool: PgPool,
}

impl PgBedRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BedRepository for PgBedRepository {
    async fn create(
        &self,
        ctx: &TenantContext,
        id: BedId,
        room_id: RoomId,
        label: &str,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query("INSERT INTO beds (id, org_id, room_id, label) VALUES ($1,$2,$3,$4)")
            .bind(id.as_uuid())
            .bind(org)
            .bind(room_id.as_uuid())
            .bind(label)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        let after = select_jsonb(&mut *tx, "beds", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "create",
            "bed",
            id.as_uuid(),
            None,
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_by_room(
        &self,
        ctx: &TenantContext,
        room_id: RoomId,
    ) -> Result<Vec<BedSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        #[allow(clippy::type_complexity)]
        let rows: Vec<(Uuid, Uuid, String, Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT b.id, b.room_id, b.label, r.id, r.full_name FROM beds b \
             LEFT JOIN residents r ON r.bed_id = b.id AND r.org_id = b.org_id \
                 AND r.deleted_at IS NULL AND r.status <> 'moved_out' \
             WHERE b.org_id = $1 AND b.room_id = $2 AND b.deleted_at IS NULL ORDER BY b.label",
        )
        .bind(org)
        .bind(room_id.as_uuid())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, room_id, label, occupant_resident_id, occupant_name)| BedSummary {
                    id,
                    room_id,
                    label,
                    occupant_resident_id,
                    occupant_name,
                },
            )
            .collect())
    }

    async fn list_by_branch(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<Vec<BedSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        #[allow(clippy::type_complexity)]
        let rows: Vec<(Uuid, Uuid, String, Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT b.id, b.room_id, b.label, r.id, r.full_name FROM beds b \
             JOIN rooms rm ON rm.id = b.room_id AND rm.org_id = b.org_id AND rm.deleted_at IS NULL \
             LEFT JOIN residents r ON r.bed_id = b.id AND r.org_id = b.org_id \
                 AND r.deleted_at IS NULL AND r.status <> 'moved_out' \
             WHERE b.org_id = $1 AND rm.branch_id = $2 AND b.deleted_at IS NULL \
             ORDER BY rm.number, b.label",
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
                |(id, room_id, label, occupant_resident_id, occupant_name)| BedSummary {
                    id,
                    room_id,
                    label,
                    occupant_resident_id,
                    occupant_name,
                },
            )
            .collect())
    }

    async fn room_of(
        &self,
        ctx: &TenantContext,
        bed_id: BedId,
    ) -> Result<Option<RoomId>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT room_id FROM beds WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
        )
        .bind(bed_id.as_uuid())
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(|(id,)| RoomId::new(id)))
    }
}
