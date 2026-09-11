//! Minimal, idempotent migration runner.
//!
//! Phase 0 scope: build the control-plane objects in `public`, ensure the
//! shared `pool` schema exists, and apply the tenant table set into it. The SQL
//! uses `IF NOT EXISTS`, so re-running is safe. Iterating siloed schemas and
//! version bookkeeping land alongside org provisioning in later phases.
use sqlx::PgPool;
use tracing::info;

const CONTROL_PLANE: &str = include_str!("../../../../migrations/control_plane/0001_init.sql");
const EMAIL_AUTH: &str = include_str!("../../../../migrations/control_plane/0002_email_auth.sql");
const TENANT_INVITATIONS: &str =
    include_str!("../../../../migrations/control_plane/0003_tenant_invitations.sql");
const MEMBERSHIP_STATUS: &str =
    include_str!("../../../../migrations/control_plane/0004_membership_status.sql");
const INVITATION_CONSTRAINTS: &str =
    include_str!("../../../../migrations/control_plane/0005_invitation_constraints.sql");
const INVITATION_DELIVERY: &str =
    include_str!("../../../../migrations/control_plane/0006_invitation_delivery.sql");
const MEMBERSHIP_IDENTIFIERS: &str =
    include_str!("../../../../migrations/control_plane/0007_membership_identifiers.sql");
const INVITATION_KYC: &str =
    include_str!("../../../../migrations/control_plane/0008_invitation_kyc.sql");
const PLATFORM_ORGANIZATION: &str =
    include_str!("../../../../migrations/control_plane/0009_platform_organization.sql");
const INVITATION_PLAN_KIND: &str =
    include_str!("../../../../migrations/control_plane/0010_invitation_plan_kind.sql");
const INVITATION_DAILY_PLAN: &str =
    include_str!("../../../../migrations/control_plane/0011_invitation_daily_plan.sql");
const MANAGER_MULTI_BRANCH: &str =
    include_str!("../../../../migrations/control_plane/0012_manager_multi_branch.sql");
const TENANT: &str = include_str!("../../../../migrations/tenant/0001_init.sql");
const TENANT_MEMBERSHIP_LINKS: &str =
    include_str!("../../../../migrations/tenant/0002_membership_links.sql");
const TENANT_INVOICE_NUMBERS: &str =
    include_str!("../../../../migrations/tenant/0003_invoice_numbers.sql");
const TENANT_FLOORS: &str = include_str!("../../../../migrations/tenant/0005_floors.sql");
const TENANT_INVOICE_SEQ_PER_ORG: &str =
    include_str!("../../../../migrations/tenant/0006_invoice_sequences_per_org.sql");

pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
    // Serialize concurrent boots on ONE pinned (session-level) connection: two
    // instances racing the same transactional DDL can deadlock/dupe-apply
    // without a lock, and pg_advisory_lock is session-scoped, so lock and
    // unlock must use the SAME connection. Record the applied version so the
    // run is idempotent and visible in the DB.
    let mut conn = pool.acquire().await?;
    sqlx::query("SELECT pg_advisory_lock(727272)")
        .execute(&mut *conn)
        .await?;
    let result = run_locked(&mut conn).await;
    sqlx::query("SELECT pg_advisory_unlock(727272)")
        .execute(&mut *conn)
        .await?;
    result
}

async fn run_locked(conn: &mut sqlx::PgConnection) -> anyhow::Result<()> {
    // Existing installations used idempotent DDL at every boot. Treat the
    // recorded baseline as authoritative instead: this avoids repeatedly
    // recreating indexes and RLS policies, and lets a Supabase migration mark
    // an already-provisioned database as compatible before the server starts.
    let has_bookkeeping: bool =
        sqlx::query_scalar("SELECT to_regclass('public.schema_migrations_tenant') IS NOT NULL")
            .fetch_one(&mut *conn)
            .await?;

    if !has_bookkeeping || !migration_applied(conn, "public").await? {
        info!("applying control-plane migrations (public)");
        sqlx::raw_sql(CONTROL_PLANE).execute(&mut *conn).await?;
        record_migration(conn, "public").await?;
    }
    if !migration_version_applied(conn, "public", "0002_email_auth").await? {
        info!("applying email authentication migration");
        sqlx::raw_sql(EMAIL_AUTH).execute(&mut *conn).await?;
        record_migration_version(conn, "public", "0002_email_auth").await?;
    }
    if !migration_version_applied(conn, "public", "0003_tenant_invitations").await? {
        info!("applying tenant invitation migration");
        sqlx::raw_sql(TENANT_INVITATIONS)
            .execute(&mut *conn)
            .await?;
        record_migration_version(conn, "public", "0003_tenant_invitations").await?;
    }
    if !migration_version_applied(conn, "public", "0004_membership_status").await? {
        info!("applying membership status migration");
        sqlx::raw_sql(MEMBERSHIP_STATUS).execute(&mut *conn).await?;
        record_migration_version(conn, "public", "0004_membership_status").await?;
    }
    if !migration_version_applied(conn, "public", "0005_invitation_constraints").await? {
        info!("applying tenant invitation constraints migration");
        sqlx::raw_sql(INVITATION_CONSTRAINTS)
            .execute(&mut *conn)
            .await?;
        record_migration_version(conn, "public", "0005_invitation_constraints").await?;
    }
    if !migration_version_applied(conn, "public", "0006_invitation_delivery").await? {
        info!("applying tenant invitation delivery migration");
        sqlx::raw_sql(INVITATION_DELIVERY)
            .execute(&mut *conn)
            .await?;
        record_migration_version(conn, "public", "0006_invitation_delivery").await?;
    }
    if !migration_version_applied(conn, "public", "0007_membership_identifiers").await? {
        info!("applying membership identifier migration");
        sqlx::raw_sql(MEMBERSHIP_IDENTIFIERS)
            .execute(&mut *conn)
            .await?;
        record_migration_version(conn, "public", "0007_membership_identifiers").await?;
    }
    if !migration_version_applied(conn, "public", "0008_invitation_kyc").await? {
        info!("applying invitation identity verification migration");
        sqlx::raw_sql(INVITATION_KYC).execute(&mut *conn).await?;
        record_migration_version(conn, "public", "0008_invitation_kyc").await?;
    }
    if !migration_version_applied(conn, "public", "0009_platform_organization").await? {
        info!("seeding platform organization");
        sqlx::raw_sql(PLATFORM_ORGANIZATION)
            .execute(&mut *conn)
            .await?;
        record_migration_version(conn, "public", "0009_platform_organization").await?;
    }
    if !migration_version_applied(conn, "public", "0010_invitation_plan_kind").await? {
        info!("applying invitation plan-kind migration");
        sqlx::raw_sql(INVITATION_PLAN_KIND)
            .execute(&mut *conn)
            .await?;
        record_migration_version(conn, "public", "0010_invitation_plan_kind").await?;
    }
    if !migration_version_applied(conn, "public", "0011_invitation_daily_plan").await? {
        info!("restoring daily invitation plan kind");
        sqlx::raw_sql(INVITATION_DAILY_PLAN)
            .execute(&mut *conn)
            .await?;
        record_migration_version(conn, "public", "0011_invitation_daily_plan").await?;
    }
    if !migration_version_applied(conn, "public", "0012_manager_multi_branch").await? {
        info!("applying manager multi-branch migration");
        sqlx::raw_sql(MANAGER_MULTI_BRANCH)
            .execute(&mut *conn)
            .await?;
        record_migration_version(conn, "public", "0012_manager_multi_branch").await?;
    }

    if !migration_applied(conn, "pool").await? {
        info!("applying pooled tenant schema and tables (pool)");
        sqlx::raw_sql("CREATE SCHEMA IF NOT EXISTS pool;")
            .execute(&mut *conn)
            .await?;
        // Apply the tenant DDL with search_path pointed at `pool` so objects
        // are created there. This is a pinned connection under the advisory
        // lock, so the session setting cannot affect another request.
        sqlx::query("SET search_path TO pool, public, extensions")
            .execute(&mut *conn)
            .await?;
        sqlx::raw_sql(TENANT).execute(&mut *conn).await?;
        record_migration(conn, "pool").await?;
    }

    apply_tenant_migration(
        conn,
        "pool",
        "0002_membership_links",
        TENANT_MEMBERSHIP_LINKS,
    )
    .await?;
    apply_tenant_migration(conn, "pool", "0003_invoice_numbers", TENANT_INVOICE_NUMBERS).await?;
    apply_tenant_migration(conn, "pool", "0005_floors", TENANT_FLOORS).await?;
    apply_tenant_migration(
        conn,
        "pool",
        "0006_invoice_seq_per_org",
        TENANT_INVOICE_SEQ_PER_ORG,
    )
    .await?;
    let silo_schemas: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT schema_name FROM public.tenant_placement \
         WHERE mode = 'siloed' AND schema_name IS NOT NULL AND schema_name <> 'pool'",
    )
    .fetch_all(&mut *conn)
    .await?;
    for schema in silo_schemas {
        apply_tenant_migration(
            conn,
            &schema,
            "0002_membership_links",
            TENANT_MEMBERSHIP_LINKS,
        )
        .await?;
        apply_tenant_migration(
            conn,
            &schema,
            "0003_invoice_numbers",
            TENANT_INVOICE_NUMBERS,
        )
        .await?;
        apply_tenant_migration(conn, &schema, "0005_floors", TENANT_FLOORS).await?;
        apply_tenant_migration(
            conn,
            &schema,
            "0006_invoice_seq_per_org",
            TENANT_INVOICE_SEQ_PER_ORG,
        )
        .await?;
    }
    sqlx::query("SET search_path TO public")
        .execute(&mut *conn)
        .await?;

    info!("migrations complete");
    Ok(())
}

async fn apply_tenant_migration(
    conn: &mut sqlx::PgConnection,
    schema: &str,
    version: &str,
    sql: &str,
) -> anyhow::Result<()> {
    if migration_version_applied(conn, schema, version).await? {
        return Ok(());
    }
    if !schema
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        anyhow::bail!("unsafe tenant schema name in routing table");
    }
    info!(schema, version, "applying tenant schema migration");
    let quoted_schema = format!("\"{}\"", schema.replace('"', "\"\""));
    sqlx::query(&format!("SET search_path TO {quoted_schema}"))
        .execute(&mut *conn)
        .await?;
    sqlx::raw_sql(sql).execute(&mut *conn).await?;
    record_migration_version(conn, schema, version).await
}

async fn migration_applied(conn: &mut sqlx::PgConnection, schema: &str) -> anyhow::Result<bool> {
    migration_version_applied(conn, schema, "0001_init").await
}

async fn migration_version_applied(
    conn: &mut sqlx::PgConnection,
    schema: &str,
    version: &str,
) -> anyhow::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM public.schema_migrations_tenant WHERE schema_name = $1 AND version = $2)",
    )
    .bind(schema)
    .bind(version)
    .fetch_one(&mut *conn)
    .await
    .map_err(Into::into)
}

async fn record_migration(conn: &mut sqlx::PgConnection, schema: &str) -> anyhow::Result<()> {
    record_migration_version(conn, schema, "0001_init").await
}

async fn record_migration_version(
    conn: &mut sqlx::PgConnection,
    schema: &str,
    version: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO public.schema_migrations_tenant (schema_name, version) \
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(schema)
    .bind(version)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
