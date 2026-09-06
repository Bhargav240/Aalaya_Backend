//! Maker-checker applier. Applies an update/delete to the target entity AND
//! writes the audit row in ONE transaction. Owner edits go through `apply_now`;
//! manager edits are queued via `create_pending` and later `approve`d.
use crate::persistence::audit_util::{select_jsonb, write_audit};
use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{
    AuditActor, ChangeRequestRepository, ChangeRequestSummary, ChangeTarget,
};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::UserId;
use sqlx::PgConnection;
use uuid::Uuid;

pub struct PgChangeRequestRepository {
    pool: sqlx::PgPool,
}

impl PgChangeRequestRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

/// Maps a change-request entity_type to its physical table. The result is a
/// fixed literal, never user input, so it is safe to interpolate.
fn table_for(entity_type: &str) -> Result<&'static str, AppError> {
    match entity_type {
        "room" => Ok("rooms"),
        "resident" => Ok("residents"),
        "branch" => Ok("branches"),
        "bed" => Ok("beds"),
        "floor" => Ok("floors"),
        other => Err(AppError::Validation(format!("unsupported entity: {other}"))),
    }
}

/// Insert an entity a manager proposed and an owner approved.
///
/// The id was allocated when the request was raised, not here, so the row an
/// owner approves is the one the request has been referring to all along — and
/// `change_requests.entity_id` is NOT NULL, which a deferred id could not
/// satisfy.
///
/// Uniqueness (a floor name, say) is only enforced at this insert, so two
/// pending requests for the same name both queue and the second fails here
/// with the database's own error. That is the honest outcome: the first
/// approval genuinely took the name.
async fn apply_create(
    conn: &mut PgConnection,
    entity_type: &str,
    id: Uuid,
    org_id: Uuid,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let text = |key: &str| payload.get(key).and_then(|v| v.as_str());
    let required = |key: &str| -> Result<&str, AppError> {
        text(key).ok_or_else(|| AppError::Validation(format!("{key} is required")))
    };
    let uuid_field = |key: &str| -> Result<Uuid, AppError> {
        required(key).and_then(|v| {
            Uuid::parse_str(v).map_err(|_| AppError::Validation(format!("{key} must be a uuid")))
        })
    };

    match entity_type {
        "floor" => {
            sqlx::query(
                "INSERT INTO floors (id, org_id, branch_id, name, sort_order) \
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(id)
            .bind(org_id)
            .bind(uuid_field("branch_id")?)
            .bind(required("name")?)
            .bind(
                payload
                    .get("sort_order")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0) as i32,
            )
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx)?;

            // A floor proposed with rooms is approved with them, in this same
            // transaction -- approving the floor and leaving the manager to
            // re-request every room would defeat the point of one approval.
            let branch_id = uuid_field("branch_id")?;
            for room in payload
                .get("rooms")
                .and_then(serde_json::Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
            {
                let number = room
                    .get("number")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("room number is required".into()))?;
                let cooling = room
                    .get("cooling_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("non_ac");
                let room_id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO rooms (id, org_id, branch_id, number, cooling_type, floor_id) \
                     VALUES ($1,$2,$3,$4,$5,$6)",
                )
                .bind(room_id)
                .bind(org_id)
                .bind(branch_id)
                .bind(number.trim())
                .bind(cooling)
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(map_sqlx)?;

                let beds = room
                    .get("beds")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                for index in 0..beds {
                    let label = char::from_u32(u32::from(b'A') + index as u32)
                        .ok_or_else(|| {
                            AppError::Validation("a room cannot have more than 26 beds".into())
                        })?
                        .to_string();
                    sqlx::query(
                        "INSERT INTO beds (id, org_id, room_id, label) VALUES ($1,$2,$3,$4)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(org_id)
                    .bind(room_id)
                    .bind(&label)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
                }
            }
        }
        "room" => {
            sqlx::query(
                "INSERT INTO rooms (id, org_id, branch_id, number, cooling_type, floor_id) \
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(id)
            .bind(org_id)
            .bind(uuid_field("branch_id")?)
            .bind(required("number")?)
            .bind(text("cooling_type").unwrap_or("non_ac"))
            .bind(
                text("floor_id")
                    .map(|v| {
                        Uuid::parse_str(v)
                            .map_err(|_| AppError::Validation("floor_id must be a uuid".into()))
                    })
                    .transpose()?,
            )
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx)?;

            // An approved room arrives with the beds it was proposed with,
            // otherwise the manager would have to request each bed separately
            // after the room finally appeared.
            let beds = payload
                .get("beds")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            for index in 0..beds {
                let label = char::from_u32(u32::from(b'A') + index as u32)
                    .ok_or_else(|| {
                        AppError::Validation("a room cannot have more than 26 beds".into())
                    })?
                    .to_string();
                sqlx::query("INSERT INTO beds (id, org_id, room_id, label) VALUES ($1,$2,$3,$4)")
                    .bind(Uuid::new_v4())
                    .bind(org_id)
                    .bind(id)
                    .bind(&label)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
        }
        "bed" => {
            sqlx::query("INSERT INTO beds (id, org_id, room_id, label) VALUES ($1,$2,$3,$4)")
                .bind(id)
                .bind(org_id)
                .bind(uuid_field("room_id")?)
                .bind(required("label")?)
                .execute(&mut *conn)
                .await
                .map_err(map_sqlx)?;
        }
        other => {
            return Err(AppError::Validation(format!(
                "{other} cannot be created through approval"
            )))
        }
    }
    Ok(())
}

async fn apply_update(
    conn: &mut PgConnection,
    entity_type: &str,
    id: Uuid,
    org_id: Uuid,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let set = |col: &str| {
        // fixed column names from our own match; value is bound
        format!(
            "UPDATE {t} SET {col} = $1 WHERE id = $2 AND org_id = $3 AND deleted_at IS NULL",
            t = table_for(entity_type).unwrap_or("__none")
        )
    };
    match entity_type {
        "room" => {
            if let Some(v) = payload.get("number").and_then(|x| x.as_str()) {
                sqlx::query(&set("number"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
            if let Some(v) = payload.get("cooling_type").and_then(|x| x.as_str()) {
                sqlx::query(&set("cooling_type"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
            // Explicit null moves the room off its floor; absent leaves it be.
            if let Some(v) = payload.get("floor_id") {
                let floor: Option<Uuid> = match v {
                    serde_json::Value::Null => None,
                    other => Some(
                        other
                            .as_str()
                            .and_then(|s| Uuid::parse_str(s).ok())
                            .ok_or_else(|| {
                                AppError::Validation("floor_id must be a uuid or null".into())
                            })?,
                    ),
                };
                sqlx::query(&set("floor_id"))
                    .bind(floor)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
        }
        "floor" => {
            if let Some(v) = payload.get("name").and_then(|x| x.as_str()) {
                sqlx::query(&set("name"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
            if let Some(v) = payload
                .get("sort_order")
                .and_then(serde_json::Value::as_i64)
            {
                sqlx::query(&set("sort_order"))
                    .bind(v as i32)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
        }
        "branch" => {
            if let Some(v) = payload.get("name").and_then(|x| x.as_str()) {
                sqlx::query(&set("name"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
            if let Some(v) = payload.get("address").and_then(|x| x.as_str()) {
                sqlx::query(&set("address"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
        }
        "resident" => {
            if let Some(v) = payload.get("full_name").and_then(|x| x.as_str()) {
                sqlx::query(&set("full_name"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
            if let Some(v) = payload.get("phone").and_then(|x| x.as_str()) {
                sqlx::query(&set("phone"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
            if let Some(v) = payload.get("plan_kind").and_then(|x| x.as_str()) {
                if pg_domain::PlanKind::parse(v).is_none() {
                    return Err(AppError::Validation("invalid plan_kind".into()));
                }
                sqlx::query(&set("plan_kind"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
            // Rent & deposit revisions pass through maker-checker (annual
            // revision is routine; an unapproved rent change is a dispute
            // generator, so a manager's edit waits like any other).
            if let Some(v) = payload.get("rent_paise").and_then(|x| x.as_i64()) {
                if v < 0 {
                    return Err(AppError::Validation("rent_paise cannot be negative".into()));
                }
                sqlx::query(&set("rent_paise"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
            if let Some(v) = payload.get("deposit_paise").and_then(|x| x.as_i64()) {
                if v < 0 {
                    return Err(AppError::Validation(
                        "deposit_paise cannot be negative".into(),
                    ));
                }
                sqlx::query(&set("deposit_paise"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
        }
        "bed" => {
            if let Some(v) = payload.get("label").and_then(|x| x.as_str()) {
                sqlx::query(&set("label"))
                    .bind(v)
                    .bind(id)
                    .bind(org_id)
                    .execute(&mut *conn)
                    .await
                    .map_err(map_sqlx)?;
            }
        }
        other => return Err(AppError::Validation(format!("unsupported entity: {other}"))),
    }
    Ok(())
}

async fn apply_change(
    conn: &mut PgConnection,
    org_id: Uuid,
    target: &ChangeTarget,
    actor: &AuditActor,
) -> Result<(), AppError> {
    let table = table_for(&target.entity_type)?;

    // A create has nothing to read first; update and delete must find their row.
    let before = if target.operation == "create" {
        None
    } else {
        Some(
            select_jsonb(&mut *conn, table, target.entity_id, org_id)
                .await?
                .ok_or(AppError::NotFound)?,
        )
    };

    match target.operation.as_str() {
        "create" => {
            apply_create(
                &mut *conn,
                &target.entity_type,
                target.entity_id,
                org_id,
                &target.payload,
            )
            .await?;
        }
        "update" => {
            apply_update(
                &mut *conn,
                &target.entity_type,
                target.entity_id,
                org_id,
                &target.payload,
            )
            .await?;
        }
        "delete" => {
            // A floor may not be removed while rooms still point at it. The
            // repository enforces this for an owner's direct delete; without
            // the same check here, approving a manager's request would orphan
            // the rooms instead.
            if target.entity_type == "floor" {
                let occupied: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM rooms \
                     WHERE org_id = $1 AND floor_id = $2 AND deleted_at IS NULL",
                )
                .bind(org_id)
                .bind(target.entity_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(map_sqlx)?;
                if occupied > 0 {
                    return Err(AppError::Validation(format!(
                        "move or delete the {occupied} room(s) on this floor first"
                    )));
                }
            }
            let sql = format!(
                "UPDATE {table} SET deleted_at = now() WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL"
            );
            sqlx::query(&sql)
                .bind(target.entity_id)
                .bind(org_id)
                .execute(&mut *conn)
                .await
                .map_err(map_sqlx)?;
        }
        other => {
            return Err(AppError::Validation(format!(
                "unsupported operation: {other}"
            )))
        }
    }

    let after = if target.operation == "delete" {
        None
    } else {
        select_jsonb(&mut *conn, table, target.entity_id, org_id).await?
    };
    write_audit(
        &mut *conn,
        org_id,
        actor,
        &target.operation,
        &target.entity_type,
        target.entity_id,
        before,
        after,
    )
    .await?;
    Ok(())
}

fn format_change_description(
    entity_type: &str,
    operation: &str,
    entity_name: Option<&str>,
    proposed: Option<&serde_json::Value>,
    target_floor_name: Option<&str>,
) -> String {
    let name_str = entity_name.unwrap_or("");
    match operation {
        "delete" => {
            if name_str.is_empty() {
                format!("Delete {entity_type}")
            } else {
                format!("Delete {entity_type} {name_str}")
            }
        }
        "create" => {
            if name_str.is_empty() {
                format!("Create {entity_type}")
            } else {
                format!("Create {entity_type} {name_str}")
            }
        }
        "update" => {
            if let Some(payload) = proposed {
                let mut parts = Vec::new();
                if let Some(rent) = payload.get("rent_paise").and_then(|v| v.as_i64()) {
                    parts.push(format!("Revise rent to ₹{}", rent / 100));
                }
                if let Some(dep) = payload.get("deposit_paise").and_then(|v| v.as_i64()) {
                    parts.push(format!("Revise deposit to ₹{}", dep / 100));
                }
                if let Some(phone) = payload.get("phone").and_then(|v| v.as_str()) {
                    parts.push(format!("Update phone to {phone}"));
                }
                if let Some(plan) = payload.get("plan_kind").and_then(|v| v.as_str()) {
                    parts.push(format!("Change plan to {plan}"));
                }
                if let Some(cooling) = payload.get("cooling_type").and_then(|v| v.as_str()) {
                    parts.push(format!("Change cooling to {cooling}"));
                }
                if payload.get("floor_id").is_some() {
                    if let Some(t_floor) = target_floor_name {
                        parts.push(format!("Move to {t_floor}"));
                    } else {
                        parts.push("Move to another floor".to_string());
                    }
                }
                if let Some(full_name) = payload.get("full_name").and_then(|v| v.as_str()) {
                    parts.push(format!("Rename to {full_name}"));
                }
                if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                    parts.push(format!("Rename to {name}"));
                }
                if let Some(label) = payload.get("label").and_then(|v| v.as_str()) {
                    parts.push(format!("Rename to {label}"));
                }
                if !parts.is_empty() {
                    return parts.join(", ");
                }
            }
            if name_str.is_empty() {
                format!("Update {entity_type}")
            } else {
                format!("Update {entity_type} {name_str}")
            }
        }
        _ => format!("{operation} {entity_type}"),
    }
}

#[async_trait]
impl ChangeRequestRepository for PgChangeRequestRepository {
    async fn apply_now(
        &self,
        ctx: &TenantContext,
        target: &ChangeTarget,
        actor: &AuditActor,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        apply_change(&mut tx, org, target, actor).await?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn create_pending(
        &self,
        ctx: &TenantContext,
        target: &ChangeTarget,
        requested_by: UserId,
    ) -> Result<Uuid, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO change_requests (id, org_id, entity_type, entity_id, operation, proposed, requested_by) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(org)
        .bind(&target.entity_type)
        .bind(target.entity_id)
        .bind(&target.operation)
        .bind(&target.payload)
        .bind(requested_by.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(id)
    }

    async fn list_changes(
        &self,
        ctx: &TenantContext,
        status: Option<&str>,
    ) -> Result<Vec<ChangeRequestSummary>, AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let rows: Vec<(
            Uuid,
            String,
            Uuid,
            String,
            String,
            Uuid,
            Option<serde_json::Value>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<Uuid>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT \
                cr.id, \
                cr.entity_type, \
                cr.entity_id, \
                cr.operation, \
                cr.status, \
                cr.requested_by, \
                cr.proposed, \
                u.full_name AS requested_by_name, \
                CASE cr.entity_type \
                    WHEN 'resident' THEN COALESCE(cr.proposed->>'full_name', res.full_name) \
                    WHEN 'floor'    THEN COALESCE(cr.proposed->>'name', fl.name) \
                    WHEN 'room'     THEN COALESCE(cr.proposed->>'number', rm.number) \
                    WHEN 'bed'      THEN COALESCE(cr.proposed->>'label', b.label) \
                    WHEN 'branch'   THEN COALESCE(cr.proposed->>'name', br.name) \
                    ELSE NULL \
                END AS entity_name, \
                target_fl.name AS target_floor_name, \
                to_char(cr.created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, \
                to_char(cr.decided_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS decided_at, \
                cr.decided_by, \
                decider.full_name AS decided_by_name \
             FROM change_requests cr \
             LEFT JOIN public.platform_users u ON u.id = cr.requested_by \
             LEFT JOIN public.platform_users decider ON decider.id = cr.decided_by \
             LEFT JOIN residents res ON cr.entity_type = 'resident' AND res.id = cr.entity_id \
             LEFT JOIN floors fl ON cr.entity_type = 'floor' AND fl.id = cr.entity_id \
             LEFT JOIN rooms rm ON cr.entity_type = 'room' AND rm.id = cr.entity_id \
             LEFT JOIN beds b ON cr.entity_type = 'bed' AND b.id = cr.entity_id \
             LEFT JOIN branches br ON cr.entity_type = 'branch' AND br.id = cr.entity_id \
             LEFT JOIN floors target_fl ON target_fl.id = (CASE WHEN cr.proposed->>'floor_id' ~ '^[0-9a-fA-F-]{36}$' THEN (cr.proposed->>'floor_id')::uuid ELSE NULL END) \
             WHERE cr.org_id = $1 AND ($2::text IS NULL OR $2 = 'all' OR cr.status = $2) \
             ORDER BY cr.created_at DESC LIMIT 500",
        )
        .bind(org)
        .bind(status)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    entity_type,
                    entity_id,
                    operation,
                    status,
                    requested_by,
                    proposed,
                    requested_by_name,
                    entity_name,
                    target_floor_name,
                    created_at,
                    decided_at,
                    decided_by,
                    decided_by_name,
                )| {
                    let description = format_change_description(
                        &entity_type,
                        &operation,
                        entity_name.as_deref(),
                        proposed.as_ref(),
                        target_floor_name.as_deref(),
                    );
                    ChangeRequestSummary {
                        id,
                        entity_type,
                        entity_id,
                        entity_name,
                        operation,
                        status,
                        requested_by,
                        requested_by_name,
                        proposed,
                        description: Some(description),
                        created_at,
                        decided_at,
                        decided_by,
                        decided_by_name,
                    }
                },
            )
            .collect())
    }

    async fn approve(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        decided_by: UserId,
        request_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let row: Option<(String, Uuid, String, serde_json::Value, String)> = sqlx::query_as(
            "SELECT entity_type, entity_id, operation, proposed, status \
             FROM change_requests WHERE id = $1 AND org_id = $2 FOR UPDATE",
        )
        .bind(id)
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let (entity_type, entity_id, operation, payload, status) = row.ok_or(AppError::NotFound)?;
        if status != "pending" {
            return Err(AppError::Conflict("change request already decided".into()));
        }
        let target = ChangeTarget {
            entity_type,
            entity_id,
            operation,
            payload,
        };
        let actor = AuditActor {
            user_id: Some(decided_by),
            request_id,
        };
        apply_change(&mut tx, org, &target, &actor).await?;
        sqlx::query(
            "UPDATE change_requests SET status = 'approved', decided_by = $1, decided_at = now() \
             WHERE id = $2 AND org_id = $3",
        )
        .bind(decided_by.as_uuid())
        .bind(id)
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn reject(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        decided_by: UserId,
    ) -> Result<(), AppError> {
        let org = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let res = sqlx::query(
            "UPDATE change_requests SET status = 'rejected', decided_by = $1, decided_at = now() \
             WHERE id = $2 AND org_id = $3 AND status = 'pending'",
        )
        .bind(decided_by.as_uuid())
        .bind(id)
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
}
