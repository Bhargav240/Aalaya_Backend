use crate::persistence::audit_util::{select_jsonb, write_audit};
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{AuditActor, FloorRepository, FloorSummary, NewRoomSpec};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{BranchId, FloorId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgFloorRepository {
    pool: PgPool,
}

impl PgFloorRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FloorRepository for PgFloorRepository {
    async fn create(
        &self,
        ctx: &TenantContext,
        id: FloorId,
        branch_id: BranchId,
        name: &str,
        sort_order: i32,
        rooms: &[NewRoomSpec],
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        sqlx::query(
            "INSERT INTO floors (id, org_id, branch_id, name, sort_order) VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id.as_uuid())
        .bind(org)
        .bind(branch_id.as_uuid())
        .bind(name)
        .bind(sort_order)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        // Rooms and their beds ride the same transaction: if any room number
        // collides, the whole floor rolls back rather than leaving the owner
        // with a floor holding half the rooms they asked for.
        for room in rooms {
            let room_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO rooms (id, org_id, branch_id, number, cooling_type, floor_id) \
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(room_id)
            .bind(org)
            .bind(branch_id.as_uuid())
            .bind(room.number.trim())
            .bind(room.cooling_type.as_str())
            .bind(id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;

            for index in 0..room.beds {
                let label = char::from_u32(u32::from(b'A') + u32::from(index))
                    .ok_or_else(|| {
                        AppError::Validation("a room cannot have more than 26 beds".into())
                    })?
                    .to_string();
                sqlx::query("INSERT INTO beds (id, org_id, room_id, label) VALUES ($1,$2,$3,$4)")
                    .bind(Uuid::new_v4())
                    .bind(org)
                    .bind(room_id)
                    .bind(&label)
                    .execute(&mut *tx)
                    .await
                    .map_err(map_sqlx)?;
            }
        }

        let after = select_jsonb(&mut *tx, "floors", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "create",
            "floor",
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
    ) -> Result<Vec<FloorSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        // The room count comes from the same query rather than a follow-up per
        // floor, so a branch with twenty floors is still one round trip.
        let rows: Vec<(Uuid, Uuid, String, i32, i64)> = sqlx::query_as(
            "SELECT f.id, f.branch_id, f.name, f.sort_order, \
                    COUNT(r.id) FILTER (WHERE r.deleted_at IS NULL) AS room_count \
             FROM floors f \
             LEFT JOIN rooms r ON r.floor_id = f.id AND r.org_id = f.org_id \
             WHERE f.org_id = $1 AND f.branch_id = $2 AND f.deleted_at IS NULL \
             GROUP BY f.id, f.branch_id, f.name, f.sort_order \
             ORDER BY f.sort_order, f.name",
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
                |(id, branch_id, name, sort_order, room_count)| FloorSummary {
                    id,
                    branch_id,
                    name,
                    sort_order,
                    room_count,
                },
            )
            .collect())
    }

    async fn rename(
        &self,
        ctx: &TenantContext,
        id: FloorId,
        name: &str,
        sort_order: i32,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let before = select_jsonb(&mut *tx, "floors", id.as_uuid(), org).await?;
        let result = sqlx::query(
            "UPDATE floors SET name = $3, sort_order = $4 \
             WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
        )
        .bind(id.as_uuid())
        .bind(org)
        .bind(name)
        .bind(sort_order)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        let after = select_jsonb(&mut *tx, "floors", id.as_uuid(), org).await?;
        write_audit(
            &mut *tx,
            org,
            actor,
            "update",
            "floor",
            id.as_uuid(),
            before,
            after,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn delete(
        &self,
        ctx: &TenantContext,
        id: FloorId,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;

        // Rooms outlive floors. Deleting a floor with rooms still on it would
        // either orphan them or silently drop their location, so it is refused
        // and the owner is told what to move first.
        let occupied: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rooms \
             WHERE org_id = $1 AND floor_id = $2 AND deleted_at IS NULL",
        )
        .bind(org)
        .bind(id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if occupied > 0 {
            return Err(AppError::Validation(format!(
                "move or delete the {occupied} room(s) on this floor first"
            )));
        }

        let before = select_jsonb(&mut *tx, "floors", id.as_uuid(), org).await?;
        let result = sqlx::query(
            "UPDATE floors SET deleted_at = now() \
             WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
        )
        .bind(id.as_uuid())
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        write_audit(
            &mut *tx,
            org,
            actor,
            "delete",
            "floor",
            id.as_uuid(),
            before,
            None,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn branch_of(
        &self,
        ctx: &TenantContext,
        floor_id: FloorId,
    ) -> Result<Option<BranchId>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT branch_id FROM floors WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
        )
        .bind(floor_id.as_uuid())
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(|(id,)| BranchId::new(id)))
    }
}
