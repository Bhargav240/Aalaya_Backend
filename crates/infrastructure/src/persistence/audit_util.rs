//! Shared helpers for capturing before/after snapshots and appending audit rows
//! inside the SAME transaction as the mutation.
use crate::persistence::errmap::map_sqlx;
use pg_application::ports::repositories::AuditActor;
use pg_application::AppError;
use uuid::Uuid;

pub(crate) async fn select_jsonb<'e, E>(
    exec: E,
    table: &str,
    id: Uuid,
    org_id: Uuid,
) -> Result<Option<serde_json::Value>, AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    // `table` is always a fixed literal chosen by our own match, never user input.
    let v: Option<serde_json::Value> = sqlx::query_scalar(&format!(
        "SELECT to_jsonb(t) FROM {table} t WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL"
    ))
    .bind(id)
    .bind(org_id)
    .fetch_optional(exec)
    .await
    .map_err(map_sqlx)?;
    Ok(v)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn write_audit<'e, E>(
    exec: E,
    org_id: Uuid,
    actor: &AuditActor,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
) -> Result<(), AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query(
        "INSERT INTO audit_log (org_id, actor_id, action, entity_type, entity_id, before, after, request_id) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(org_id)
    .bind(actor.user_id.map(|u| u.as_uuid()))
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(before)
    .bind(after)
    .bind(actor.request_id)
    .execute(exec)
    .await
    .map_err(map_sqlx)?;
    Ok(())
}
