use async_trait::async_trait;
use pg_application::ports::{
    ActivateTenantRecord, InvitationEmail, InvitationEmailSender, Placement,
    TenantActivationRepository, TenantContext, UserRepository,
};
use pg_application::services::{accept_tenant_invitation, AcceptTenantInvitationCommand};
use pg_application::AppError;
use pg_domain::{OrgId, Role, UserId};
use pg_infrastructure::persistence::{
    PgTenantActivationRepository, PgTenantInvitationRepository, PgUserRepository,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

#[derive(Clone, Copy)]
struct Fixture {
    org: Uuid,
    branch: Uuid,
    room: Uuid,
    bed: Uuid,
    owner: Uuid,
    tenant: Uuid,
    invitation: Uuid,
}

struct NoopInvitationEmail;

#[async_trait]
impl InvitationEmailSender for NoopInvitationEmail {
    async fn send(&self, _email: InvitationEmail) -> Result<(), AppError> {
        Ok(())
    }
}

#[tokio::test]
async fn activation_is_atomic_and_double_approval_is_rejected() {
    let Some(pool) = integration_pool().await else {
        return;
    };
    let fixture = seed_fixture(&pool, "tenant-one@example.com", true).await;
    let repository = PgTenantActivationRepository::new(pool.clone());

    let activated = repository
        .activate(&context(fixture), &record(fixture))
        .await
        .expect("first approval activates the tenant");
    let second = repository
        .activate(&context(fixture), &record(fixture))
        .await;

    assert!(matches!(second, Err(AppError::Conflict(_))));
    assert_eq!(
        scalar_count(
            &pool,
            "SELECT count(*) FROM public.user_org_roles WHERE org_id=$1 AND role='tenant' AND membership_status='active'",
            fixture.org,
        )
        .await,
        1,
    );
    assert_eq!(
        scalar_count(
            &pool,
            "SELECT count(*) FROM pool.residents WHERE org_id=$1 AND status='active'",
            fixture.org,
        )
        .await,
        1,
    );
    let linked_membership: Option<Uuid> =
        sqlx::query_scalar("SELECT membership_id FROM pool.residents WHERE id=$1")
            .bind(activated.resident_id.as_uuid())
            .fetch_one(&pool)
            .await
            .expect("resident link");
    assert_eq!(linked_membership, Some(activated.membership_id));
    assert_eq!(invitation_status(&pool, fixture.invitation).await, "active");
    assert_eq!(
        scalar_count(
            &pool,
            "SELECT count(*) FROM pool.audit_log WHERE org_id=$1 AND action='activate_tenant'",
            fixture.org,
        )
        .await,
        1,
    );
    cleanup(&pool, fixture.org).await;
}

#[tokio::test]
async fn verified_email_mismatch_rolls_back_every_activation_write() {
    let Some(pool) = integration_pool().await else {
        return;
    };
    let fixture = seed_fixture(&pool, "tenant-two@example.com", true).await;
    sqlx::query("UPDATE public.platform_users SET email='other@example.com' WHERE id=$1")
        .bind(fixture.tenant)
        .execute(&pool)
        .await
        .expect("change email");
    let result = PgTenantActivationRepository::new(pool.clone())
        .activate(&context(fixture), &record(fixture))
        .await;

    assert!(matches!(result, Err(AppError::Forbidden)));
    assert_unchanged_after_failed_activation(&pool, fixture).await;
    cleanup(&pool, fixture.org).await;
}

#[tokio::test]
async fn occupied_bed_conflict_rolls_back_every_activation_write() {
    let Some(pool) = integration_pool().await else {
        return;
    };
    let fixture = seed_fixture(&pool, "tenant-three@example.com", true).await;
    sqlx::query(
        "INSERT INTO pool.residents (id,org_id,branch_id,bed_id,full_name,status) \
         VALUES ($1,$2,$3,$4,'Existing Resident','active')",
    )
    .bind(Uuid::new_v4())
    .bind(fixture.org)
    .bind(fixture.branch)
    .bind(fixture.bed)
    .execute(&pool)
    .await
    .expect("occupy bed");
    let result = PgTenantActivationRepository::new(pool.clone())
        .activate(&context(fixture), &record(fixture))
        .await;

    assert!(matches!(result, Err(AppError::Conflict(_))));
    assert_eq!(
        scalar_count(
            &pool,
            "SELECT count(*) FROM public.user_org_roles WHERE org_id=$1 AND role='tenant'",
            fixture.org,
        )
        .await,
        0,
    );
    assert_eq!(
        invitation_status(&pool, fixture.invitation).await,
        "approval_pending"
    );
    cleanup(&pool, fixture.org).await;
}

#[tokio::test]
async fn simultaneous_approvals_for_one_bed_allow_exactly_one_tenant() {
    let Some(pool) = integration_pool().await else {
        return;
    };
    let first = seed_fixture(&pool, "tenant-race-a@example.com", true).await;
    let second_tenant = Uuid::new_v4();
    let second_invitation = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO public.platform_users \
         (id,org_id,email,email_verified_at,full_name,status) \
         VALUES ($1,$2,'tenant-race-b@example.com',now(),'Race B','active')",
    )
    .bind(second_tenant)
    .bind(first.org)
    .execute(&pool)
    .await
    .expect("second tenant");
    insert_invitation(
        &pool,
        first,
        second_invitation,
        second_tenant,
        "tenant-race-b@example.com",
        true,
    )
    .await;
    let second = Fixture {
        tenant: second_tenant,
        invitation: second_invitation,
        ..first
    };
    let first_repository = PgTenantActivationRepository::new(pool.clone());
    let second_repository = PgTenantActivationRepository::new(pool.clone());
    let first_context = context(first);
    let second_context = context(second);
    let first_record = record(first);
    let second_record = record(second);

    let (left, right) = tokio::join!(
        first_repository.activate(&first_context, &first_record),
        second_repository.activate(&second_context, &second_record),
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert!(left.is_ok() || matches!(left, Err(AppError::Conflict(_))));
    assert!(right.is_ok() || matches!(right, Err(AppError::Conflict(_))));
    assert_eq!(
        scalar_count(
            &pool,
            "SELECT count(*) FROM pool.residents WHERE org_id=$1 AND bed_id IS NOT NULL AND status='active'",
            first.org,
        )
        .await,
        1,
    );
    cleanup(&pool, first.org).await;
}

#[tokio::test]
async fn pending_membership_cannot_enter_tenant_context_until_activated() {
    let Some(pool) = integration_pool().await else {
        return;
    };
    let fixture = seed_fixture(&pool, "tenant-pending@example.com", false).await;
    sqlx::query(
        "INSERT INTO public.user_org_roles \
         (user_id,org_id,role,branch_id,membership_status) VALUES ($1,$2,'tenant',$3,'pending')",
    )
    .bind(fixture.tenant)
    .bind(fixture.org)
    .bind(fixture.branch)
    .execute(&pool)
    .await
    .expect("pending membership");
    let users = PgUserRepository::new(pool.clone());

    assert!(!users
        .has_active_membership(
            UserId::new(fixture.tenant),
            OrgId::new(fixture.org),
            Role::Tenant,
        )
        .await
        .expect("pending check"));
    assert!(users
        .list_active_memberships(UserId::new(fixture.tenant))
        .await
        .expect("pending list")
        .is_empty());
    sqlx::query(
        "UPDATE public.user_org_roles SET membership_status='active' \
         WHERE user_id=$1 AND org_id=$2 AND role='tenant'",
    )
    .bind(fixture.tenant)
    .bind(fixture.org)
    .execute(&pool)
    .await
    .expect("activate membership");
    assert!(users
        .has_active_membership(
            UserId::new(fixture.tenant),
            OrgId::new(fixture.org),
            Role::Tenant,
        )
        .await
        .expect("active check"));
    cleanup(&pool, fixture.org).await;
}

#[tokio::test]
async fn manager_branch_lookup_is_scoped_to_the_selected_organization() {
    let Some(pool) = integration_pool().await else {
        return;
    };
    let first = seed_fixture(&pool, "multi-manager@example.com", false).await;
    let second_org = Uuid::new_v4();
    let second_branch = Uuid::new_v4();
    sqlx::query("INSERT INTO public.organizations (id,name) VALUES ($1,'Second Manager Org')")
        .bind(second_org)
        .execute(&pool)
        .await
        .expect("second org");
    sqlx::query(
        "INSERT INTO public.tenant_placement (org_id,mode,schema_name) VALUES ($1,'pooled','pool')",
    )
    .bind(second_org)
    .execute(&pool)
    .await
    .expect("second placement");
    sqlx::query("INSERT INTO pool.branches (id,org_id,name) VALUES ($1,$2,'Second Branch')")
        .bind(second_branch)
        .bind(second_org)
        .execute(&pool)
        .await
        .expect("second branch");
    sqlx::query(
        "INSERT INTO public.user_org_roles \
         (user_id,org_id,role,branch_id,membership_status) \
         VALUES ($1,$2,'manager',$3,'active'),($1,$4,'manager',$5,'active')",
    )
    .bind(first.tenant)
    .bind(first.org)
    .bind(first.branch)
    .bind(second_org)
    .bind(second_branch)
    .execute(&pool)
    .await
    .expect("manager memberships");
    let users = PgUserRepository::new(pool.clone());

    assert_eq!(
        users
            .branch_in_org(UserId::new(first.tenant), OrgId::new(first.org))
            .await
            .expect("first context")
            .map(|id| id.as_uuid()),
        Some(first.branch),
    );
    assert_eq!(
        users
            .branch_in_org(UserId::new(first.tenant), OrgId::new(second_org))
            .await
            .expect("second context")
            .map(|id| id.as_uuid()),
        Some(second_branch),
    );
    cleanup(&pool, second_org).await;
    cleanup(&pool, first.org).await;
}

#[tokio::test]
async fn database_expiry_and_compare_and_set_prevent_invalid_or_double_acceptance() {
    let Some(pool) = integration_pool().await else {
        return;
    };
    let fixture = seed_fixture(&pool, "acceptance-db@example.com", false).await;
    insert_invited(&pool, fixture, false).await;
    let invitations = PgTenantInvitationRepository::new(pool.clone());
    let users = PgUserRepository::new(pool.clone());
    let email_sender = NoopInvitationEmail;
    let command = || AcceptTenantInvitationCommand {
        invitations: &invitations,
        users: &users,
        email_sender: &email_sender,
        actor_id: UserId::new(fixture.tenant),
        actor_role: Role::Prospect,
        invitation_id: fixture.invitation,
    };

    let expired = accept_tenant_invitation(command()).await;
    assert!(matches!(expired, Err(AppError::Conflict(_))));
    assert_eq!(
        invitation_status(&pool, fixture.invitation).await,
        "invited"
    );

    sqlx::query(
        "UPDATE public.tenant_invitations SET expires_at=now()+interval '1 day' WHERE id=$1",
    )
    .bind(fixture.invitation)
    .execute(&pool)
    .await
    .expect("renew fixture");
    accept_tenant_invitation(command())
        .await
        .expect("first acceptance");
    let duplicate = accept_tenant_invitation(command()).await;
    assert!(matches!(duplicate, Err(AppError::Conflict(_))));
    assert_eq!(
        invitation_status(&pool, fixture.invitation).await,
        "approval_pending",
    );
    cleanup(&pool, fixture.org).await;
}

async fn integration_pool() -> Option<PgPool> {
    let database_url = std::env::var("MIGRATION_DATABASE_URL").ok()?;
    Some(
        PgPoolOptions::new()
            .max_connections(8)
            .connect(&database_url)
            .await
            .expect("connect integration database"),
    )
}

async fn seed_fixture(pool: &PgPool, email: &str, with_invitation: bool) -> Fixture {
    let fixture = Fixture {
        org: Uuid::new_v4(),
        branch: Uuid::new_v4(),
        room: Uuid::new_v4(),
        bed: Uuid::new_v4(),
        owner: Uuid::new_v4(),
        tenant: Uuid::new_v4(),
        invitation: Uuid::new_v4(),
    };
    let owner_email = format!("owner-{}@example.com", fixture.owner);
    sqlx::query("INSERT INTO public.organizations (id,name) VALUES ($1,'Activation QA')")
        .bind(fixture.org)
        .execute(pool)
        .await
        .expect("organization");
    sqlx::query(
        "INSERT INTO public.tenant_placement (org_id,mode,schema_name) VALUES ($1,'pooled','pool')",
    )
    .bind(fixture.org)
    .execute(pool)
    .await
    .expect("placement");
    sqlx::query(
        "INSERT INTO public.platform_users (id,org_id,email,email_verified_at,full_name,status) \
         VALUES ($1,$2,$5,now(),'Owner','active'), \
                ($3,$2,$4,now(),'Tenant','active')",
    )
    .bind(fixture.owner)
    .bind(fixture.org)
    .bind(fixture.tenant)
    .bind(email)
    .bind(owner_email)
    .execute(pool)
    .await
    .expect("users");
    sqlx::query("INSERT INTO pool.branches (id,org_id,name) VALUES ($1,$2,'Main Branch')")
        .bind(fixture.branch)
        .bind(fixture.org)
        .execute(pool)
        .await
        .expect("branch");
    sqlx::query(
        "INSERT INTO pool.rooms (id,org_id,branch_id,number,cooling_type) \
         VALUES ($1,$2,$3,'101','ac')",
    )
    .bind(fixture.room)
    .bind(fixture.org)
    .bind(fixture.branch)
    .execute(pool)
    .await
    .expect("room");
    sqlx::query("INSERT INTO pool.beds (id,org_id,room_id,label) VALUES ($1,$2,$3,'A')")
        .bind(fixture.bed)
        .bind(fixture.org)
        .bind(fixture.room)
        .execute(pool)
        .await
        .expect("bed");
    if with_invitation {
        insert_invitation(
            pool,
            fixture,
            fixture.invitation,
            fixture.tenant,
            email,
            true,
        )
        .await;
    }
    fixture
}

async fn insert_invitation(
    pool: &PgPool,
    fixture: Fixture,
    invitation: Uuid,
    tenant: Uuid,
    email: &str,
    include_bed: bool,
) {
    sqlx::query(
        "INSERT INTO public.tenant_invitations \
         (id,org_id,branch_id,intended_room_id,intended_bed_id,email,full_name,status, \
          invited_by_user_id,accepted_by_user_id,expires_at,accepted_at,last_sent_at) \
         VALUES ($1,$2,$3,$4,$5,$6,'Tenant','approval_pending',$7,$8,now()+interval '7 days',now(),now())",
    )
    .bind(invitation)
    .bind(fixture.org)
    .bind(fixture.branch)
    .bind(fixture.room)
    .bind(include_bed.then_some(fixture.bed))
    .bind(email)
    .bind(fixture.owner)
    .bind(tenant)
    .execute(pool)
    .await
    .expect("invitation");
}

async fn insert_invited(pool: &PgPool, fixture: Fixture, active: bool) {
    let expiry = if active {
        "now()+interval '7 days'"
    } else {
        "now()-interval '1 second'"
    };
    let query = format!(
        "INSERT INTO public.tenant_invitations \
         (id,org_id,branch_id,email,full_name,status,invited_by_user_id,expires_at,last_sent_at) \
         VALUES ($1,$2,$3,'acceptance-db@example.com','Tenant','invited',$4,{expiry},now())"
    );
    sqlx::query(&query)
        .bind(fixture.invitation)
        .bind(fixture.org)
        .bind(fixture.branch)
        .bind(fixture.owner)
        .execute(pool)
        .await
        .expect("invited fixture");
}

fn context(fixture: Fixture) -> TenantContext {
    TenantContext {
        org_id: OrgId::new(fixture.org),
        placement: Placement::Pooled,
    }
}

fn record(fixture: Fixture) -> ActivateTenantRecord {
    ActivateTenantRecord {
        invitation_id: fixture.invitation,
        approved_by_user_id: UserId::new(fixture.owner),
        request_id: Some(Uuid::new_v4()),
    }
}

async fn invitation_status(pool: &PgPool, invitation: Uuid) -> String {
    sqlx::query_scalar("SELECT status FROM public.tenant_invitations WHERE id=$1")
        .bind(invitation)
        .fetch_one(pool)
        .await
        .expect("invitation status")
}

async fn scalar_count(pool: &PgPool, query: &str, org: Uuid) -> i64 {
    sqlx::query_scalar(query)
        .bind(org)
        .fetch_one(pool)
        .await
        .expect("count")
}

async fn assert_unchanged_after_failed_activation(pool: &PgPool, fixture: Fixture) {
    assert_eq!(
        scalar_count(
            pool,
            "SELECT count(*) FROM public.user_org_roles WHERE org_id=$1 AND role='tenant'",
            fixture.org,
        )
        .await,
        0,
    );
    assert_eq!(
        scalar_count(
            pool,
            "SELECT count(*) FROM pool.residents WHERE org_id=$1",
            fixture.org,
        )
        .await,
        0,
    );
    assert_eq!(
        scalar_count(
            pool,
            "SELECT count(*) FROM pool.audit_log WHERE org_id=$1",
            fixture.org,
        )
        .await,
        0,
    );
    assert_eq!(
        invitation_status(pool, fixture.invitation).await,
        "approval_pending"
    );
}

async fn cleanup(pool: &PgPool, org: Uuid) {
    for table in [
        "pool.audit_log",
        "pool.residents",
        "pool.beds",
        "pool.rooms",
        "pool.branches",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE org_id=$1"))
            .bind(org)
            .execute(pool)
            .await
            .expect("tenant cleanup");
    }
    sqlx::query(
        "DELETE FROM public.manual_kyc_reviews WHERE invitation_id IN \
         (SELECT id FROM public.tenant_invitations WHERE org_id=$1)",
    )
    .bind(org)
    .execute(pool)
    .await
    .expect("manual review cleanup");
    sqlx::query(
        "DELETE FROM public.invitation_identity_verifications WHERE invitation_id IN \
         (SELECT id FROM public.tenant_invitations WHERE org_id=$1)",
    )
    .bind(org)
    .execute(pool)
    .await
    .expect("verification cleanup");
    sqlx::query("DELETE FROM public.tenant_invitations WHERE org_id=$1")
        .bind(org)
        .execute(pool)
        .await
        .expect("invitation cleanup");
    sqlx::query("DELETE FROM public.user_org_roles WHERE org_id=$1")
        .bind(org)
        .execute(pool)
        .await
        .expect("membership cleanup");
    sqlx::query("DELETE FROM public.email_otps WHERE user_id IN (SELECT id FROM public.platform_users WHERE org_id=$1)")
        .bind(org)
        .execute(pool)
        .await
        .expect("otp cleanup");
    sqlx::query("DELETE FROM public.platform_users WHERE org_id=$1")
        .bind(org)
        .execute(pool)
        .await
        .expect("user cleanup");
    sqlx::query("DELETE FROM public.tenant_placement WHERE org_id=$1")
        .bind(org)
        .execute(pool)
        .await
        .expect("placement cleanup");
    sqlx::query("DELETE FROM public.organizations WHERE id=$1")
        .bind(org)
        .execute(pool)
        .await
        .expect("org cleanup");
}
