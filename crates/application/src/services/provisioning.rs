//! Tenant placement operations (pool -> silo promotion).
use crate::authz;
use crate::error::AppError;
use crate::ports::tenancy::{TenantProvisioner, TenantResolver};
use pg_domain::{OrgId, Role};

/// Promote the caller's org from the shared pool into its own schema, backfill
/// its data, then invalidate the routing cache so new requests hit the silo.
pub async fn promote_org(
    provisioner: &dyn TenantProvisioner,
    resolver: &dyn TenantResolver,
    role: Role,
    org_id: OrgId,
) -> Result<String, AppError> {
    if !authz::is_owner(role) {
        return Err(AppError::Forbidden);
    }
    let schema = provisioner.promote_to_silo(org_id).await?;
    resolver.invalidate(org_id).await;
    Ok(schema)
}
