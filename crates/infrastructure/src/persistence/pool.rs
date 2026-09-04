use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

/// HTTP traffic pool. Background jobs open their own pools via
/// `connect_worker` so a stuck sweep cannot starve user requests.
pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Narrow pool for background workers (vacancy sweep, archive). Small so a
/// stuck org sweep can't wedge the whole scheduler; HTTP keeps the full pool.
pub async fn connect_worker(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Startup migrations use session-level advisory locks and therefore must not
/// share the transaction-pooler configuration used by request traffic.
pub async fn connect_migrations(database_url: &str) -> anyhow::Result<PgPool> {
    Ok(PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(15))
        .connect(database_url)
        .await?)
}
