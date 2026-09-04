//! Notifier port: deliver notices (push / email / in-app). Log adapter for now.
use crate::error::AppError;
use async_trait::async_trait;
use pg_domain::{BranchId, OrgId};

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(
        &self,
        org_id: OrgId,
        branch_id: Option<BranchId>,
        title: &str,
    ) -> Result<(), AppError>;
}
