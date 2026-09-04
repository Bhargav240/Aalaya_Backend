use pg_application::ports::{
    auth::TokenService,
    billing::BillingEngine,
    clock::Clock,
    discovery::ListingRepository,
    email::{EmailSender, InvitationEmailSender},
    google::GoogleVerifier,
    notifier::Notifier,
    otp::OtpCodeGenerator,
    repositories::{
        ArchiveRepository, AuditRepository, AwayRepository, BedRepository, BranchRepository,
        ChangeRequestRepository, ComplaintRepository, DeviceTokenRepository, EmailOtpRepository,
        EnquiryRepository, FinanceRepository, FloorRepository, NoticeRepository,
        OrganizationRepository, PaymentRepository, ReceiptTemplateRepository,
        RefreshTokenRepository, ResidentRepository, RoomRepository, SettlementRepository,
        TenantActivationRepository, TenantInvitationRepository, TrustedDeviceRepository,
        UserRepository, VacancyRepository, VerificationRepository,
    },
    storage::{ObjectStore, ReceiptRenderer},
    tenancy::{TenantProvisioner, TenantResolver},
    verification::IdentityVerifier,
    DigiLockerClient, InvitationVerificationRepository,
};
use pg_application::AppError;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

#[derive(Clone, serde::Serialize)]
pub struct PlatformLog {
    pub timestamp: String,
    pub level: String,
    pub event_type: String,
    pub route: String,
    pub method: String,
    pub status_code: u16,
    pub latency_ms: u128,
    pub request_id: String,
}

pub const PLATFORM_LOG_CAPACITY: usize = 10_000;

/// The concrete adapters, injected once at startup. Present only when the DB is
/// connected; absent => the server runs in degraded mode.
pub struct Deps {
    pub onboarding_layout: Option<pg_application::services::OnboardingLayout>,
    pub org_repo: Arc<dyn OrganizationRepository>,
    pub tenant_invitation_repo: Arc<dyn TenantInvitationRepository>,
    pub tenant_activation_repo: Arc<dyn TenantActivationRepository>,
    pub user_repo: Arc<dyn UserRepository>,
    pub branch_repo: Arc<dyn BranchRepository>,
    pub room_repo: Arc<dyn RoomRepository>,
    pub bed_repo: Arc<dyn BedRepository>,
    pub resident_repo: Arc<dyn ResidentRepository>,
    pub away_repo: Arc<dyn AwayRepository>,
    pub settlement_repo: Arc<dyn SettlementRepository>,
    pub cr_repo: Arc<dyn ChangeRequestRepository>,
    pub audit_repo: Arc<dyn AuditRepository>,
    pub payment_repo: Arc<dyn PaymentRepository>,
    pub store: Arc<dyn ObjectStore>,
    pub renderer: Arc<dyn ReceiptRenderer>,
    pub billing: Arc<dyn BillingEngine>,
    pub receipt_url_ttl_secs: i64,
    pub template_repo: Arc<dyn ReceiptTemplateRepository>,
    pub complaint_repo: Arc<dyn ComplaintRepository>,
    pub notice_repo: Arc<dyn NoticeRepository>,
    pub notifier: Arc<dyn Notifier>,
    pub verifier: Arc<dyn IdentityVerifier>,
    pub google_verifier: Arc<dyn GoogleVerifier>,
    pub verification_repo: Arc<dyn VerificationRepository>,
    pub verification_method: String,
    pub archive_repo: Arc<dyn ArchiveRepository>,
    pub provisioner: Arc<dyn TenantProvisioner>,
    pub listing_repo: Arc<dyn ListingRepository>,
    pub finance_repo: Arc<dyn FinanceRepository>,
    pub vacancy_repo: Arc<dyn VacancyRepository>,
    pub enquiry_repo: Arc<dyn EnquiryRepository>,
    pub device_token_repo: Arc<dyn DeviceTokenRepository>,
    pub trusted_device_repo: Arc<dyn TrustedDeviceRepository>,
    pub floor_repo: Arc<dyn FloorRepository>,
    pub tokens: Arc<dyn TokenService>,
    pub resolver: Arc<dyn TenantResolver>,
    pub clock: Arc<dyn Clock>,
    pub provisioning_key: Option<String>,
    pub refresh_repo: Arc<dyn RefreshTokenRepository>,
    pub refresh_ttl_secs: i64,
    pub otp_codegen: Arc<dyn OtpCodeGenerator>,
    pub otp_ttl_secs: i64,
    pub otp_resend_cooldown_secs: i64,
    pub otp_max_attempts: i32,
    pub email_otp_repo: Arc<dyn EmailOtpRepository>,
    pub email_sender: Arc<dyn EmailSender>,
    pub invitation_email_sender: Arc<dyn InvitationEmailSender>,
    pub invitation_resend_cooldown_secs: i64,
    pub digilocker_client: Option<Arc<dyn DigiLockerClient>>,
    pub invitation_verification_repo: Arc<dyn InvitationVerificationRepository>,
    pub digilocker_session_encryption_key: Option<String>,
    pub digilocker_session_ttl_secs: i64,
    pub app_deep_link_base: String,
    pub email_otp_max_sends: i64,
    pub email_otp_window_secs: i64,
    /// The "platform" org that owns platform_operator + prospect accounts.
    pub platform_org_id: pg_domain::OrgId,
}

#[derive(Clone)]
pub struct AppState {
    pub deps: Option<Arc<Deps>>,
    /// JTI blocklist (jti → absolute expiry). Process-local: entries die with
    /// the process. Good enough for the single-instance deployment; each
    /// access token's natural TTL bounds the blocklist's max size. Instance
    /// restart "forgets" revocations — accepted: the 15-min TTL then governs.
    pub revoked_jtis: Arc<dashmap::DashMap<String, i64>>,
    pub platform_logs: Arc<RwLock<VecDeque<PlatformLog>>>,
    /// Short-lived cache for active user memberships, eliminating a redundant
    /// PostgreSQL round-trip on every authenticated request.
    pub active_memberships_cache: Arc<
        dashmap::DashMap<
            (pg_domain::UserId, pg_domain::OrgId, pg_domain::Role),
            (bool, time::OffsetDateTime),
        >,
    >,
}

impl AppState {
    pub fn deps(&self) -> Result<&Arc<Deps>, AppError> {
        self.deps
            .as_ref()
            .ok_or_else(|| AppError::Internal("database unavailable".into()))
    }
    pub fn db_ready(&self) -> bool {
        self.deps.is_some()
    }

    /// Revoke one access token by jti. The map is GC'd opportunistically:
    /// entries older than `now` are dead weight (their tokens expired anyway).
    pub fn revoke_jti(&self, jti: &str, exp: i64) {
        self.revoked_jtis.insert(jti.to_string(), exp);
        // Opportunistic GC every ~1024 inserts keeps the map bounded without
        // a dedicated sweeper thread.
        if self.revoked_jtis.len().is_multiple_of(1024) {
            let now = time::OffsetDateTime::now_utc().unix_timestamp();
            self.revoked_jtis.retain(|_, &mut e| e > now);
        }
    }

    /// Fast path for the extractor — true if this token was explicitly revoked.
    pub fn is_jti_revoked(&self, jti: &str) -> bool {
        self.revoked_jtis.contains_key(jti)
    }

    /// Check whether the user has an active membership, caching the result in
    /// memory for 60 seconds to eliminate physical database round-trips.
    pub async fn check_active_membership(
        &self,
        user_id: pg_domain::UserId,
        org_id: pg_domain::OrgId,
        role: pg_domain::Role,
    ) -> Result<bool, AppError> {
        let key = (user_id, org_id, role);
        let now = time::OffsetDateTime::now_utc();
        if let Some(entry) = self.active_memberships_cache.get(&key) {
            let (is_active, cached_at) = *entry;
            if (now - cached_at) < time::Duration::seconds(60) {
                return Ok(is_active);
            }
        }
        let deps = self.deps()?;
        let is_active = deps
            .user_repo
            .has_active_membership(user_id, org_id, role)
            .await?;
        self.active_memberships_cache.insert(key, (is_active, now));
        Ok(is_active)
    }

    /// Invalidate membership cache for a user (e.g. on logout or role change).
    pub fn invalidate_membership_cache(&self, user_id: pg_domain::UserId) {
        self.active_memberships_cache
            .retain(|(u, _, _), _| *u != user_id);
    }
}
