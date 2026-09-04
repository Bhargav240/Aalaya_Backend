use crate::persistence::audit_util::{select_jsonb, write_audit};
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{AuditActor, RoomRepository, RoomSummary};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{BranchId, FloorId, RoomId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgRoomRepository {
    pool: PgPool,
}

impl PgRoomRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoomRepository for PgRoomRepository {
    async fn create(
        &self,
        ctx: &TenantContext,
        id: RoomId,
        branch_id: BranchId,
        number: &str,
        cooling_type: &str,
        floor_id: Option<FloorId>,
        beds: u16,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query(
            "INSERT INTO rooms (id, org_id, branch_id, number, cooling_type, floor_id) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id.as_uuid())
        .bind(org)
        .bind(branch_id.as_uuid())
        .bind(number)
        .bind(cooling_type)
        .bind(floor_id.map(|f| f.as_uuid()))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        // Beds ride the same transaction, so a room is never left half-built.
        for index in 0..beds {
            let label = char::from_u32(u32::from(b'A') + u32::from(index))
                .ok_or_else(|| AppError::Validation("a room cannot have more than 26 beds".into()))?
                .to_string();
            sqlx::query("INSERT INTO beds (id, org_id, room_id, label) VALUES ($1,$2,$3,$4)")
                .bind(Uuid::new_v4())
                .bind(org)
                .bind(id.as_uuid())
                .bind(&label)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx)?;
        }

        let after = select_jsonb(&mut *tx, "rooms", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "create",
            "room",
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
    ) -> Result<Vec<RoomSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let rows: Vec<(Uuid, Uuid, String, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, branch_id, number, cooling_type, floor_id FROM rooms \
             WHERE org_id = $1 AND branch_id = $2 AND deleted_at IS NULL ORDER BY number",
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
                |(id, branch_id, number, cooling_type, floor_id)| RoomSummary {
                    id,
                    branch_id,
                    number,
                    cooling_type,
                    floor_id,
                },
            )
            .collect())
    }

    async fn branch_of(
        &self,
        ctx: &TenantContext,
        room_id: RoomId,
    ) -> Result<Option<BranchId>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let branch_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT branch_id FROM rooms \
             WHERE org_id = $1 AND id = $2 AND deleted_at IS NULL",
        )
        .bind(org)
        .bind(room_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(branch_id.map(BranchId::new))
    }
}
