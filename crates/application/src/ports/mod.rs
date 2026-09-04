//! Ports: the trait boundary between the application core and the outside world.
pub mod auth;
pub mod billing;
pub mod clock;
pub mod discovery;
pub mod email;
pub mod google;
pub mod notifier;
pub mod otp;
pub mod repositories;
pub mod storage;
pub mod tenancy;
pub mod verification;

pub use auth::{Claims, TokenKind, TokenPair, TokenService};
pub use billing::BillingEngine;
pub use clock::{Clock, SystemClock};
pub use discovery::{Listing, ListingInput, ListingRepository, ListingStatus};
pub use email::{EmailSender, InvitationEmail, InvitationEmailKind, InvitationEmailSender};
pub use google::{GoogleVerifier, VerifiedGoogleUser};
pub use notifier::Notifier;
pub use otp::OtpCodeGenerator;
pub use repositories::{
    ActivateTenantRecord, ArchiveRepository, AuditActor, AuditRepository, AuditSummary, AwayPeriod,
    AwayPeriodDetail, AwayRepository, BedRepository, BedSummary, BranchFinance, BranchRepository,
    BranchSummary, ChangeRequestRepository, ChangeRequestSummary, ChangeTarget,
    ComplaintRepository, ComplaintStateRow, ComplaintSummary, DeviceTokenRepository,
    EmailOtpRepository, EnquiryRecord, EnquiryRepository, FinanceReport, FinanceRepository,
    FloorRepository, FloorSummary, LoginRecord, MembershipContext, NewRoomSpec, NoticeRepository,
    NoticeSummary, OrganizationRepository, PaymentRepository, ReceiptFile, ReceiptMeta,
    ReceiptTemplateRepository, RecordedPayment, ResidentDetail, ResidentRepository,
    ResidentSummary, RoomRepository, RoomSummary, SettlementRecord, SettlementRepository,
    TenantActivationRepository, TenantActivationResult, TrustedDeviceRepository, UserRepository,
    VacancyRepository, VacancySnapshot, VerificationRepository, VoidedPayment,
};
pub use storage::{ObjectStore, ReceiptData, ReceiptRenderer, ReceiptTemplate};
pub use tenancy::{Placement, TenancyError, TenantContext, TenantProvisioner, TenantResolver};
pub use verification::{
    DigiLockerClient, DigiLockerIdentity, DigiLockerSession, IdentityVerifier,
    InvitationVerification, InvitationVerificationRepository, NewDigiLockerSession,
    VerificationOutcome, VerificationRequest, VerificationStatus,
};
