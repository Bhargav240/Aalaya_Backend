//! Away days: purely a safety notification. Rent/ledger untouched (by design).
#![allow(non_snake_case)]
mod fakes;

use fakes::*;
use pg_application::error::AppError;
use pg_application::ports::tenancy::{Placement, TenantContext};
use pg_application::ports::AuditActor;
use pg_application::services::away;
use pg_domain::{BranchId, OrgId, ResidentId, Role, UserId};
use time::macros::date;
use uuid::Uuid;

fn ctx() -> TenantContext {
    TenantContext {
        org_id: OrgId::new(Uuid::new_v4()),
        placement: Placement::Pooled,
    }
}
fn actor() -> AuditActor {
    AuditActor {
        user_id: None,
        request_id: None,
    }
}

#[tokio::test]
async fn report__past_end_date__rejected() {
    let clock = FixedClock::at(date!(2024 - 01 - 20));
    let err = away::report(away::ReportAwayCommand {
        away_repo: &FakeAwayRepo::new(),
        resident_repo: &FakeResidentRepo::new(),
        clock: &clock,
        ctx: &ctx(),
        user_id: UserId::new(Uuid::new_v4()),
        actor: &actor(),
        input: away::AwayPeriodInput {
            from_date: date!(2024 - 01 - 10),
            to_date: date!(2024 - 01 - 12), // already over
            note: None,
        },
    })
    .await
    .expect_err("past period");
    assert!(
        matches!(err, AppError::Forbidden),
        "no resident row → Forbidden"
    );
}

#[tokio::test]
async fn report__non_tenant__forbidden() {
    // Owners/managers don't live in beds: no resident row → forbidden.
    let clock = FixedClock::at(date!(2024 - 01 - 01));
    let err = away::report(away::ReportAwayCommand {
        away_repo: &FakeAwayRepo::new(),
        resident_repo: &FakeResidentRepo::new(),
        clock: &clock,
        ctx: &ctx(),
        user_id: UserId::new(Uuid::new_v4()),
        actor: &actor(),
        input: away::AwayPeriodInput {
            from_date: date!(2024 - 02 - 01),
            to_date: date!(2024 - 02 - 05),
            note: None,
        },
    })
    .await
    .expect_err("non-tenant");
    assert!(matches!(err, AppError::Forbidden));
}

#[tokio::test]
async fn credit_is_not_a_thing__away_does_not_reduce_rent__by_design() {
    // Boundary test: if a future change re-links away days to rent, this test
    // fails loudly. Away days are a safety notification ONLY.
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/money.rs"
    ))
    .expect("money.rs");
    assert!(
        !source.contains("away"),
        "money.rs must never reference away days — rent only changes at move-out"
    );
}

#[tokio::test]
async fn list_all__owner__sees_the_whole_org() {
    let away = FakeAwayRepo::new();
    away::list_all(
        &away,
        &FakeUserRepo::default(),
        &ctx(),
        Role::Owner,
        UserId::new(Uuid::new_v4()),
    )
    .await
    .expect("owner may list");
    assert_eq!(
        *away.detailed_scopes.lock().expect("poisoned"),
        vec![None],
        "an owner is not branch-scoped"
    );
}

#[tokio::test]
async fn list_all__manager__is_scoped_to_their_own_branch() {
    // The manager on site is exactly who needs to know a bed is deliberately
    // empty, but only for the branch they run.
    let away = FakeAwayRepo::new();
    let users = FakeUserRepo::default();
    let user_id = UserId::new(Uuid::new_v4());
    let branch = BranchId::new(Uuid::new_v4());
    users
        .branch_scopes
        .lock()
        .expect("poisoned")
        .insert(user_id, branch);

    away::list_all(&away, &users, &ctx(), Role::Manager, user_id)
        .await
        .expect("manager may list");
    assert_eq!(
        *away.detailed_scopes.lock().expect("poisoned"),
        vec![Some(branch)],
        "a manager must never receive other branches' residents"
    );
}

#[tokio::test]
async fn list_all__manager_with_no_branch__forbidden() {
    // `None` must mean "owner, unscoped" — never "manager, unscoped", or an
    // unassigned manager would silently read the entire org.
    let away = FakeAwayRepo::new();
    let err = away::list_all(
        &away,
        &FakeUserRepo::default(),
        &ctx(),
        Role::Manager,
        UserId::new(Uuid::new_v4()),
    )
    .await
    .expect_err("no branch assigned");
    assert!(matches!(err, AppError::Forbidden));
    assert!(
        away.detailed_scopes.lock().expect("poisoned").is_empty(),
        "must reject before querying"
    );
}

#[tokio::test]
async fn list_all__tenant__forbidden() {
    let err = away::list_all(
        &FakeAwayRepo::new(),
        &FakeUserRepo::default(),
        &ctx(),
        Role::Tenant,
        UserId::new(Uuid::new_v4()),
    )
    .await
    .expect_err("tenant");
    assert!(matches!(err, AppError::Forbidden));
}

/// Seeds a tenant with a resident row and returns (user_id, repos).
fn tenant_with_resident() -> (UserId, FakeResidentRepo) {
    let user_id = UserId::new(Uuid::new_v4());
    let residents = FakeResidentRepo::new();
    residents.seed_link(user_id.as_uuid(), ResidentId::new(Uuid::new_v4()));
    (user_id, residents)
}

#[tokio::test]
async fn update__user_without_resident_row__forbidden() {
    let err = away::update(
        &FakeAwayRepo::new(),
        &FakeResidentRepo::new(),
        &FixedClock::at(date!(2024 - 01 - 01)),
        &ctx(),
        UserId::new(Uuid::new_v4()),
        &actor(),
        Uuid::new_v4(),
        away::AwayPeriodInput {
            from_date: date!(2024 - 02 - 01),
            to_date: date!(2024 - 02 - 05),
            note: None,
        },
    )
    .await
    .expect_err("no resident");
    assert!(matches!(err, AppError::Forbidden));
}

#[tokio::test]
async fn update__return_before_departure__rejected() {
    // A correction can be just as wrong as the original report, so an edit
    // re-runs every rule instead of trusting the row already passed once.
    let (user_id, residents) = tenant_with_resident();
    let err = away::update(
        &FakeAwayRepo::new(),
        &residents,
        &FixedClock::at(date!(2024 - 01 - 01)),
        &ctx(),
        user_id,
        &actor(),
        Uuid::new_v4(),
        away::AwayPeriodInput {
            from_date: date!(2024 - 02 - 10),
            to_date: date!(2024 - 02 - 01),
            note: None,
        },
    )
    .await
    .expect_err("backwards range");
    assert!(matches!(err, AppError::Validation(_)));
}

#[tokio::test]
async fn update__longer_than_the_cap__rejected() {
    let (user_id, residents) = tenant_with_resident();
    let err = away::update(
        &FakeAwayRepo::new(),
        &residents,
        &FixedClock::at(date!(2024 - 01 - 01)),
        &ctx(),
        user_id,
        &actor(),
        Uuid::new_v4(),
        away::AwayPeriodInput {
            from_date: date!(2024 - 02 - 01),
            to_date: date!(2024 - 12 - 01),
            note: None,
        },
    )
    .await
    .expect_err("over the cap");
    assert!(matches!(err, AppError::Validation(_)));
}

#[tokio::test]
async fn update__does_not_collide_with_the_row_being_edited() {
    // The regression this guards: the period being corrected is itself an
    // active period covering these dates, so an overlap check that does not
    // exclude it rejects every single edit as a conflict.
    let (user_id, residents) = tenant_with_resident();
    let away = FakeAwayRepo::new();
    let period_id = Uuid::new_v4();
    away.seed(pg_application::ports::AwayPeriod {
        id: period_id,
        resident_id: Uuid::new_v4(),
        from_date: date!(2024 - 02 - 01),
        to_date: date!(2024 - 02 - 10),
        note: None,
        status: "active".into(),
        created_at: time::OffsetDateTime::UNIX_EPOCH,
    });

    away::update(
        &away,
        &residents,
        &FixedClock::at(date!(2024 - 01 - 01)),
        &ctx(),
        user_id,
        &actor(),
        period_id,
        away::AwayPeriodInput {
            from_date: date!(2024 - 02 - 03),
            to_date: date!(2024 - 02 - 12),
            note: None,
        },
    )
    .await
    .expect("editing a period must not conflict with itself");

    assert_eq!(
        *away.updates.lock().expect("poisoned"),
        vec![(period_id, date!(2024 - 02 - 03), date!(2024 - 02 - 12))]
    );
}

#[tokio::test]
async fn update__collides_with_a_different_period__conflict() {
    let (user_id, residents) = tenant_with_resident();
    let away = FakeAwayRepo::new();
    away.seed(pg_application::ports::AwayPeriod {
        id: Uuid::new_v4(), // a DIFFERENT period
        resident_id: Uuid::new_v4(),
        from_date: date!(2024 - 02 - 01),
        to_date: date!(2024 - 02 - 10),
        note: None,
        status: "active".into(),
        created_at: time::OffsetDateTime::UNIX_EPOCH,
    });

    let err = away::update(
        &away,
        &residents,
        &FixedClock::at(date!(2024 - 01 - 01)),
        &ctx(),
        user_id,
        &actor(),
        Uuid::new_v4(),
        away::AwayPeriodInput {
            from_date: date!(2024 - 02 - 05),
            to_date: date!(2024 - 02 - 15),
            note: None,
        },
    )
    .await
    .expect_err("overlaps another period");
    assert!(matches!(err, AppError::Conflict(_)));
    assert!(
        away.updates.lock().expect("poisoned").is_empty(),
        "must not write on conflict"
    );
}

#[tokio::test]
async fn cancel__user_without_resident_row__forbidden() {
    let err = away::cancel(
        &FakeAwayRepo::new(),
        &FakeResidentRepo::new(),
        &ctx(),
        UserId::new(Uuid::new_v4()),
        &actor(),
        Uuid::new_v4(),
    )
    .await
    .expect_err("no resident");
    assert!(matches!(err, AppError::Forbidden));
}
