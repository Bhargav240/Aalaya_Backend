//! Tenant resolver adapters. `DbTenantResolver` reads the routing table;
//! `CachedTenantResolver` decorates it with an in-memory cache so routing costs
//! no per-request DB round-trip. Both satisfy the same `TenantResolver` port.
use async_trait::async_trait;
use dashmap::DashMap;
use pg_application::ports::tenancy::{Placement, TenancyError, TenantContext, TenantResolver};
use pg_domain::OrgId;
use sqlx::PgPool;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

/// Tenant routing cache TTL. Promotion flips `tenant_placement` to `siloed`
/// and any caller holding a stale `Pooled` context would keep writing into the
/// pool. A 30s TTL gives a bounded post-promotion inconsistency window that
/// matches the promotion path's atomic-copy model.
const ROUTE_CACHE_TTL: Duration = Duration::seconds(30);

pub struct DbTenantResolver {
    pool: PgPool,
}

impl DbTenantResolver {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantResolver for DbTenantResolver {
    async fn resolve(&self, org_id: OrgId) -> Result<TenantContext, TenancyError> {
        let row: Option<(String, Option<String>, String)> = sqlx::query_as(
            "SELECT mode, schema_name, status FROM public.tenant_placement WHERE org_id = $1",
        )
        .bind(org_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TenancyError::Backend(e.to_string()))?;

        let (mode, schema_name, status) = row.ok_or(TenancyError::UnknownOrg)?;
        if status != "active" {
            return Err(TenancyError::Suspended(status));
        }
        let placement = match mode.as_str() {
            "siloed" => Placement::Siloed {
                schema: schema_name.unwrap_or_else(|| format!("org_{}", org_id.as_uuid().simple())),
            },
            _ => Placement::Pooled,
        };
        Ok(TenantContext { org_id, placement })
    }

    async fn invalidate(&self, _org_id: OrgId) {}
}

pub struct CachedTenantResolver {
    inner: Arc<dyn TenantResolver>,
    cache: DashMap<OrgId, (TenantContext, OffsetDateTime)>,
}

impl CachedTenantResolver {
    pub fn new(inner: Arc<dyn TenantResolver>) -> Self {
        Self {
            inner,
            cache: DashMap::new(),
        }
    }
}

#[async_trait]
impl TenantResolver for CachedTenantResolver {
    async fn resolve(&self, org_id: OrgId) -> Result<TenantContext, TenancyError> {
        if let Some(hit) = self.cache.get(&org_id) {
            let (ctx, cached_at) = hit.value();
            if OffsetDateTime::now_utc() - *cached_at < ROUTE_CACHE_TTL {
                return Ok(ctx.clone());
            }
        }
        let ctx = self.inner.resolve(org_id).await?;
        self.cache
            .insert(org_id, (ctx.clone(), OffsetDateTime::now_utc()));
        Ok(ctx)
    }

    async fn invalidate(&self, org_id: OrgId) {
        self.cache.remove(&org_id);
        self.inner.invalidate(org_id).await;
    }
}
