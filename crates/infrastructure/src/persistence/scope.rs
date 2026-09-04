//! Tenant-scoped transactions. Points `search_path` at the tenant's schema and
//! sets `app.current_org` for RLS -- both via SET LOCAL (transaction-scoped),
//! so nothing leaks to the next request that reuses this pooled connection.
use crate::persistence::errmap::map_sqlx;
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use sqlx::{PgPool, Postgres, Transaction};

fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

pub async fn scoped_tx<'a>(
    pool: &'a PgPool,
    ctx: &TenantContext,
) -> Result<Transaction<'a, Postgres>, AppError> {
    let mut tx = pool.begin().await.map_err(map_sqlx)?;
    // Do NOT append `public` here: if a tenant table ever shares a name with a
    // control-plane table, the fallback to public would silently read/write
    // control-plane data. Tenant repos must stay fully inside the tenant schema.
    let set_path = format!(
        "SET LOCAL search_path TO {}",
        quote_ident(ctx.placement.schema_name())
    );
    sqlx::query(&set_path)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
    sqlx::query("SELECT set_config('app.current_org', $1, true)")
        .bind(ctx.org_id.as_uuid().to_string())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
    Ok(tx)
}
