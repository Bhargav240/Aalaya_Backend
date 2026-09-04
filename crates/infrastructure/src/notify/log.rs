//! Notifier adapter that logs. Swap for push/email/in-app later.
use async_trait::async_trait;
use pg_application::ports::notifier::Notifier;
use pg_application::AppError;
use pg_domain::{BranchId, OrgId};

pub struct LogNotifier;

#[async_trait]
impl Notifier for LogNotifier {
    async fn notify(
        &self,
        org_id: OrgId,
        branch_id: Option<BranchId>,
        title: &str,
    ) -> Result<(), AppError> {
        tracing::info!(
            %org_id,
            branch = ?branch_id.map(|b| b.as_uuid()),
            title,
            "notice published"
        );
        Ok(())
    }
}
