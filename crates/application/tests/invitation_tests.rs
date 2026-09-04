mod fakes;

use async_trait::async_trait;
use fakes::FakeUserRepo;
use pg_application::ports::repositories::{TenantInvitation, TenantInvitationRepository};
use pg_application::ports::{
    ActivateTenantRecord, DigiLockerSession, InvitationEmail, InvitationEmailSender,
    InvitationVerification, InvitationVerificationRepository, LoginRecord, NewDigiLockerSession,
    Placement, TenantActivationRepository, TenantActivationResult, TenantContext,
};
use pg_application::services::{
    accept_tenant_invitation, approve_tenant_invitation, list_tenant_invitations,
    manual_kyc_review, AcceptTenantInvitationCommand, ApproveTenantInvitationCommand,
    ListTenantInvitationsCommand, ManualKycReviewCommand,
};
use pg_application::AppError;
use pg_domain::{BranchId, InvitationStatus, OrgId, ResidentId, Role, UserId, UserStatus};
use std::sync::Mutex;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

struct InvitationRepo {
    item: Mutex<TenantInvitation>,
}

impl InvitationRepo {
    fn new(item: TenantInvitation) -> Self {
        Self {
            item: Mutex::new(item),
        }
    }
}

fn copy_invitation(value: &TenantInvitation) -> TenantInvitation {
    TenantInvitation {
        id: value.id,
        org_id: value.org_id,
        branch_id: value.branch_id,
        intended_room_id: value.intended_room_id,
        intended_bed_id: value.intended_bed_id,
        email: value.email.clone(),
        full_name: value.full_name.clone(),
        plan_kind: value.plan_kind.clone(),
        monthly_rent_paise: value.monthly_rent_paise,
        deposit_paise: value.deposit_paise,
        planned_move_in_date: value.planned_move_in_date,
        require_identity_verification: value.require_identity_verification,
        status: value.status,
        accepted_by_user_id: value.accepted_by_user_id,
        expires_at: value.expires_at,
        last_sent_at: value.last_sent_at,
        accepted_at: value.accepted_at,
        approved_at: value.approved_at,
        approved_by_user_id: value.approved_by_user_id,
    }
}

#[async_trait]
impl TenantInvitationRepository for InvitationRepo {
    async fn create(
        &self,
        _invitation: &TenantInvitation,
        _invited_by: UserId,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn get(&self, id: Uuid) -> Result<Option<TenantInvitation>, AppError> {
        let item = self.item.lock().expect("item");
        Ok((item.id == id).then(|| copy_invitation(&item)))
    }
    async fn list_for_org(&self, org_id: OrgId) -> Result<Vec<TenantInvitation>, AppError> {
        let item = self.item.lock().expect("item");
        Ok((item.org_id == org_id)
            .then(|| copy_invitation(&item))
            .into_iter()
            .collect())
    }
    async fn list_for_email(&self, email: &str) -> Result<Vec<TenantInvitation>, AppError> {
        let item = self.item.lock().expect("item");
        Ok(item
            .email
            .eq_ignore_ascii_case(email)
            .then(|| copy_invitation(&item))
            .into_iter()
            .collect())
    }
    async fn accept(&self, id: Uuid, user_id: UserId, email: &str) -> Result<bool, AppError> {
        let mut item = self.item.lock().expect("item");
        if item.id != id
            || item.status != InvitationStatus::Invited
            || !item.email.eq_ignore_ascii_case(email)
        {
            return Ok(false);
        }
        item.status = InvitationStatus::EmailVerified;
        item.accepted_by_user_id = Some(user_id);
        item.accepted_at = Some(OffsetDateTime::now_utc());
        Ok(true)
    }
    async fn decline(
        &self,
        id: Uuid,
        email: &str,
        from: InvitationStatus,
    ) -> Result<bool, AppError> {
        let mut item = self.item.lock().expect("item");
        if item.id == id && item.status == from && item.email.eq_ignore_ascii_case(email) {
            item.status = InvitationStatus::Cancelled;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn transition(
        &self,
        id: Uuid,
        from: InvitationStatus,
        to: InvitationStatus,
        _actor: UserId,
    ) -> Result<bool, AppError> {
        let mut item = self.item.lock().expect("item");
        if item.id == id && item.status == from {
            item.status = to;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn resend(
        &self,
        _id: Uuid,
        _expected_status: InvitationStatus,
        _expires_at: OffsetDateTime,
        _not_sent_after: OffsetDateTime,
    ) -> Result<bool, AppError> {
        Ok(true)
    }
}

#[derive(Default)]
struct EmailSpy(Mutex<Vec<String>>);

#[async_trait]
impl InvitationEmailSender for EmailSpy {
    async fn send(&self, email: InvitationEmail) -> Result<(), AppError> {
        self.0
            .lock()
            .expect("email spy")
            .push(email.kind.as_str().to_string());
        Ok(())
    }
}

#[derive(Default)]
struct ActivationSpy(Mutex<u32>);

#[async_trait]
impl TenantActivationRepository for ActivationSpy {
    async fn activate(
        &self,
        _ctx: &TenantContext,
        _record: &ActivateTenantRecord,
    ) -> Result<TenantActivationResult, AppError> {
        *self.0.lock().expect("activation spy") += 1;
        Ok(TenantActivationResult {
            membership_id: Uuid::new_v4(),
            resident_id: ResidentId::new(Uuid::new_v4()),
        })
    }
}

struct RecordedManualReview {
    reviewer: UserId,
    approved: bool,
    reason: String,
    request_id: Option<Uuid>,
}

#[derive(Default)]
struct ManualReviewSpy(Mutex<Vec<RecordedManualReview>>);

#[async_trait]
impl InvitationVerificationRepository for ManualReviewSpy {
    async fn create_digilocker_session(
        &self,
        _session: NewDigiLockerSession<'_>,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn consume_digilocker_session(
        &self,
        _state_hash: &str,
        _encryption_key: &str,
    ) -> Result<Option<DigiLockerSession>, AppError> {
        Ok(None)
    }
    async fn complete_digilocker(
        &self,
        _invitation_id: Uuid,
        _provider_reference: Option<&str>,
        _document_type: Option<&str>,
        _verified: bool,
        _safe_failure_reason: Option<&str>,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn latest(
        &self,
        _invitation_id: Uuid,
    ) -> Result<Option<InvitationVerification>, AppError> {
        Ok(None)
    }
    async fn manual_review(
        &self,
        _invitation_id: Uuid,
        reviewer_user_id: UserId,
        approved: bool,
        reason: &str,
        request_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        self.0.lock().expect("reviews").push(RecordedManualReview {
            reviewer: reviewer_user_id,
            approved,
            reason: reason.to_string(),
            request_id,
        });
        Ok(())
    }
}

fn invitation(status: InvitationStatus, require_kyc: bool) -> TenantInvitation {
    let now = OffsetDateTime::now_utc();
    TenantInvitation {
        id: Uuid::new_v4(),
        org_id: OrgId::new(Uuid::new_v4()),
        branch_id: BranchId::new(Uuid::new_v4()),
        intended_room_id: None,
        intended_bed_id: None,
        email: "tenant@example.com".into(),
        full_name: "Tenant Person".into(),
        plan_kind: "monthly".into(),
        monthly_rent_paise: Some(900_000),
        deposit_paise: Some(1_800_000),
        planned_move_in_date: None,
        require_identity_verification: require_kyc,
        status,
        accepted_by_user_id: None,
        expires_at: now + Duration::days(7),
        last_sent_at: now - Duration::minutes(5),
        accepted_at: None,
        approved_at: None,
        approved_by_user_id: None,
    }
}

fn verified_user(users: &FakeUserRepo, email: &str) -> UserId {
    let id = UserId::new(Uuid::new_v4());
    users.insert(
        email,
        LoginRecord {
            user_id: id,
            org_id: OrgId::new(Uuid::new_v4()),
            role: Role::Prospect,
            status: UserStatus::Active,
            full_name: None,
        },
    );
    id
}

#[tokio::test]
async fn accepting_a_kyc_required_invitation_enters_kyc_pending() {
    let users = FakeUserRepo::new();
    let user = verified_user(&users, "tenant@example.com");
    let item = invitation(InvitationStatus::Invited, true);
    let id = item.id;
    let repo = InvitationRepo::new(item);
    let email = EmailSpy::default();

    let status = accept_tenant_invitation(AcceptTenantInvitationCommand {
        invitations: &repo,
        users: &users,
        email_sender: &email,
        actor_id: user,
        actor_role: Role::Prospect,
        invitation_id: id,
    })
    .await
    .expect("accept");

    assert_eq!(status, InvitationStatus::KycPending);
    assert_eq!(*email.0.lock().expect("email"), vec!["kyc_required"]);
}

#[tokio::test]
async fn accepting_without_kyc_enters_approval_pending() {
    let users = FakeUserRepo::new();
    let user = verified_user(&users, "tenant@example.com");
    let item = invitation(InvitationStatus::Invited, false);
    let id = item.id;
    let repo = InvitationRepo::new(item);

    let status = accept_tenant_invitation(AcceptTenantInvitationCommand {
        invitations: &repo,
        users: &users,
        email_sender: &EmailSpy::default(),
        actor_id: user,
        actor_role: Role::Prospect,
        invitation_id: id,
    })
    .await
    .expect("accept");
    assert_eq!(status, InvitationStatus::ApprovalPending);
}

#[tokio::test]
async fn email_mismatch_expiry_and_double_acceptance_are_rejected() {
    let users = FakeUserRepo::new();
    let mismatched = verified_user(&users, "different@example.com");
    let item = invitation(InvitationStatus::Invited, false);
    let id = item.id;
    let repo = InvitationRepo::new(item);
    let result = accept_tenant_invitation(AcceptTenantInvitationCommand {
        invitations: &repo,
        users: &users,
        email_sender: &EmailSpy::default(),
        actor_id: mismatched,
        actor_role: Role::Prospect,
        invitation_id: id,
    })
    .await;
    assert!(matches!(result, Err(AppError::Forbidden)));

    let users = FakeUserRepo::new();
    let user = verified_user(&users, "tenant@example.com");
    repo.item.lock().expect("item").expires_at = OffsetDateTime::now_utc() - Duration::seconds(1);
    let result = accept_tenant_invitation(AcceptTenantInvitationCommand {
        invitations: &repo,
        users: &users,
        email_sender: &EmailSpy::default(),
        actor_id: user,
        actor_role: Role::Prospect,
        invitation_id: id,
    })
    .await;
    assert!(matches!(result, Err(AppError::Conflict(_))));

    {
        let mut row = repo.item.lock().expect("item");
        row.expires_at = OffsetDateTime::now_utc() + Duration::days(1);
        row.status = InvitationStatus::ApprovalPending;
    }
    let result = accept_tenant_invitation(AcceptTenantInvitationCommand {
        invitations: &repo,
        users: &users,
        email_sender: &EmailSpy::default(),
        actor_id: user,
        actor_role: Role::Prospect,
        invitation_id: id,
    })
    .await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn manager_list_is_branch_scoped_and_cross_org_owner_is_blocked_on_approval() {
    let users = FakeUserRepo::new();
    let manager = UserId::new(Uuid::new_v4());
    let item = invitation(InvitationStatus::Invited, false);
    let branch = item.branch_id;
    let org = item.org_id;
    users
        .branch_scopes
        .lock()
        .expect("branches")
        .insert(manager, BranchId::new(Uuid::new_v4()));
    let repo = InvitationRepo::new(item);
    let visible = list_tenant_invitations(ListTenantInvitationsCommand {
        invitations: &repo,
        users: &users,
        actor_id: manager,
        actor_org_id: org,
        actor_role: Role::Manager,
    })
    .await
    .expect("list");
    assert!(visible.is_empty());
    users
        .branch_scopes
        .lock()
        .expect("branches")
        .insert(manager, branch);
    let visible = list_tenant_invitations(ListTenantInvitationsCommand {
        invitations: &repo,
        users: &users,
        actor_id: manager,
        actor_org_id: org,
        actor_role: Role::Manager,
    })
    .await
    .expect("list");
    assert_eq!(visible.len(), 1);

    let invitation_id = repo.item.lock().expect("item").id;
    let result = approve_tenant_invitation(ApproveTenantInvitationCommand {
        invitations: &repo,
        activation: &ActivationSpy::default(),
        users: &users,
        email_sender: &EmailSpy::default(),
        ctx: &TenantContext {
            org_id: OrgId::new(Uuid::new_v4()),
            placement: Placement::Pooled,
        },
        actor_id: UserId::new(Uuid::new_v4()),
        actor_org_id: OrgId::new(Uuid::new_v4()),
        actor_role: Role::Owner,
        invitation_id,
        request_id: None,
    })
    .await;
    assert!(matches!(result, Err(AppError::Forbidden)));
}

#[tokio::test]
async fn approval_requires_ready_state_and_prevents_self_approval() {
    let users = FakeUserRepo::new();
    let actor = UserId::new(Uuid::new_v4());
    let mut item = invitation(InvitationStatus::ApprovalPending, false);
    item.accepted_by_user_id = Some(actor);
    let id = item.id;
    let org = item.org_id;
    let repo = InvitationRepo::new(item);
    let activation = ActivationSpy::default();
    let result = approve_tenant_invitation(ApproveTenantInvitationCommand {
        invitations: &repo,
        activation: &activation,
        users: &users,
        email_sender: &EmailSpy::default(),
        ctx: &TenantContext {
            org_id: org,
            placement: Placement::Pooled,
        },
        actor_id: actor,
        actor_org_id: org,
        actor_role: Role::Owner,
        invitation_id: id,
        request_id: None,
    })
    .await;
    assert!(matches!(result, Err(AppError::Forbidden)));
    assert_eq!(*activation.0.lock().expect("activation"), 0);
}

#[tokio::test]
async fn manual_kyc_fallback_is_config_gated_and_manager_branch_scoped() {
    let users = FakeUserRepo::new();
    let manager = UserId::new(Uuid::new_v4());
    let item = invitation(InvitationStatus::KycPending, true);
    let id = item.id;
    let org = item.org_id;
    let branch = item.branch_id;
    let repo = InvitationRepo::new(item);
    let reviews = ManualReviewSpy::default();
    users
        .branch_scopes
        .lock()
        .expect("branch scopes")
        .insert(manager, branch);

    manual_kyc_review(ManualKycReviewCommand {
        invitations: &repo,
        users: &users,
        verifications: &reviews,
        actor_id: manager,
        actor_org_id: org,
        actor_role: Role::Manager,
        invitation_id: id,
        approved: true,
        reason: "Government ID checked in person".into(),
        request_id: Some(Uuid::new_v4()),
        manual_review_allowed: true,
    })
    .await
    .expect("manual review");
    {
        let recorded = reviews.0.lock().expect("reviews");
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].reviewer, manager);
        assert!(recorded[0].approved);
        assert_eq!(recorded[0].reason, "Government ID checked in person");
        assert!(recorded[0].request_id.is_some());
    }

    let blocked = manual_kyc_review(ManualKycReviewCommand {
        invitations: &repo,
        users: &users,
        verifications: &reviews,
        actor_id: manager,
        actor_org_id: org,
        actor_role: Role::Manager,
        invitation_id: id,
        approved: true,
        reason: "Government ID checked in person".into(),
        request_id: None,
        manual_review_allowed: false,
    })
    .await;
    assert!(matches!(blocked, Err(AppError::Forbidden)));
}
