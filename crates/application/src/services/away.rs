//! Away days: residents notify the PG when their bed will be empty, so the
//! management doesn't worry. This is purely a safety notification — it does
//! NOT touch rent, dues, or the ledger in any way. Move-out (30-day notice,
//! deposit settlement) is the only thing that changes what a resident owes.
use crate::authz;
use crate::error::AppError;
use crate::ports::clock::Clock;
use crate::ports::repositories::{
    AuditActor, AwayPeriodDetail, AwayRepository, ResidentRepository, UserRepository,
};
use crate::ports::tenancy::TenantContext;
use pg_domain::{ResidentId, Role, UserId};
use time::Date;
use uuid::Uuid;

pub struct AwayPeriodInput {
    pub from_date: Date,
    pub to_date: Date,
    pub note: Option<String>,
}

/// Dependencies and caller data required to report an away period.
///
/// Keeping this operation's inputs named prevents dependencies and actor
/// context from being accidentally reordered at call sites.
pub struct ReportAwayCommand<'a> {
    pub away_repo: &'a dyn AwayRepository,
    pub resident_repo: &'a dyn ResidentRepository,
    pub clock: &'a dyn Clock,
    pub ctx: &'a TenantContext,
    pub user_id: UserId,
    pub actor: &'a AuditActor,
    pub input: AwayPeriodInput,
}

#[derive(Debug)]
pub struct AwayPeriodView {
    pub id: Uuid,
    pub from_date: Date,
    pub to_date: Date,
    pub note: Option<String>,
}

/// Cap on a single reported period (guard against accidental "forever away").
pub const MAX_PERIOD_DAYS: i64 = 60;

pub async fn report(command: ReportAwayCommand<'_>) -> Result<Uuid, AppError> {
    // Must be a tenant with a live resident row — owners/managers don't live
    // in beds and don't need to report away.
    let rid = command
        .resident_repo
        .resident_id_of_user(command.ctx, command.user_id)
        .await?
        .ok_or(AppError::Forbidden)?;

    validate_range(&command.input, command.clock.now().date())?;
    reject_overlap(
        command.away_repo,
        command.ctx,
        rid,
        &command.input,
        // A new period has no id yet, so nothing is excluded from the check.
        None,
    )
    .await?;

    let id = Uuid::new_v4();
    command
        .away_repo
        .create(
            command.ctx,
            id,
            rid,
            command.input.from_date,
            command.input.to_date,
            command.input.note.as_deref(),
            command.actor,
        )
        .await?;
    Ok(id)
}

/// The date rules a reported period must satisfy, shared by report and edit.
///
/// An edit has to re-run every one of these: a correction is just as capable
/// of producing a backwards range or a 90-day absence as the original report.
fn validate_range(input: &AwayPeriodInput, today: Date) -> Result<(), AppError> {
    if input.from_date > input.to_date {
        return Err(AppError::Validation(
            "return date must be on or after the start date".into(),
        ));
    }
    if input.to_date < today {
        return Err(AppError::Validation(
            "you can't report a period that has already ended".into(),
        ));
    }
    if (input.to_date - input.from_date).whole_days() + 1 > MAX_PERIOD_DAYS {
        return Err(AppError::Validation(format!(
            "a single away period can't exceed {MAX_PERIOD_DAYS} days"
        )));
    }
    Ok(())
}

/// Rejects a range that collides with another active period for the resident,
/// so the same night is never reported twice.
async fn reject_overlap(
    away_repo: &dyn AwayRepository,
    ctx: &TenantContext,
    rid: ResidentId,
    input: &AwayPeriodInput,
    exclude: Option<Uuid>,
) -> Result<(), AppError> {
    let existing = away_repo
        .overlaps(ctx, rid, input.from_date, input.to_date, exclude)
        .await?;
    if !existing.is_empty() {
        return Err(AppError::Conflict(
            "an active away period already overlaps this range".into(),
        ));
    }
    Ok(())
}

/// Corrects a period the resident already reported.
///
/// Editing rather than cancel-and-re-report keeps one audited row with a
/// before/after trail, so staff who already planned around the old dates can
/// see what changed instead of watching the period vanish and reappear.
pub async fn update(
    away_repo: &dyn AwayRepository,
    resident_repo: &dyn ResidentRepository,
    clock: &dyn Clock,
    ctx: &TenantContext,
    user_id: UserId,
    actor: &AuditActor,
    period_id: Uuid,
    input: AwayPeriodInput,
) -> Result<(), AppError> {
    let rid = resident_repo
        .resident_id_of_user(ctx, user_id)
        .await?
        .ok_or(AppError::Forbidden)?;

    validate_range(&input, clock.now().date())?;
    // Exclude the row being edited, otherwise it always overlaps itself.
    reject_overlap(away_repo, ctx, rid, &input, Some(period_id)).await?;

    let ok = away_repo
        .update(
            ctx,
            period_id,
            rid,
            input.from_date,
            input.to_date,
            input.note.as_deref(),
            actor,
        )
        .await?;
    if !ok {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn list_mine(
    away_repo: &dyn AwayRepository,
    resident_repo: &dyn ResidentRepository,
    ctx: &TenantContext,
    user_id: UserId,
) -> Result<Vec<AwayPeriodView>, AppError> {
    let rid = resident_repo
        .resident_id_of_user(ctx, user_id)
        .await?
        .ok_or(AppError::Forbidden)?;
    items_to_view(away_repo.list_active_for_resident(ctx, rid).await?)
}

/// Who is away right now, for the people responsible for the building.
///
/// The manager who actually walks the floor is the one who needs to know a bed
/// is deliberately empty, so this is not owner-only. They are scoped to the
/// branch they run, matching notices and finance -- an org-wide list would
/// leak residents of branches they have no authority over.
pub async fn list_all(
    away_repo: &dyn AwayRepository,
    user_repo: &dyn UserRepository,
    ctx: &TenantContext,
    role: Role,
    user_id: UserId,
) -> Result<Vec<AwayPeriodDetail>, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    let branch = if authz::is_owner(role) {
        None
    } else {
        // A manager with no branch assigned would otherwise fall through to
        // the org-wide query; None here must mean "owner", never "unscoped".
        Some(
            user_repo
                .branch_in_org(user_id, ctx.org_id)
                .await?
                .ok_or(AppError::Forbidden)?,
        )
    };
    away_repo.list_active_detailed(ctx, branch).await
}

pub async fn cancel(
    away_repo: &dyn AwayRepository,
    resident_repo: &dyn ResidentRepository,
    ctx: &TenantContext,
    user_id: UserId,
    actor: &AuditActor,
    period_id: Uuid,
) -> Result<(), AppError> {
    let rid = resident_repo
        .resident_id_of_user(ctx, user_id)
        .await?
        .ok_or(AppError::Forbidden)?;
    let ok = away_repo.cancel(ctx, period_id, rid, actor).await?;
    if !ok {
        return Err(AppError::NotFound);
    }
    Ok(())
}

fn items_to_view(
    items: Vec<crate::ports::repositories::AwayPeriod>,
) -> Result<Vec<AwayPeriodView>, AppError> {
    Ok(items
        .into_iter()
        .map(|p| AwayPeriodView {
            id: p.id,
            from_date: p.from_date,
            to_date: p.to_date,
            note: p.note,
        })
        .collect())
}
