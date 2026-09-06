//! Core domain use cases: rooms/beds/residents CRUD and the maker-checker flow.
use crate::authz;
use crate::error::AppError;
use crate::ports::repositories::{
    AuditActor, AuditRepository, AuditSummary, BedRepository, BedSummary, BranchRepository,
    ChangeRequestRepository, ChangeRequestSummary, ChangeTarget, FloorRepository, NewRoomSpec,
    ResidentRepository, ResidentSummary, RoomRepository, RoomSummary,
};
use crate::ports::tenancy::TenantContext;
use pg_domain::{BedId, BranchId, FloorId, PlanKind, ResidentId, Role, RoomId, UserId};
use uuid::Uuid;

#[derive(Debug)]
pub enum MutationOutcome {
    Applied,
    Pending { change_request_id: Uuid },
}

// ---- Branches (org topology is owner-only) ----
pub async fn create_branch(
    repo: &dyn BranchRepository,
    ctx: &TenantContext,
    role: Role,
    actor: &AuditActor,
    name: String,
) -> Result<BranchId, AppError> {
    if role == Role::Owner || role == Role::Manager || !authz::is_operator(role) {
        return Err(AppError::Forbidden);
    }
    let id = BranchId::new(Uuid::new_v4());
    repo.create(ctx, id, &name, actor).await?;
    Ok(id)
}

/// Creates a ready-to-use branch with a uniform floor, room and bed layout.
/// The normal `create_branch` API remains available for system provisioning.
pub async fn create_branch_with_layout(
    branch_repo: &dyn BranchRepository,
    floor_repo: &dyn FloorRepository,
    ctx: &TenantContext,
    role: Role,
    actor: &AuditActor,
    name: String,
    floor_count: u16,
    rooms_per_floor: u16,
    beds_per_room: u16,
) -> Result<BranchId, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("branch name is required".into()));
    }
    if name.chars().count() > 100 {
        return Err(AppError::Validation("branch name is too long".into()));
    }
    if floor_count == 0 || floor_count > 100 {
        return Err(AppError::Validation(
            "floor_count must be between 1 and 100".into(),
        ));
    }
    if rooms_per_floor == 0 || rooms_per_floor > 100 {
        return Err(AppError::Validation(
            "rooms_per_floor must be between 1 and 100".into(),
        ));
    }
    if beds_per_room == 0 || beds_per_room > 26 {
        return Err(AppError::Validation(
            "beds_per_room must be between 1 and 26".into(),
        ));
    }

    let branch_id = create_branch(branch_repo, ctx, role, actor, name.to_string()).await?;
    for floor_number in 1..=floor_count {
        let rooms = (1..=rooms_per_floor)
            .map(|room_number| NewRoomSpec {
                number: format!("{floor_number}{room_number:02}"),
                cooling_type: "non_ac".into(),
                beds: beds_per_room,
            })
            .collect::<Vec<_>>();
        floor_repo
            .create(
                ctx,
                FloorId::new(Uuid::new_v4()),
                branch_id,
                &format!("Floor {floor_number}"),
                i32::from(floor_number),
                &rooms,
                actor,
            )
            .await?;
    }
    Ok(branch_id)
}

// ---- Rooms ----
pub async fn create_room(
    repo: &dyn RoomRepository,
    ctx: &TenantContext,
    role: Role,
    actor: &AuditActor,
    branch_id: BranchId,
    number: String,
    cooling_type: String,
    floor_id: Option<FloorId>,
    beds: u16,
) -> Result<Uuid, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    let cooling_type = cooling_type.trim().to_ascii_lowercase();
    if !matches!(cooling_type.as_str(), "ac" | "non_ac") {
        return Err(AppError::Validation(
            "cooling_type must be ac or non_ac".into(),
        ));
    }
    let id = RoomId::new(Uuid::new_v4());
    if beds > 26 {
        return Err(AppError::Validation(
            "a room cannot have more than 26 beds".into(),
        ));
    }
    repo.create(
        ctx,
        id,
        branch_id,
        &number,
        &cooling_type,
        floor_id,
        beds,
        actor,
    )
    .await?;
    Ok(id.as_uuid())
}

pub async fn list_rooms(
    repo: &dyn RoomRepository,
    ctx: &TenantContext,
    role: Role,
    branch_id: BranchId,
) -> Result<Vec<RoomSummary>, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    repo.list_by_branch(ctx, branch_id).await
}

// ---- Beds ----
pub async fn create_bed(
    repo: &dyn BedRepository,
    ctx: &TenantContext,
    role: Role,
    actor: &AuditActor,
    room_id: RoomId,
    label: String,
) -> Result<Uuid, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    let id = BedId::new(Uuid::new_v4());
    repo.create(ctx, id, room_id, &label, actor).await?;
    Ok(id.as_uuid())
}

pub async fn list_beds(
    repo: &dyn BedRepository,
    ctx: &TenantContext,
    role: Role,
    room_id: RoomId,
) -> Result<Vec<BedSummary>, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    repo.list_by_room(ctx, room_id).await
}

/// Every bed in a branch in one query — the payments/occupancy screens use
/// this instead of fanning out per-room. Same manage-role gate as list_rooms.
pub async fn list_beds_by_branch(
    repo: &dyn BedRepository,
    ctx: &TenantContext,
    role: Role,
    branch_id: BranchId,
) -> Result<Vec<BedSummary>, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    repo.list_by_branch(ctx, branch_id).await
}

/// Assign a resident to a bed (or clear with None). The bed must exist, sit in
/// the resident's branch, and be free (unless already held by this resident).
pub struct AssignBedCommand<'a> {
    pub resident_repo: &'a dyn ResidentRepository,
    pub bed_repo: &'a dyn BedRepository,
    pub room_repo: &'a dyn RoomRepository,
    pub ctx: &'a TenantContext,
    pub role: Role,
    pub actor: &'a AuditActor,
    pub resident_id: ResidentId,
    pub bed_id: Option<BedId>,
}
pub async fn assign_bed(command: AssignBedCommand<'_>) -> Result<(), AppError> {
    let AssignBedCommand {
        resident_repo,
        bed_repo,
        room_repo,
        ctx,
        role,
        actor,
        resident_id,
        bed_id,
    } = command;
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    let resident = resident_repo
        .get(ctx, resident_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if let Some(bed) = bed_id {
        let room = bed_repo
            .room_of(ctx, bed)
            .await?
            .ok_or(AppError::NotFound)?;
        let branch = room_repo
            .branch_of(ctx, room)
            .await?
            .ok_or(AppError::NotFound)?;
        if branch.as_uuid() != resident.branch_id {
            return Err(AppError::Validation(
                "bed belongs to a different branch".into(),
            ));
        }
        if let Some(occupant) = resident_repo.occupant_of_bed(ctx, bed).await? {
            if occupant != resident_id {
                return Err(AppError::Conflict("bed already occupied".into()));
            }
        }
    }
    resident_repo
        .assign_bed(ctx, resident_id, bed_id, actor)
        .await
}

// ---- Residents ----
/// A resident is registered by phone — that phone is how they'll sign in.
/// Registration IS the invitation: when a phone is present we immediately
/// create a platform_user and wire the tenant-side resident row to it, so
/// the resident can sign in with an SMS code right away (no invite codes, no
/// A resident row does not grant application access. Tenant authentication and
/// membership linking happen exclusively through the invitation workflow.
pub struct CreateResidentCommand<'a> {
    pub repo: &'a dyn ResidentRepository,
    pub ctx: &'a TenantContext,
    pub role: Role,
    pub actor: &'a AuditActor,
    pub branch_id: BranchId,
    pub full_name: String,
    pub phone: Option<String>,
    pub plan_kind: PlanKind,
    pub rent_paise: i64,
    pub deposit_paise: i64,
}
pub async fn create_resident(command: CreateResidentCommand<'_>) -> Result<Uuid, AppError> {
    let CreateResidentCommand {
        repo,
        ctx,
        role,
        actor,
        branch_id,
        full_name,
        phone,
        plan_kind,
        rent_paise,
        deposit_paise,
    } = command;
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    if full_name.trim().is_empty() {
        return Err(AppError::Validation("full_name required".into()));
    }

    let normalized_phone = match phone.as_deref() {
        Some(raw) => Some(
            crate::services::normalize_phone(raw)
                .ok_or_else(|| AppError::Validation("invalid phone".into()))?,
        ),
        None => None,
    };

    let id = ResidentId::new(Uuid::new_v4());
    repo.create(
        ctx,
        id,
        branch_id,
        &full_name,
        normalized_phone.as_deref(),
        plan_kind,
        rent_paise,
        deposit_paise,
        actor,
    )
    .await?;
    Ok(id.as_uuid())
}

pub async fn list_residents(
    repo: &dyn ResidentRepository,
    ctx: &TenantContext,
    role: Role,
    branch_id: BranchId,
) -> Result<Vec<ResidentSummary>, AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    repo.list_by_branch(ctx, branch_id).await
}

// ---- Maker-checker: edit/delete route through here ----
pub async fn edit_entity(
    cr: &dyn ChangeRequestRepository,
    ctx: &TenantContext,
    role: Role,
    actor: &AuditActor,
    requested_by: UserId,
    target: ChangeTarget,
) -> Result<MutationOutcome, AppError> {
    match role {
        Role::Owner => {
            cr.apply_now(ctx, &target, actor).await?;
            Ok(MutationOutcome::Applied)
        }
        Role::Manager => {
            let id = cr.create_pending(ctx, &target, requested_by).await?;
            Ok(MutationOutcome::Pending {
                change_request_id: id,
            })
        }
        Role::Tenant | Role::PlatformOperator | Role::Prospect => Err(AppError::Forbidden),
    }
}

pub async fn list_changes(
    cr: &dyn ChangeRequestRepository,
    ctx: &TenantContext,
    role: Role,
    status: Option<&str>,
) -> Result<Vec<ChangeRequestSummary>, AppError> {
    if !authz::is_owner(role) {
        return Err(AppError::Forbidden);
    }
    cr.list_changes(ctx, status).await
}

pub async fn list_pending_changes(
    cr: &dyn ChangeRequestRepository,
    ctx: &TenantContext,
    role: Role,
) -> Result<Vec<ChangeRequestSummary>, AppError> {
    list_changes(cr, ctx, role, Some("pending")).await
}

pub async fn approve_change(
    cr: &dyn ChangeRequestRepository,
    ctx: &TenantContext,
    role: Role,
    decided_by: UserId,
    request_id: Option<Uuid>,
    id: Uuid,
) -> Result<(), AppError> {
    if !authz::is_owner(role) {
        return Err(AppError::Forbidden);
    }
    cr.approve(ctx, id, decided_by, request_id).await
}

pub async fn reject_change(
    cr: &dyn ChangeRequestRepository,
    ctx: &TenantContext,
    role: Role,
    decided_by: UserId,
    id: Uuid,
) -> Result<(), AppError> {
    if !authz::is_owner(role) {
        return Err(AppError::Forbidden);
    }
    cr.reject(ctx, id, decided_by).await
}

pub async fn list_audit(
    audit: &dyn AuditRepository,
    ctx: &TenantContext,
    role: Role,
    limit: i64,
) -> Result<Vec<AuditSummary>, AppError> {
    if !authz::is_owner(role) {
        return Err(AppError::Forbidden);
    }
    audit.list_recent(ctx, limit).await
}

pub async fn link_resident_user(
    repo: &dyn crate::ports::repositories::ResidentRepository,
    ctx: &TenantContext,
    role: Role,
    actor: &AuditActor,
    resident_id: pg_domain::ResidentId,
    user_id: UserId,
) -> Result<(), AppError> {
    if !authz::can_manage(role) {
        return Err(AppError::Forbidden);
    }
    repo.link_user(ctx, resident_id, user_id, actor).await
}
