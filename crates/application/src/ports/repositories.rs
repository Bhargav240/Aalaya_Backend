//! One focused repository trait per aggregate (Interface Segregation).
//! Control-plane repos hit `public`; tenant repos take a `TenantContext`.
use crate::error::AppError;
use crate::ports::storage::ReceiptTemplate;
use crate::ports::tenancy::TenantContext;
use async_trait::async_trait;
use pg_domain::{
    BedId, BranchId, ComplaintId, FloorId, InvitationStatus, NoticeId, OrgId, Organization,
    PaymentId, PlanKind, ReceiptId, ResidentId, Role, RoomId, UserId, UserStatus,
};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct AuditActor {
    pub user_id: Option<UserId>,
    pub request_id: Option<Uuid>,
}

// ============ Control plane ============

#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    async fn create(&self, org: &Organization) -> Result<(), AppError>;
    async fn exists(&self, org_id: OrgId) -> Result<bool, AppError>;
    /// Every live org (archive scheduler / vacancy sweeps iterate these).
    async fn list_all_live(&self) -> Result<Vec<OrgId>, AppError>;
}

pub struct TenantInvitation {
    pub id: Uuid,
    pub org_id: OrgId,
    pub branch_id: BranchId,
    pub intended_room_id: Option<RoomId>,
    pub intended_bed_id: Option<BedId>,
    pub email: String,
    pub full_name: String,
    pub plan_kind: String,
    pub monthly_rent_paise: Option<i64>,
    pub deposit_paise: Option<i64>,
    pub planned_move_in_date: Option<time::Date>,
    pub require_identity_verification: bool,
    pub status: InvitationStatus,
    pub accepted_by_user_id: Option<UserId>,
    pub expires_at: OffsetDateTime,
    pub last_sent_at: OffsetDateTime,
    pub accepted_at: Option<OffsetDateTime>,
    pub approved_at: Option<OffsetDateTime>,
    pub approved_by_user_id: Option<UserId>,
}

#[async_trait]
pub trait TenantInvitationRepository: Send + Sync {
    async fn create(
        &self,
        invitation: &TenantInvitation,
        invited_by: UserId,
    ) -> Result<(), AppError>;
    async fn get(&self, id: Uuid) -> Result<Option<TenantInvitation>, AppError>;
    async fn list_for_org(&self, org_id: OrgId) -> Result<Vec<TenantInvitation>, AppError>;
    async fn list_for_email(&self, email: &str) -> Result<Vec<TenantInvitation>, AppError>;
    async fn accept(&self, id: Uuid, user_id: UserId, email: &str) -> Result<bool, AppError>;
    async fn decline(
        &self,
        id: Uuid,
        email: &str,
        from: InvitationStatus,
    ) -> Result<bool, AppError>;
    async fn transition(
        &self,
        id: Uuid,
        from: InvitationStatus,
        to: InvitationStatus,
        actor: UserId,
    ) -> Result<bool, AppError>;
    async fn resend(
        &self,
        id: Uuid,
        expected_status: InvitationStatus,
        expires_at: OffsetDateTime,
        not_sent_after: OffsetDateTime,
    ) -> Result<bool, AppError>;
}

pub struct ActivateTenantRecord {
    pub invitation_id: Uuid,
    pub approved_by_user_id: UserId,
    pub request_id: Option<Uuid>,
}

pub struct TenantActivationResult {
    pub membership_id: Uuid,
    pub resident_id: ResidentId,
}

/// Atomic boundary that spans the control plane and one routed tenant schema.
#[async_trait]
pub trait TenantActivationRepository: Send + Sync {
    async fn activate(
        &self,
        ctx: &TenantContext,
        record: &ActivateTenantRecord,
    ) -> Result<TenantActivationResult, AppError>;
}

pub struct LoginRecord {
    pub user_id: UserId,
    pub org_id: OrgId,
    pub role: Role,
    pub status: UserStatus,
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipContext {
    pub membership_id: Uuid,
    pub org_id: OrgId,
    pub organization_name: String,
    pub role: Role,
    pub branch_id: Option<BranchId>,
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Bootstrap an owner account by email. Phone is optional contact data.
    async fn create_active_owner(
        &self,
        id: UserId,
        org_id: OrgId,
        email: &str,
        phone: Option<&str>,
        full_name: &str,
    ) -> Result<(), AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<LoginRecord>, AppError>;
    async fn create_email_explorer(
        &self,
        id: UserId,
        platform_org_id: OrgId,
        email: &str,
    ) -> Result<LoginRecord, AppError>;
    async fn mark_email_verified(&self, user_id: UserId) -> Result<(), AppError>;
    async fn verified_email_of(&self, user_id: UserId) -> Result<Option<String>, AppError>;
    async fn find_by_google_sub(
        &self,
        sub: &str,
    ) -> Result<Option<(UserId, OrgId, Role)>, AppError>;
    async fn link_google_sub(&self, user_id: UserId, sub: &str) -> Result<(), AppError>;
    async fn auth_context(&self, user_id: UserId) -> Result<Option<(OrgId, Role)>, AppError>;
    async fn has_active_membership(
        &self,
        user_id: UserId,
        org_id: OrgId,
        role: Role,
    ) -> Result<bool, AppError>;
    /// Every active context the user may explicitly switch into. Pending and
    /// moved-out memberships are deliberately excluded from session issuance.
    async fn list_active_memberships(
        &self,
        user_id: UserId,
    ) -> Result<Vec<MembershipContext>, AppError>;
    /// The branch a user is scoped to (managers/tenants); None for org-wide roles.
    async fn branch_of(&self, user_id: UserId) -> Result<Option<BranchId>, AppError>;
    /// Branch assigned to this user in the selected organization. Multi-context
    /// users can be managers in more than one organization, so authorization
    /// must not pick an arbitrary branch from another active membership.
    async fn branch_in_org(
        &self,
        user_id: UserId,
        _org_id: OrgId,
    ) -> Result<Option<BranchId>, AppError> {
        self.branch_of(user_id).await
    }
    /// Branches explicitly assigned to this user in the selected organization.
    async fn branches_in_org(
        &self,
        user_id: UserId,
        org_id: OrgId,
    ) -> Result<Vec<BranchId>, AppError> {
        let one = self.branch_in_org(user_id, org_id).await?;
        Ok(one.into_iter().collect())
    }
    /// Whether a manager has access to the given branch in the organization
    /// (either explicitly assigned or assigned org-wide).
    async fn has_branch_access(
        &self,
        user_id: UserId,
        org_id: OrgId,
        branch_id: BranchId,
    ) -> Result<bool, AppError> {
        let assigned = self.branch_in_org(user_id, org_id).await?;
        Ok(assigned == Some(branch_id))
    }
    /// The registered (expected) Aadhaar name for identity matching.
    async fn expected_name(&self, user_id: UserId) -> Result<Option<String>, AppError>;
    async fn update_profile(
        &self,
        user_id: UserId,
        full_name: Option<&str>,
        phone: Option<&str>,
    ) -> Result<(), AppError>;
    async fn mark_verified(&self, user_id: UserId) -> Result<(), AppError>;
    async fn is_verified(&self, user_id: UserId) -> Result<bool, AppError>;
}

/// Server-side refresh-token state: enables rotation and revocation (logout).
/// Only SHA-256 hashes of tokens are ever stored.
#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn store(
        &self,
        token_hash: &str,
        user_id: UserId,
        org_id: OrgId,
        ttl_secs: i64,
    ) -> Result<(), AppError>;
    /// Atomically revoke the token if it is live (not revoked, not expired).
    /// Returns true when the token was live — the caller may then rotate.
    async fn consume(&self, token_hash: &str) -> Result<bool, AppError>;
    /// Revoke every live refresh token for the user (logout everywhere).
    async fn revoke_all(&self, user_id: UserId) -> Result<(), AppError>;
}

#[async_trait]
pub trait EmailOtpRepository: Send + Sync {
    async fn create(
        &self,
        id: Uuid,
        user_id: UserId,
        email: &str,
        code: &str,
        ttl_secs: i64,
    ) -> Result<(), AppError>;
    /// Locks, verifies and consumes the newest challenge in one transaction.
    /// Implementations must perform an Argon2 verification even when no usable
    /// challenge exists, keeping invalid-code timing substantially uniform.
    async fn verify_latest(
        &self,
        email: &str,
        user_id: UserId,
        code: &str,
        max_attempts: i32,
    ) -> Result<bool, AppError>;
    /// Remove a just-created challenge when delivery fails so it cannot block
    /// the user's resend cooldown.
    async fn discard(&self, id: Uuid) -> Result<(), AppError>;
    async fn last_sent_at(&self, email: &str) -> Result<Option<OffsetDateTime>, AppError>;
    async fn sent_since(&self, email: &str, window_secs: i64) -> Result<i64, AppError>;
}

// ============ Trusted devices (control plane) ============

#[async_trait]
pub trait TrustedDeviceRepository: Send + Sync {
    /// True when this device_id was already confirmed for this user.
    async fn is_trusted(&self, user_id: UserId, device_id: &str) -> Result<bool, AppError>;
    async fn trust(
        &self,
        id: Uuid,
        user_id: UserId,
        device_id: &str,
        name: Option<&str>,
    ) -> Result<(), AppError>;
}

// ============ Push device tokens (control plane) ============

#[async_trait]
pub trait DeviceTokenRepository: Send + Sync {
    async fn register(
        &self,
        id: Uuid,
        user_id: UserId,
        org_id: OrgId,
        token: &str,
        platform: &str,
    ) -> Result<(), AppError>;
    async fn unregister(&self, user_id: UserId, token: &str) -> Result<(), AppError>;
    /// Tokens for all users of a branch (or org-wide when branch is None) —
    /// used to fan out notices.
    async fn tokens_for_recipients(
        &self,
        org_id: OrgId,
        branch_id: Option<BranchId>,
    ) -> Result<Vec<String>, AppError>;
    async fn tokens_for_user(&self, user_id: UserId) -> Result<Vec<String>, AppError>;
}

// ============ Marketplace enquiries (control plane) ============

pub struct EnquiryRecord {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub phone: String,
    pub note: Option<String>,
    pub status: String,
    pub created_epoch: i64,
    pub prospect_name: Option<String>,
    pub prospect_email: Option<String>,
}

#[async_trait]
pub trait EnquiryRepository: Send + Sync {
    async fn create(
        &self,
        id: Uuid,
        org_id: OrgId,
        branch_id: BranchId,
        prospect_id: UserId,
        phone: &str,
        note: Option<&str>,
    ) -> Result<(), AppError>;
    /// Leads for one branch (the manager's inbox).
    async fn list_for_branch(&self, branch_id: BranchId) -> Result<Vec<EnquiryRecord>, AppError>;
    /// Every lead in the org (the owner sees the whole picture).
    async fn list_for_org(&self, org_id: OrgId) -> Result<Vec<EnquiryRecord>, AppError>;
    async fn set_status(&self, id: Uuid, status: &str) -> Result<(), AppError>;
    /// The org a listing branch belongs to (public listing lookup).
    async fn org_of_branch(&self, branch_id: BranchId) -> Result<Option<OrgId>, AppError>;
}

// ============ Live vacancy snapshot (control plane projection) ============

pub struct VacancySnapshot {
    pub beds_total: i32,
    pub beds_available: i32,
    pub starting_price_paise: Option<i64>,
    pub stale: bool,
}

#[async_trait]
pub trait VacancyRepository: Send + Sync {
    /// Derived count + cheapest bed price for a branch, computed in the TENANT
    /// schema by the projector.
    async fn compute_for_branch(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<(i32, i32, Option<i64>), AppError>;
    /// Write the snapshot to the control plane so discovery can read it.
    async fn upsert_snapshot(
        &self,
        org_id: OrgId,
        branch_id: BranchId,
        beds_total: i32,
        beds_available: i32,
        starting_price_paise: Option<i64>,
    ) -> Result<(), AppError>;
    async fn snapshot_for(&self, branch_id: BranchId) -> Result<Option<VacancySnapshot>, AppError>;
    async fn mark_stale(&self, branch_id: BranchId) -> Result<(), AppError>;
    /// Every branch that currently has a live listing (the projector's sweep set).
    async fn live_branch_ids(&self) -> Result<Vec<(OrgId, BranchId)>, AppError>;
}

// ============ Tenant-scoped aggregates ============

pub struct BranchSummary {
    pub id: Uuid,
    pub name: String,
}

#[async_trait]
pub trait BranchRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &TenantContext,
        id: BranchId,
        name: &str,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn list(&self, ctx: &TenantContext) -> Result<Vec<BranchSummary>, AppError>;
    async fn count(&self, ctx: &TenantContext) -> Result<i64, AppError>;
}

pub struct RoomSummary {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub number: String,
    pub cooling_type: String,
    /// Explicitly assigned floor, if any. Rooms created before floors existed
    /// carry `None`, and clients fall back to deriving the floor from
    /// [`RoomSummary::number`] exactly as they did before.
    pub floor_id: Option<Uuid>,
}

pub struct FloorSummary {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub sort_order: i32,
    /// Live rooms currently assigned to this floor.
    pub room_count: i64,
}

/// A room to create alongside its floor, as described by the add-floor form.
///
/// Beds are a count rather than a list: the form asks how many people share
/// the room, and the labels (A, B, C...) follow from that, exactly as the
/// onboarding bootstrapper names them.
pub struct NewRoomSpec {
    pub number: String,
    pub cooling_type: String,
    pub beds: u16,
}

#[async_trait]
pub trait FloorRepository: Send + Sync {
    /// Creates the floor and, in the same transaction, any rooms and beds
    /// described with it. A half-built floor is worse than none, so this is
    /// all-or-nothing.
    async fn create(
        &self,
        ctx: &TenantContext,
        id: FloorId,
        branch_id: BranchId,
        name: &str,
        sort_order: i32,
        rooms: &[NewRoomSpec],
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn list_by_branch(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<Vec<FloorSummary>, AppError>;
    async fn rename(
        &self,
        ctx: &TenantContext,
        id: FloorId,
        name: &str,
        sort_order: i32,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    /// Soft-delete. Refuses while rooms are still assigned, so a floor can
    /// never be removed out from under the rooms that point at it.
    async fn delete(
        &self,
        ctx: &TenantContext,
        id: FloorId,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn branch_of(
        &self,
        ctx: &TenantContext,
        floor_id: FloorId,
    ) -> Result<Option<BranchId>, AppError>;
}

#[async_trait]
pub trait RoomRepository: Send + Sync {
    /// Creates a room and, in the same transaction, `beds` beds labelled
    /// A, B, C… A room created with zero beds cannot hold anyone until beds
    /// are added separately.
    async fn create(
        &self,
        ctx: &TenantContext,
        id: RoomId,
        branch_id: BranchId,
        number: &str,
        cooling_type: &str,
        floor_id: Option<FloorId>,
        beds: u16,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn list_by_branch(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<Vec<RoomSummary>, AppError>;
    async fn branch_of(
        &self,
        ctx: &TenantContext,
        room_id: RoomId,
    ) -> Result<Option<BranchId>, AppError>;
}

pub struct BedSummary {
    pub id: Uuid,
    pub room_id: Uuid,
    pub label: String,
    /// Active resident currently assigned to this bed, if any.
    pub occupant_resident_id: Option<Uuid>,
    pub occupant_name: Option<String>,
}

#[async_trait]
pub trait BedRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &TenantContext,
        id: BedId,
        room_id: RoomId,
        label: &str,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn list_by_room(
        &self,
        ctx: &TenantContext,
        room_id: RoomId,
    ) -> Result<Vec<BedSummary>, AppError>;
    /// Every bed in a branch (join across rooms) — one round trip for the
    /// Payments/occupancy screens, replacing the previous per-room fan-out.
    async fn list_by_branch(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<Vec<BedSummary>, AppError>;
    /// The room a bed belongs to (None if the bed is missing/deleted).
    async fn room_of(&self, ctx: &TenantContext, bed_id: BedId)
        -> Result<Option<RoomId>, AppError>;
}

pub struct ResidentSummary {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub full_name: String,
    pub status: String,
    pub plan_kind: String,
}

/// The outcome of recording a payment.
///
/// [`replayed`] is the important field: a retried request with a known
/// idempotency key lands on the ORIGINAL payment and receipt, and the caller
/// must not render or store a second invoice for it.
pub struct RecordedPayment {
    pub payment_id: Uuid,
    pub receipt_id: Uuid,
    pub receipt_number: String,
    pub replayed: bool,
}

pub struct ResidentDetail {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub branch_name: String,
    pub room_number: Option<String>,
    pub bed_label: Option<String>,
    pub full_name: String,
    /// Contact number, printed in a receipt's "Bill To" block. Optional
    /// because a resident can be created before their phone is captured.
    pub phone: Option<String>,
    pub status: String,
    pub plan_kind: String,
    pub rent_paise: i64,
    pub deposit_paise: i64,
    pub joined: OffsetDateTime,
    /// Set once the resident is settled out; rent accrual stops here.
    pub moved_out_at: Option<OffsetDateTime>,
}

#[async_trait]
pub trait ResidentRepository: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
        branch_id: BranchId,
        full_name: &str,
        phone: Option<&str>,
        plan_kind: PlanKind,
        rent_paise: i64,
        deposit_paise: i64,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn list_by_branch(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<Vec<ResidentSummary>, AppError>;
    async fn get(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
    ) -> Result<Option<ResidentDetail>, AppError>;
    async fn mark_moved_out(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
        last_day: time::Date,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    /// Assign (or clear, with None) the bed a resident occupies.
    async fn assign_bed(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
        bed_id: Option<BedId>,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    /// The active (non-moved-out) resident occupying a bed, if any.
    async fn occupant_of_bed(
        &self,
        ctx: &TenantContext,
        bed_id: BedId,
    ) -> Result<Option<ResidentId>, AppError>;
    async fn link_user(
        &self,
        ctx: &TenantContext,
        id: ResidentId,
        user_id: UserId,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn resident_id_of_user(
        &self,
        ctx: &TenantContext,
        user_id: UserId,
    ) -> Result<Option<ResidentId>, AppError>;
}

// ============ Money: payments & receipts ============

pub struct ReceiptMeta {
    pub id: Uuid,
    pub receipt_number: String,
    pub amount_paise: i64,
    pub status: String,
    pub issued_epoch: i64,
}

pub struct ReceiptFile {
    pub storage_key: Option<String>,
    pub status: String,
    pub resident_id: Uuid,
}

#[async_trait]
pub trait PaymentRepository: Send + Sync {
    /// Record the cash payment + a `pending` receipt row, atomically, with audit.
    /// `idempotency_key` (when the client supplies one) makes retried POSTs safe.
    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        ctx: &TenantContext,
        payment_id: PaymentId,
        receipt_id: ReceiptId,
        resident_id: ResidentId,
        amount_paise: i64,
        receipt_number: &str,
        recorded_by: Option<UserId>,
        method: &str,
        reference: Option<&str>,
        idempotency_key: Option<&str>,
        actor: &AuditActor,
    ) -> Result<RecordedPayment, AppError>;
    async fn set_receipt_ready(
        &self,
        ctx: &TenantContext,
        receipt_id: ReceiptId,
        storage_key: &str,
    ) -> Result<(), AppError>;
    async fn paid_total(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
    ) -> Result<i64, AppError>;
    async fn list_receipts(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
    ) -> Result<Vec<ReceiptMeta>, AppError>;
    async fn get_receipt(
        &self,
        ctx: &TenantContext,
        receipt_id: ReceiptId,
    ) -> Result<Option<ReceiptFile>, AppError>;
    /// Reverse the resident's most recent live payment, withdrawing its receipt
    /// with it. Returns `None` when there is nothing left to reverse.
    ///
    /// A soft delete rather than a hard one: the row stays for audit, and every
    /// balance query already filters on `deleted_at IS NULL`, so the resident's
    /// outstanding amount recomputes on its own.
    async fn void_last_payment(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
        actor: &AuditActor,
    ) -> Result<Option<VoidedPayment>, AppError>;
}

/// What was undone, so the caller can confirm it precisely to the user.
pub struct VoidedPayment {
    pub payment_id: Uuid,
    pub amount_paise: i64,
    pub invoice_number: Option<String>,
}

// ============ Maker-checker ============

pub struct ChangeTarget {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub operation: String,
    pub payload: Value,
}

pub struct ChangeRequestSummary {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub entity_name: Option<String>,
    pub operation: String,
    pub status: String,
    pub requested_by: Uuid,
    pub requested_by_name: Option<String>,
    pub proposed: Option<Value>,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub decided_at: Option<String>,
    pub decided_by: Option<Uuid>,
    pub decided_by_name: Option<String>,
}

#[async_trait]
pub trait ChangeRequestRepository: Send + Sync {
    async fn apply_now(
        &self,
        ctx: &TenantContext,
        target: &ChangeTarget,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn create_pending(
        &self,
        ctx: &TenantContext,
        target: &ChangeTarget,
        requested_by: UserId,
    ) -> Result<Uuid, AppError>;
    async fn list_changes(
        &self,
        ctx: &TenantContext,
        status: Option<&str>,
    ) -> Result<Vec<ChangeRequestSummary>, AppError>;
    async fn list_pending(
        &self,
        ctx: &TenantContext,
    ) -> Result<Vec<ChangeRequestSummary>, AppError> {
        self.list_changes(ctx, Some("pending")).await
    }
    async fn approve(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        decided_by: UserId,
        request_id: Option<Uuid>,
    ) -> Result<(), AppError>;
    async fn reject(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        decided_by: UserId,
    ) -> Result<(), AppError>;
}

pub struct AuditSummary {
    pub id: i64,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub created_at_epoch: i64,
}

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn list_recent(
        &self,
        ctx: &TenantContext,
        limit: i64,
    ) -> Result<Vec<AuditSummary>, AppError>;
}

// ============ Receipt templates (per org, control plane) ============

#[async_trait]
pub trait ReceiptTemplateRepository: Send + Sync {
    /// Resolve the org's template, falling back to the system default.
    async fn resolve(&self, org_id: OrgId) -> Result<ReceiptTemplate, AppError>;
    async fn upsert(&self, org_id: OrgId, template: &ReceiptTemplate) -> Result<(), AppError>;
}

// ============ Complaints ============

pub struct ComplaintSummary {
    pub id: Uuid,
    pub branch_id: Option<Uuid>,
    pub raised_by: Uuid,
    pub title: String,
    /// What the resident actually wrote. Stored since the table was created but
    /// never returned, so the apps could only ever show a one-line title.
    pub description: Option<String>,
    pub state: String,
    pub is_business_level: bool,
    pub created_at: OffsetDateTime,
    /// Moves every time the state does, so it reads as "last updated".
    pub updated_at: OffsetDateTime,
    pub tenant_name: Option<String>,
    pub floor_number: Option<String>,
    pub room_number: Option<String>,
}

pub struct ComplaintStateRow {
    pub state: String,
    pub branch_id: Option<Uuid>,
    pub raised_by: Uuid,
    pub is_business_level: bool,
}

#[async_trait]
pub trait ComplaintRepository: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn raise(
        &self,
        ctx: &TenantContext,
        id: ComplaintId,
        branch_id: Option<BranchId>,
        raised_by: UserId,
        title: &str,
        description: Option<&str>,
        is_business_level: bool,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn list_visible(
        &self,
        ctx: &TenantContext,
        role: Role,
        user_id: UserId,
        branch: Option<BranchId>,
    ) -> Result<Vec<ComplaintSummary>, AppError>;
    async fn get_state(
        &self,
        ctx: &TenantContext,
        id: ComplaintId,
    ) -> Result<Option<ComplaintStateRow>, AppError>;
    async fn transition(
        &self,
        ctx: &TenantContext,
        id: ComplaintId,
        from: &str,
        to: &str,
        by: UserId,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
}

// ============ Notices ============

pub struct NoticeSummary {
    pub id: Uuid,
    pub branch_id: Option<Uuid>,
    pub title: String,
    pub body: Option<String>,
    pub created_epoch: i64,
}

#[async_trait]
pub trait NoticeRepository: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        ctx: &TenantContext,
        id: NoticeId,
        branch_id: Option<BranchId>,
        title: &str,
        body: Option<&str>,
        created_by: UserId,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
    async fn list_for(
        &self,
        ctx: &TenantContext,
        role: Role,
        branch: Option<BranchId>,
    ) -> Result<Vec<NoticeSummary>, AppError>;
}

// ============ Identity verification (control plane) ============

#[async_trait]
pub trait VerificationRepository: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        id: Uuid,
        user_id: UserId,
        org_id: OrgId,
        method: &str,
        status: &str,
        verified_name: Option<&str>,
        phone: Option<&str>,
        emergency_contact: Option<&str>,
        reason: Option<&str>,
    ) -> Result<(), AppError>;
}

// ============ Archive (cold storage of soft-deleted records) ============

#[async_trait]
pub trait ArchiveRepository: Send + Sync {
    /// Deleted rows past the cutoff, as a JSON array + their count.
    async fn fetch_deleted(
        &self,
        ctx: &TenantContext,
        table: &str,
        cutoff: OffsetDateTime,
    ) -> Result<(Value, i64), AppError>;
    /// Physically purge those rows, record a manifest row, and audit.
    async fn purge(
        &self,
        ctx: &TenantContext,
        table: &str,
        cutoff: OffsetDateTime,
        object_key: &str,
        row_count: i64,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
}

// ============ Move-out settlements (tenant) ============

pub struct SettlementRecord {
    pub resident_id: Uuid,
    pub last_day: time::Date,
    pub notice_days: i64,
    pub rent_to_date_paise: i64,
    pub paid_total_paise: i64,
    pub outstanding_paise: i64,
    pub deposit_held_paise: i64,
    pub deductions_total_paise: i64,
    pub refund_paise: i64,
    pub forfeited: bool,
    pub note: String,
    pub deductions: Vec<(String, i64)>,
}

#[async_trait]
pub trait SettlementRepository: Send + Sync {
    /// Persist a move-out settlement + its deduction line items in one
    /// transaction. Returns the settlement id.
    async fn record(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        record: &SettlementRecord,
        actor: &AuditActor,
    ) -> Result<(), AppError>;
}

// ============ Away days (tenant) ============

#[derive(Clone, Debug)]
pub struct AwayPeriod {
    pub id: Uuid,
    pub resident_id: Uuid,
    pub from_date: time::Date,
    pub to_date: time::Date,
    pub note: Option<String>,
    pub status: String,
    pub created_at: OffsetDateTime,
}

/// An away period joined to the person and bed it belongs to.
///
/// The staff view needs to answer "whose bed is empty, and which one" at a
/// glance; a bare `resident_id` makes the caller issue a lookup per row and
/// leaves the UI showing UUIDs until it does.
#[derive(Clone, Debug)]
pub struct AwayPeriodDetail {
    pub period: AwayPeriod,
    pub resident_name: String,
    pub room_number: Option<String>,
    pub branch_id: Uuid,
    pub branch_name: String,
}

#[async_trait]
pub trait AwayRepository: Send + Sync {
    /// Insert a new active away period + audit. Overlap validation happens in
    /// the service before this is called.
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        resident_id: ResidentId,
        from_date: time::Date,
        to_date: time::Date,
        note: Option<&str>,
        actor: &AuditActor,
    ) -> Result<(), AppError>;

    /// Active (non-cancelled) periods belonging to a resident, newest first.
    async fn list_active_for_resident(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
    ) -> Result<Vec<AwayPeriod>, AppError>;

    /// Active periods with resident and bed attached, newest first.
    ///
    /// `branch` scopes the result to one branch (a manager sees only the
    /// branch they run); `None` returns the whole org (owner).
    async fn list_active_detailed(
        &self,
        ctx: &TenantContext,
        branch: Option<BranchId>,
    ) -> Result<Vec<AwayPeriodDetail>, AppError>;

    /// Correct a resident's own period in place. Returns false when no active
    /// period of theirs has that id, so a stale or forged id is a 404 rather
    /// than a silent no-op.
    #[allow(clippy::too_many_arguments)]
    async fn update(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        resident_id: ResidentId,
        from_date: time::Date,
        to_date: time::Date,
        note: Option<&str>,
        actor: &AuditActor,
    ) -> Result<bool, AppError>;

    /// Cancel a resident's own period. Tenant-only per RLS + service check.
    async fn cancel(
        &self,
        ctx: &TenantContext,
        id: Uuid,
        resident_id: ResidentId,
        actor: &AuditActor,
    ) -> Result<bool, AppError>;

    /// Active periods for `resident` overlapping `[from, to]` inclusive, used
    /// to reject a double-booked range.
    ///
    /// `exclude` drops one period from the comparison. Editing a period must
    /// pass its own id here, or the row being corrected overlaps itself and
    /// every edit is rejected as a conflict.
    async fn overlaps(
        &self,
        ctx: &TenantContext,
        resident_id: ResidentId,
        from: time::Date,
        to: time::Date,
        exclude: Option<Uuid>,
    ) -> Result<Vec<AwayPeriod>, AppError>;
}

// ============ Finance reports ============

pub struct FinanceReport {
    /// All cash taken at the desk, rent and deposits together.
    pub collected_paise: i64,
    /// The deposit share of `collected_paise`. Reported separately because
    /// deposit money is held on the resident's behalf and refunded at
    /// move-out, so it must not be read as rent income.
    pub deposits_collected_paise: i64,
    /// Deposits currently HELD, from residents.deposit_paise -- the liability,
    /// not the cash received this period.
    pub deposits_paise: i64,
    pub accrued_paise: i64,
    pub outstanding_paise: i64,
    pub residents_active: i64,
    pub residents_total: i64,
    /// Live bed counts (not resident counts) -- the actual "occupancy" figure.
    /// A branch can have more beds than residents (e.g. a just-added empty
    /// bed), which `residents_active/residents_total` alone can't reflect.
    pub beds_total: i32,
    pub beds_occupied: i32,
}

pub struct BranchFinance {
    pub branch_id: Uuid,
    pub branch_name: String,
    pub report: FinanceReport,
}

/// Per-resident rollup for the Payments screen's dues list (A7 batch fix):
/// one round trip instead of N ledgers.
pub struct ResidentDue {
    pub resident_id: Uuid,
    pub full_name: String,
    pub plan_kind: String,
    pub status: String,
    pub rent_paise: i64,
    pub deposit_paise: i64,
    pub paid_total_paise: i64,
    pub accrued_paise: i64,
    pub outstanding_paise: i64,
}

#[async_trait]
pub trait FinanceRepository: Send + Sync {
    async fn branch_report(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<FinanceReport, AppError>;
    async fn org_report(
        &self,
        ctx: &TenantContext,
    ) -> Result<(FinanceReport, Vec<BranchFinance>), AppError>;
    /// One-query dues list used by the payments screen (A7). Accrued mirrors
    /// SimpleBillingEngine and is the same formula `ACCRUED_EXPR` uses.
    async fn branch_dues(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<Vec<ResidentDue>, AppError>;
}

