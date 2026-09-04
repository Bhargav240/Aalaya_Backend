//! In-memory repository fakes. State lives in `Mutex` interiors; only the
//! methods the suites exercise have real behavior, the rest `unimplemented!()`.
use async_trait::async_trait;
use pg_application::error::AppError;
use pg_application::ports::repositories::RefreshTokenRepository;
use pg_application::ports::tenancy::TenantContext;
use pg_application::ports::{
    AuditActor, BedRepository, BedSummary, BranchRepository, BranchSummary,
    ChangeRequestRepository, ChangeRequestSummary, ChangeTarget, LoginRecord, MembershipContext,
    ResidentDetail, ResidentRepository, ResidentSummary, RoomRepository, RoomSummary,
    UserRepository,
};
use pg_domain::{BedId, BranchId, OrgId, PlanKind, ResidentId, RoomId, UserId, UserStatus};
use std::collections::HashMap;
use std::sync::Mutex;
use time::OffsetDateTime;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Users (control plane): phone lookup + auth context for OTP / auth flows.
// ---------------------------------------------------------------------------

pub struct TestUser {
    pub user_id: UserId,
    pub org_id: OrgId,
    pub role: pg_domain::Role,
    pub status: UserStatus,
    pub phone: Option<String>,
    pub full_name: Option<String>,
}

pub struct FakeUserRepo {
    /// Name returned by `expected_name`; `None` models a user with no name on
    /// record, which receipts must still handle.
    pub expected_name: Mutex<Option<String>>,
    pub users: Mutex<HashMap<String, LoginRecord>>,
    pub google_bindings: Mutex<HashMap<String, (UserId, OrgId, pg_domain::Role)>>,
    linked_subs: Mutex<HashMap<UserId, String>>,
    /// audit trail of calls made against this repo
    pub mark_verified_calls: Mutex<u32>,
    /// Seedable map for `auth_context(user_id)` lookups (refresh rotation).
    pub auth_contexts: Mutex<HashMap<UserId, (OrgId, pg_domain::Role)>>,
    pub additional_auth_contexts: Mutex<HashMap<UserId, Vec<(OrgId, pg_domain::Role)>>>,
    pub branch_scopes: Mutex<HashMap<UserId, BranchId>>,
}

impl TestUser {
    pub fn owner(org_id: OrgId) -> Self {
        Self {
            user_id: UserId::new(Uuid::new_v4()),
            org_id,
            role: pg_domain::Role::Owner,
            status: UserStatus::Active,
            phone: None,
            full_name: None,
        }
    }
    pub fn with_phone(mut self, phone: &str) -> Self {
        self.phone = Some(phone.to_string());
        self
    }
}

impl FakeUserRepo {
    pub fn new() -> Self {
        Self {
            expected_name: Mutex::new(Some("Test Staff".to_string())),
            users: Mutex::new(HashMap::new()),
            google_bindings: Mutex::new(HashMap::new()),
            linked_subs: Mutex::new(HashMap::new()),
            mark_verified_calls: Mutex::new(0),
            auth_contexts: Mutex::new(HashMap::new()),
            additional_auth_contexts: Mutex::new(HashMap::new()),
            branch_scopes: Mutex::new(HashMap::new()),
        }
    }

    /// Seed a login record keyed by its verified identity (normally email).
    pub fn insert(&self, identity: &str, rec: LoginRecord) {
        self.users
            .lock()
            .expect("poisoned")
            .insert(identity.to_string(), rec);
    }

    /// Explicitly pre-bind a google sub → identity triple (Google sign-in).
    pub fn bind_google(&self, sub: &str, triple: (UserId, OrgId, pg_domain::Role)) {
        self.google_bindings
            .lock()
            .expect("poisoned")
            .insert(sub.to_string(), triple);
    }

    /// Roles recorded for the user (test assertion helper).
    pub fn roles_of(&self, _uid: UserId) -> Vec<pg_domain::Role> {
        vec![pg_domain::Role::Owner]
    }
}

impl Default for FakeUserRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserRepository for FakeUserRepo {
    async fn create_active_owner(
        &self,
        _id: UserId,
        _org_id: OrgId,
        _email: &str,
        _phone: Option<&str>,
        _full_name: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn find_by_email(&self, email: &str) -> Result<Option<LoginRecord>, AppError> {
        Ok(self
            .users
            .lock()
            .expect("poisoned")
            .get(email)
            .map(|record| LoginRecord {
                user_id: record.user_id,
                org_id: record.org_id,
                role: record.role,
                status: record.status,
                full_name: record.full_name.clone(),
            }))
    }
    async fn create_email_explorer(
        &self,
        id: UserId,
        platform_org_id: OrgId,
        email: &str,
    ) -> Result<LoginRecord, AppError> {
        let record = LoginRecord {
            user_id: id,
            org_id: platform_org_id,
            role: pg_domain::Role::Prospect,
            status: UserStatus::Active,
            full_name: None,
        };
        self.insert(
            email,
            LoginRecord {
                user_id: record.user_id,
                org_id: record.org_id,
                role: record.role,
                status: record.status,
                full_name: None,
            },
        );
        Ok(record)
    }
    async fn mark_email_verified(&self, _user_id: UserId) -> Result<(), AppError> {
        Ok(())
    }
    async fn verified_email_of(&self, user_id: UserId) -> Result<Option<String>, AppError> {
        Ok(self
            .users
            .lock()
            .expect("poisoned")
            .iter()
            .find(|(identity, record)| identity.contains('@') && record.user_id == user_id)
            .map(|(email, _)| email.clone()))
    }
    async fn auth_context(
        &self,
        user_id: UserId,
    ) -> Result<Option<(OrgId, pg_domain::Role)>, AppError> {
        Ok(self
            .auth_contexts
            .lock()
            .expect("poisoned")
            .get(&user_id)
            .copied())
    }
    async fn has_active_membership(
        &self,
        user_id: UserId,
        org_id: OrgId,
        role: pg_domain::Role,
    ) -> Result<bool, AppError> {
        let primary_matches = self
            .auth_contexts
            .lock()
            .expect("poisoned")
            .get(&user_id)
            .is_some_and(|context| *context == (org_id, role));
        let additional_matches = self
            .additional_auth_contexts
            .lock()
            .expect("poisoned")
            .get(&user_id)
            .is_some_and(|contexts| contexts.contains(&(org_id, role)));
        Ok(primary_matches || additional_matches)
    }
    async fn list_active_memberships(
        &self,
        user_id: UserId,
    ) -> Result<Vec<MembershipContext>, AppError> {
        let mut contexts: Vec<_> = self
            .auth_contexts
            .lock()
            .expect("poisoned")
            .get(&user_id)
            .map(|(org_id, role)| MembershipContext {
                membership_id: Uuid::new_v4(),
                org_id: *org_id,
                organization_name: "Test organization".to_string(),
                role: *role,
                branch_id: None,
            })
            .into_iter()
            .collect();
        contexts.extend(
            self.additional_auth_contexts
                .lock()
                .expect("poisoned")
                .get(&user_id)
                .into_iter()
                .flatten()
                .map(|(org_id, role)| MembershipContext {
                    membership_id: Uuid::new_v4(),
                    org_id: *org_id,
                    organization_name: "Additional organization".to_string(),
                    role: *role,
                    branch_id: None,
                }),
        );
        Ok(contexts)
    }
    async fn branch_of(&self, user_id: UserId) -> Result<Option<BranchId>, AppError> {
        Ok(self
            .branch_scopes
            .lock()
            .expect("poisoned")
            .get(&user_id)
            .copied())
    }
    async fn expected_name(&self, _user_id: UserId) -> Result<Option<String>, AppError> {
        Ok(self.expected_name.lock().expect("poisoned").clone())
    }
    async fn find_by_google_sub(
        &self,
        sub: &str,
    ) -> Result<Option<(UserId, OrgId, pg_domain::Role)>, AppError> {
        Ok(self
            .google_bindings
            .lock()
            .expect("poisoned")
            .get(sub)
            .copied())
    }
    async fn link_google_sub(&self, user_id: UserId, sub: &str) -> Result<(), AppError> {
        self.linked_subs
            .lock()
            .expect("poisoned")
            .insert(user_id, sub.to_string());
        Ok(())
    }
    async fn mark_verified(&self, _user_id: UserId) -> Result<(), AppError> {
        *self.mark_verified_calls.lock().expect("poisoned") += 1;
        Ok(())
    }
    async fn is_verified(&self, _user_id: UserId) -> Result<bool, AppError> {
        Ok(true) // tests never block on the gate
    }
}

// ---------------------------------------------------------------------------
// Refresh tokens: rotation + revocation.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeRefreshTokenRepo {
    /// token_hash -> (user_id, org_id, live)
    store: Mutex<HashMap<String, (Uuid, Uuid, bool)>>,
    pub revoke_all_calls: Mutex<Vec<UserId>>,
}

impl FakeRefreshTokenRepo {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_live(&self, token_hash: &str) -> bool {
        self.store
            .lock()
            .expect("poisoned")
            .get(token_hash)
            .map(|(_, _, live)| *live)
            .unwrap_or(false)
    }
}

#[async_trait]
impl RefreshTokenRepository for FakeRefreshTokenRepo {
    async fn store(
        &self,
        token_hash: &str,
        user_id: UserId,
        org_id: OrgId,
        _ttl_secs: i64,
    ) -> Result<(), AppError> {
        self.store.lock().expect("poisoned").insert(
            token_hash.to_string(),
            (user_id.as_uuid(), org_id.as_uuid(), true),
        );
        Ok(())
    }
    async fn consume(&self, token_hash: &str) -> Result<bool, AppError> {
        match self.store.lock().expect("poisoned").get_mut(token_hash) {
            Some((_, _, live)) if *live => {
                *live = false;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    async fn revoke_all(&self, user_id: UserId) -> Result<(), AppError> {
        self.revoke_all_calls
            .lock()
            .expect("poisoned")
            .push(user_id);
        for (_, _, live) in self.store.lock().expect("poisoned").values_mut() {
            *live = false;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tenant aggregates used by money + maker-checker suites.
// ---------------------------------------------------------------------------

pub struct FakeBranchRepo {
    count: Mutex<i64>,
    created: Mutex<Vec<String>>,
}

impl FakeBranchRepo {
    pub fn with_count(n: i64) -> Self {
        Self {
            count: Mutex::new(n),
            created: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl BranchRepository for FakeBranchRepo {
    async fn create(
        &self,
        _ctx: &TenantContext,
        _id: BranchId,
        name: &str,
        _actor: &AuditActor,
    ) -> Result<(), AppError> {
        self.created
            .lock()
            .expect("poisoned")
            .push(name.to_string());
        Ok(())
    }
    async fn list(&self, _ctx: &TenantContext) -> Result<Vec<BranchSummary>, AppError> {
        unimplemented!("FakeBranchRepo::list")
    }
    async fn count(&self, _ctx: &TenantContext) -> Result<i64, AppError> {
        Ok(*self.count.lock().expect("poisoned"))
    }
}

pub struct FakeRoomRepo;

#[async_trait]
impl RoomRepository for FakeRoomRepo {
    async fn create(
        &self,
        _ctx: &TenantContext,
        _id: RoomId,
        _b: BranchId,
        _n: &str,
        _cooling_type: &str,
        _floor_id: Option<pg_domain::FloorId>,
        _beds: u16,
        _a: &AuditActor,
    ) -> Result<(), AppError> {
        unimplemented!()
    }
    async fn list_by_branch(
        &self,
        _ctx: &TenantContext,
        _b: BranchId,
    ) -> Result<Vec<RoomSummary>, AppError> {
        unimplemented!()
    }
    async fn branch_of(
        &self,
        _ctx: &TenantContext,
        _r: RoomId,
    ) -> Result<Option<BranchId>, AppError> {
        unimplemented!()
    }
}

pub struct FakeBedRepo;

#[async_trait]
impl BedRepository for FakeBedRepo {
    async fn create(
        &self,
        _ctx: &TenantContext,
        _id: BedId,
        _r: RoomId,
        _l: &str,
        _a: &AuditActor,
    ) -> Result<(), AppError> {
        unimplemented!()
    }
    async fn list_by_room(
        &self,
        _ctx: &TenantContext,
        _r: RoomId,
    ) -> Result<Vec<BedSummary>, AppError> {
        unimplemented!()
    }
    async fn list_by_branch(
        &self,
        _ctx: &TenantContext,
        _b: BranchId,
    ) -> Result<Vec<BedSummary>, AppError> {
        unimplemented!()
    }
    async fn room_of(&self, _ctx: &TenantContext, _b: BedId) -> Result<Option<RoomId>, AppError> {
        unimplemented!()
    }
}

// ---------------------------------------------------------------------------
// Residents.
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SeededResident {
    pub id: ResidentId,
    pub branch_id: BranchId,
    pub branch_name: String,
    pub full_name: String,
    /// Printed in a receipt's "Bill To" block; `None` models a resident
    /// recorded before their number was captured.
    pub phone: Option<String>,
    pub status: String,
    pub plan_kind: String,
    pub rent_paise: i64,
    pub deposit_paise: i64,
    pub joined: OffsetDateTime,
    pub moved_out_at: Option<OffsetDateTime>,
}

pub struct FakeResidentRepo {
    by_id: Mutex<HashMap<Uuid, SeededResident>>,
    pub moved_out_calls: Mutex<Vec<(Uuid, time::Date)>>,
    /// user_id -> resident_id linkage returned by resident_id_of_user.
    pub linked_residents: Mutex<HashMap<Uuid, ResidentId>>,
}

impl SeededResident {
    pub fn active(branch_id: BranchId) -> Self {
        Self {
            id: ResidentId::new(Uuid::new_v4()),
            phone: Some("9000000001".to_string()),
            branch_id,
            branch_name: "Main Branch".into(),
            full_name: "Aadhaar Name".into(),
            status: "active".into(),
            plan_kind: "monthly".into(),
            rent_paise: 1_000_000, // ₹10,000
            deposit_paise: 50_0000,
            joined: OffsetDateTime::now_utc(),
            moved_out_at: None,
        }
    }
}

impl FakeResidentRepo {
    pub fn new() -> Self {
        Self {
            by_id: Mutex::new(HashMap::new()),
            moved_out_calls: Mutex::new(Vec::new()),
            linked_residents: Mutex::new(HashMap::new()),
        }
    }
    pub fn seed(&self, r: SeededResident) {
        self.by_id
            .lock()
            .expect("poisoned")
            .insert(r.id.as_uuid(), r);
    }
}

impl FakeResidentRepo {
    /// Wire a user_id → resident_id into the fake (for resident_id_of_user).
    pub fn seed_link(&self, user_id: uuid::Uuid, resident_id: ResidentId) {
        self.linked_residents
            .lock()
            .expect("poisoned")
            .insert(user_id, resident_id);
    }
}

impl Default for FakeResidentRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ResidentRepository for FakeResidentRepo {
    async fn create(
        &self,
        _ctx: &TenantContext,
        _id: ResidentId,
        _branch_id: BranchId,
        _full_name: &str,
        _phone: Option<&str>,
        _plan_kind: PlanKind,
        _rent_paise: i64,
        _deposit_paise: i64,
        _actor: &AuditActor,
    ) -> Result<(), AppError> {
        unimplemented!("FakeResidentRepo::create")
    }
    async fn list_by_branch(
        &self,
        _ctx: &TenantContext,
        _b: BranchId,
    ) -> Result<Vec<ResidentSummary>, AppError> {
        unimplemented!("FakeResidentRepo::list_by_branch")
    }
    async fn get(
        &self,
        _ctx: &TenantContext,
        id: ResidentId,
    ) -> Result<Option<ResidentDetail>, AppError> {
        let found = self
            .by_id
            .lock()
            .expect("poisoned")
            .get(&id.as_uuid())
            .cloned();
        Ok(found.map(|r| ResidentDetail {
            id: r.id.as_uuid(),
            branch_id: r.branch_id.as_uuid(),
            branch_name: r.branch_name,
            room_number: None,
            bed_label: None,
            full_name: r.full_name,
            phone: r.phone,
            status: r.status,
            plan_kind: r.plan_kind,
            rent_paise: r.rent_paise,
            deposit_paise: r.deposit_paise,
            joined: r.joined,
            moved_out_at: r.moved_out_at,
        }))
    }
    async fn mark_moved_out(
        &self,
        _ctx: &TenantContext,
        id: ResidentId,
        last_day: time::Date,
        _actor: &AuditActor,
    ) -> Result<(), AppError> {
        self.moved_out_calls
            .lock()
            .expect("poisoned")
            .push((id.as_uuid(), last_day));
        Ok(())
    }
    async fn assign_bed(
        &self,
        _ctx: &TenantContext,
        _id: ResidentId,
        _bed: Option<BedId>,
        _a: &AuditActor,
    ) -> Result<(), AppError> {
        unimplemented!("FakeResidentRepo::assign_bed")
    }
    async fn occupant_of_bed(
        &self,
        _ctx: &TenantContext,
        _b: BedId,
    ) -> Result<Option<ResidentId>, AppError> {
        unimplemented!("FakeResidentRepo::occupant_of_bed")
    }
    async fn link_user(
        &self,
        _ctx: &TenantContext,
        _id: ResidentId,
        _u: UserId,
        _a: &AuditActor,
    ) -> Result<(), AppError> {
        unimplemented!("FakeResidentRepo::link_user")
    }
    async fn resident_id_of_user(
        &self,
        _ctx: &TenantContext,
        u: UserId,
    ) -> Result<Option<ResidentId>, AppError> {
        Ok(self
            .linked_residents
            .lock()
            .expect("poisoned")
            .get(&u.as_uuid())
            .copied())
    }
}

// ---------------------------------------------------------------------------
// Maker-checker.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Organizations + tenancy resolution.
// ---------------------------------------------------------------------------

pub struct OrgRepoOk;

#[async_trait]
impl pg_application::ports::OrganizationRepository for OrgRepoOk {
    async fn create(&self, _org: &pg_domain::Organization) -> Result<(), AppError> {
        Ok(())
    }
    async fn exists(&self, _org_id: OrgId) -> Result<bool, AppError> {
        Ok(true)
    }
    async fn list_all_live(&self) -> Result<Vec<OrgId>, AppError> {
        Ok(vec![])
    }
}

pub struct StubResolver;

// ---------------------------------------------------------------------------
// Move-out settlements: capture what's written.
// ---------------------------------------------------------------------------

pub struct FakeSettlementRepo {
    pub recorded: Mutex<Vec<(Uuid, i64)>>,
}

impl FakeSettlementRepo {
    pub fn new() -> Self {
        Self {
            recorded: Mutex::new(Vec::new()),
        }
    }
}

impl Default for FakeSettlementRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl pg_application::ports::SettlementRepository for FakeSettlementRepo {
    async fn record(
        &self,
        _ctx: &TenantContext,
        id: uuid::Uuid,
        r: &pg_application::ports::SettlementRecord,
        _actor: &AuditActor,
    ) -> Result<(), AppError> {
        self.recorded
            .lock()
            .expect("poisoned")
            .push((id, r.refund_paise));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Away days (empty by default → zero rent credit).
// ---------------------------------------------------------------------------

pub struct FakeAwayRepo {
    pub seeded: Mutex<Vec<pg_application::ports::AwayPeriod>>,
    /// Branch filter each `list_active_detailed` call arrived with, so a test
    /// can prove a manager was scoped rather than handed the whole org.
    pub detailed_scopes: Mutex<Vec<Option<pg_domain::BranchId>>>,
    /// Ranges passed to `update`, for asserting what actually got written.
    pub updates: Mutex<Vec<(Uuid, time::Date, time::Date)>>,
}

impl FakeAwayRepo {
    pub fn new() -> Self {
        Self {
            seeded: Mutex::new(Vec::new()),
            detailed_scopes: Mutex::new(Vec::new()),
            updates: Mutex::new(Vec::new()),
        }
    }

    /// Seed an overlapping active period so `credit_between` yields it.
    pub fn seed(&self, p: pg_application::ports::AwayPeriod) {
        self.seeded.lock().expect("poisoned").push(p);
    }
}

impl Default for FakeAwayRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl pg_application::ports::AwayRepository for FakeAwayRepo {
    async fn create(
        &self,
        _ctx: &TenantContext,
        _id: Uuid,
        _resident_id: ResidentId,
        _from: time::Date,
        _to: time::Date,
        _note: Option<&str>,
        _actor: &AuditActor,
    ) -> Result<(), AppError> {
        unimplemented!()
    }

    async fn list_active_for_resident(
        &self,
        _ctx: &TenantContext,
        _rid: ResidentId,
    ) -> Result<Vec<pg_application::ports::AwayPeriod>, AppError> {
        unimplemented!()
    }

    async fn list_active_detailed(
        &self,
        _ctx: &TenantContext,
        branch: Option<pg_domain::BranchId>,
    ) -> Result<Vec<pg_application::ports::AwayPeriodDetail>, AppError> {
        self.detailed_scopes.lock().expect("poisoned").push(branch);
        Ok(Vec::new())
    }

    async fn update(
        &self,
        _ctx: &TenantContext,
        id: Uuid,
        _rid: ResidentId,
        from: time::Date,
        to: time::Date,
        _note: Option<&str>,
        _actor: &AuditActor,
    ) -> Result<bool, AppError> {
        self.updates.lock().expect("poisoned").push((id, from, to));
        Ok(true)
    }

    async fn cancel(
        &self,
        _ctx: &TenantContext,
        _id: Uuid,
        _rid: ResidentId,
        _actor: &AuditActor,
    ) -> Result<bool, AppError> {
        unimplemented!()
    }

    async fn overlaps(
        &self,
        _ctx: &TenantContext,
        _rid: ResidentId,
        from: time::Date,
        to: time::Date,
        exclude: Option<Uuid>,
    ) -> Result<Vec<pg_application::ports::AwayPeriod>, AppError> {
        Ok(self
            .seeded
            .lock()
            .expect("poisoned")
            .iter()
            .filter(|p| p.from_date <= to && p.to_date >= from && p.status == "active")
            .filter(|p| exclude != Some(p.id))
            .cloned()
            .collect())
    }
}

impl StubResolver {
    pub fn pooled() -> Self {
        Self
    }
}

#[async_trait]
impl pg_application::ports::TenantResolver for StubResolver {
    async fn resolve(
        &self,
        org_id: OrgId,
    ) -> Result<pg_application::ports::TenantContext, pg_application::ports::TenancyError> {
        Ok(pg_application::ports::TenantContext {
            org_id,
            placement: pg_application::ports::Placement::Pooled,
        })
    }
    async fn invalidate(&self, _org_id: OrgId) {}
}

#[derive(Default)]
pub struct FakeChangeRequestRepo {
    pub applied: Mutex<Vec<String>>,
    pub pending: Mutex<Vec<(String, Uuid)>>,
}

#[async_trait]
impl ChangeRequestRepository for FakeChangeRequestRepo {
    async fn apply_now(
        &self,
        _ctx: &TenantContext,
        target: &ChangeTarget,
        _actor: &AuditActor,
    ) -> Result<(), AppError> {
        self.applied
            .lock()
            .expect("poisoned")
            .push(format!("{}:{}", target.entity_type, target.operation));
        Ok(())
    }
    async fn create_pending(
        &self,
        _ctx: &TenantContext,
        target: &ChangeTarget,
        requested_by: UserId,
    ) -> Result<Uuid, AppError> {
        let id = Uuid::new_v4();
        self.pending.lock().expect("poisoned").push((
            format!("{}:{}", target.entity_type, target.operation),
            requested_by.as_uuid(),
        ));
        Ok(id)
    }
    async fn list_pending(
        &self,
        _ctx: &TenantContext,
    ) -> Result<Vec<ChangeRequestSummary>, AppError> {
        unimplemented!("FakeChangeRequestRepo::list_pending")
    }
    async fn approve(
        &self,
        _ctx: &TenantContext,
        _id: Uuid,
        _by: UserId,
        _req: Option<Uuid>,
    ) -> Result<(), AppError> {
        unimplemented!("FakeChangeRequestRepo::approve")
    }
    async fn reject(&self, _ctx: &TenantContext, _id: Uuid, _by: UserId) -> Result<(), AppError> {
        unimplemented!("FakeChangeRequestRepo::reject")
    }
}
