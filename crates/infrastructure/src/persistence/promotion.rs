//! Pool -> silo promotion. Creates the org's own schema (cloning each tenant
//! table's structure from `pool` via LIKE ... INCLUDING ALL), backfills the
//! org's rows, flips the placement to siloed, and purges the pool -- all in ONE
//! transaction (Postgres has transactional DDL), so a failure rolls back.
use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::tenancy::TenantProvisioner;
use pg_application::AppError;
use pg_domain::OrgId;
use sqlx::PgPool;

// uuid-PK tables (copied as-is). MUST stay in sync with every CREATE TABLE in
// migrations/tenant: a missing table is silently LOST on promotion.
const COPY_PLAIN: &[&str] = &[
    "branches",
    "rooms",
    "beds",
    "residents",
    "payments",
    "receipts",
    "change_requests",
    "complaints",
    "notices",
    "archive_manifest",
    "away_periods",
    "move_out_settlements",
];
// Tables scoped transitively via FK (no org_id of their own). Each entry is
// (table, sql_to_check_org_membership_via_owner_row_in_target_schema).
const COPY_FK_SCOPED: &[(&str, &str)] = &[(
    "move_out_deductions",
    "EXISTS (SELECT 1 FROM \"{schema}\".move_out_settlements s WHERE s.id = move_out_deductions.settlement_id AND s.org_id = $1)",
)];
// GENERATED ALWAYS identity tables (copy with OVERRIDING SYSTEM VALUE, then reset seq)
const COPY_IDENTITY: &[&str] = &["audit_log", "complaint_events"];

fn all_tables() -> Vec<&'static str> {
    COPY_PLAIN
        .iter()
        .chain(COPY_IDENTITY.iter())
        .copied()
        .collect()
}

pub struct PgTenantProvisioner {
    pool: PgPool,
}

impl PgTenantProvisioner {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantProvisioner for PgTenantProvisioner {
    async fn promote_to_silo(&self, org_id: OrgId) -> Result<String, AppError> {
        promote(&self.pool, org_id).await
    }
}

async fn promote(pool: &PgPool, org_id: OrgId) -> Result<String, AppError> {
    let org = org_id.as_uuid();
    let schema = format!("org_{}", org.simple());
    let mut tx = pool.begin().await.map_err(map_sqlx)?;

    let placement: Option<(String,)> =
        sqlx::query_as("SELECT mode FROM public.tenant_placement WHERE org_id = $1 FOR UPDATE")
            .bind(org)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx)?;
    match placement {
        None => return Err(AppError::NotFound),
        Some((mode,)) if mode == "siloed" => {
            return Err(AppError::Conflict("org is already siloed".into()))
        }
        _ => {}
    }

    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema}\""))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;

    // Scope for RLS-enforced reads/deletes on the pool (single-org silo needs no RLS).
    sqlx::query("SELECT set_config('app.current_org', $1, true)")
        .bind(org.to_string())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;

    // Clone each table's structure into the new schema.
    for t in all_tables() {
        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS \"{schema}\".{t} (LIKE pool.{t} INCLUDING ALL)"
        ))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
    }
    for (t, _) in COPY_FK_SCOPED {
        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS \"{schema}\".{t} (LIKE pool.{t} INCLUDING ALL)"
        ))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
    }

    // Backfill org-scoped tables.
    for t in COPY_PLAIN {
        sqlx::query(&format!(
            "INSERT INTO \"{schema}\".{t} SELECT * FROM pool.{t} WHERE org_id = $1"
        ))
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
    }
    // Backfill FK-scoped tables (walk the chain through the freshly-copied parent).
    for (t, via) in COPY_FK_SCOPED {
        let join = via.replace("{schema}", &schema);
        sqlx::query(&format!(
            "INSERT INTO \"{schema}\".{t} SELECT pool.{t}.* FROM pool.{t} WHERE {join}"
        ))
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
    }
    for t in COPY_IDENTITY {
        sqlx::query(&format!(
            "INSERT INTO \"{schema}\".{t} OVERRIDING SYSTEM VALUE SELECT * FROM pool.{t} WHERE org_id = $1"
        ))
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        // Advance the new identity sequence past the copied ids.
        sqlx::query(&format!(
            "SELECT setval(pg_get_serial_sequence('\"{schema}\".{t}', 'id'), \
             GREATEST((SELECT COALESCE(MAX(id), 0) FROM \"{schema}\".{t}), 1))"
        ))
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
    }

    sqlx::query(
        "UPDATE public.tenant_placement SET mode = 'siloed', schema_name = $2, updated_at = now() WHERE org_id = $1",
    )
    .bind(org)
    .bind(&schema)
    .execute(&mut *tx)
    .await
    .map_err(map_sqlx)?;

    for t in all_tables() {
        sqlx::query(&format!("DELETE FROM pool.{t} WHERE org_id = $1"))
            .bind(org)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
    }
    // And FK-scoped tables: delete their pool rows whose owner row was already deleted.
    for (t, via) in COPY_FK_SCOPED {
        let join = via.replace("{schema}", &schema);
        sqlx::query(&format!("DELETE FROM pool.{t} WHERE {join}"))
            .bind(org)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
    }

    tx.commit().await.map_err(map_sqlx)?;
    Ok(schema)
}
