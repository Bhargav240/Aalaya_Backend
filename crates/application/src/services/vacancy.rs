//! Live vacancy projector. Derives each live listing's beds/free-beds/cheapest
//! price from the real tenant bed data and writes the snapshot to the control
//! plane, so anonymous marketplace reads never hit tenant schemas at query
//! time. Runs on a timer in the server binary; invoked per-org by the caller.
use crate::error::AppError;
use crate::ports::repositories::VacancyRepository;
use crate::ports::tenancy::TenantResolver;
use tracing::{info, warn};

/// One projector sweep: refresh the vacancy snapshot for every live listing's
/// branch, from its own tenant schema. Failures mark the snapshot stale rather
/// than fail the sweep (so one broken org doesn't stall the others).
pub async fn refresh_all(
    vacancy_repo: &dyn VacancyRepository,
    resolver: &dyn TenantResolver,
) -> Result<(), AppError> {
    let live = vacancy_repo.live_branch_ids().await?;
    for (org_id, branch_id) in live {
        let ctx = match resolver.resolve(org_id).await {
            Ok(ctx) => ctx,
            Err(e) => {
                warn!(org = %org_id.as_uuid(), branch = %branch_id.as_uuid(), error = %e, "vacancy: resolve failed");
                let _ = vacancy_repo.mark_stale(branch_id).await;
                continue;
            }
        };
        match vacancy_repo.compute_for_branch(&ctx, branch_id).await {
            Ok((total, avail, cheapest)) => {
                if let Err(e) = vacancy_repo
                    .upsert_snapshot(org_id, branch_id, total, avail, cheapest)
                    .await
                {
                    warn!(branch = %branch_id.as_uuid(), error = %e, "vacancy: upsert failed");
                }
            }
            Err(e) => {
                warn!(branch = %branch_id.as_uuid(), error = %e, "vacancy: compute failed");
                let _ = vacancy_repo.mark_stale(branch_id).await;
            }
        }
    }
    info!("vacancy sweep done");
    Ok(())
}
