use async_trait::async_trait;
use pg_domain::OrgId;
use thiserror::Error;

/// Where an organization's tenant data physically lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// Shared `pool` schema, isolated by `org_id` + RLS.
    Pooled,
    /// A dedicated `org_<id>` schema.
    Siloed { schema: String },
}

impl Placement {
    pub fn schema_name(&self) -> &str {
        match self {
            Placement::Pooled => "pool",
            Placement::Siloed { schema } => schema,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TenantContext {
    pub org_id: OrgId,
    pub placement: Placement,
}

#[derive(Debug, Error)]
pub enum TenancyError {
    #[error("unknown organization")]
    UnknownOrg,
    /// `tenant_placement.status` is not `active` (e.g. `suspended`) — the org
    /// exists but is administratively blocked from serving requests.
    #[error("organization is {0}")]
    Suspended(String),
    #[error("backend error: {0}")]
    Backend(String),
}

/// Resolves an org to its placement. The middleware calls this on every request
/// (behind a cache) to decide which schema to point `search_path` at.
#[async_trait]
pub trait TenantResolver: Send + Sync {
    async fn resolve(&self, org_id: OrgId) -> Result<TenantContext, TenancyError>;
    /// Drop any cached placement for an org (after onboarding / promotion).
    async fn invalidate(&self, org_id: OrgId);
}

/// Provisions physical tenant placement changes (e.g. promoting a pooled org
/// into its own dedicated schema and backfilling its data).
#[async_trait]
pub trait TenantProvisioner: Send + Sync {
    /// Promote a pooled org to a siloed schema. Returns the new schema name.
    async fn promote_to_silo(&self, org_id: OrgId) -> Result<String, crate::error::AppError>;
}
