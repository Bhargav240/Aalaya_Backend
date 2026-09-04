//! Archive soft-deleted records to cold storage (ObjectStore), then purge them
//! from the hot DB. Manual API trigger for now (no scheduler).
use crate::authz;
use crate::error::AppError;
use crate::ports::clock::Clock;
use crate::ports::repositories::{ArchiveRepository, AuditActor};
use crate::ports::storage::ObjectStore;
use crate::ports::tenancy::TenantContext;
use pg_domain::Role;
use std::collections::BTreeMap;
use time::Duration;

const ARCHIVABLE: &[&str] = &[
    "branches",
    "rooms",
    "beds",
    "residents",
    "payments",
    "receipts",
    "complaints",
    "notices",
];

/// Dependencies and caller data for a single archive-and-purge operation.
pub struct ArchiveDeletedCommand<'a> {
    pub archive_repo: &'a dyn ArchiveRepository,
    pub store: &'a dyn ObjectStore,
    pub clock: &'a dyn Clock,
    pub ctx: &'a TenantContext,
    pub role: Role,
    pub actor: &'a AuditActor,
    pub older_than_days: i64,
}

pub async fn archive_deleted(
    command: ArchiveDeletedCommand<'_>,
) -> Result<BTreeMap<String, i64>, AppError> {
    if !authz::is_owner(command.role) {
        return Err(AppError::Forbidden);
    }
    let now = command.clock.now();
    let cutoff = now - Duration::days(command.older_than_days.max(0));
    let stamp = now.unix_timestamp();
    let mut summary = BTreeMap::new();

    for table in ARCHIVABLE {
        let (json, count) = command
            .archive_repo
            .fetch_deleted(command.ctx, table, cutoff)
            .await?;
        if count == 0 {
            continue;
        }
        // 1) write cold copy, 2) only then purge -- if the write fails we keep the data.
        let key = format!("archive/{}/{}/{}.json", command.ctx.org_id, table, stamp);
        let bytes = serde_json::to_vec(&json).map_err(|e| AppError::Internal(e.to_string()))?;
        command.store.put(&key, bytes, "application/json").await?;
        command
            .archive_repo
            .purge(command.ctx, table, cutoff, &key, count, command.actor)
            .await?;
        summary.insert((*table).to_string(), count);
    }
    Ok(summary)
}
