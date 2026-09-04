//! Maker-checker: the routing of edits/deletes through owner-approval.
#![allow(non_snake_case)]
mod fakes;

use fakes::*;
use pg_application::error::AppError;
use pg_application::ports::tenancy::{Placement, TenantContext};
use pg_application::ports::{AuditActor, ChangeTarget};
use pg_application::services::core::{edit_entity, MutationOutcome};
use pg_domain::{OrgId, Role, UserId};
use serde_json::json;
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
fn room_rename_target() -> ChangeTarget {
    ChangeTarget {
        entity_type: "room".into(),
        entity_id: Uuid::new_v4(),
        operation: "update".into(),
        payload: json!({"number": "B-204"}),
    }
}

fn outcome_tag(out: MutationOutcome) -> &'static str {
    match out {
        MutationOutcome::Applied => "applied",
        MutationOutcome::Pending { .. } => "pending",
    }
}

#[tokio::test]
async fn edit_entity__owner__applies_immediately() {
    let cr = FakeChangeRequestRepo::default();
    let out = edit_entity(
        &cr,
        &ctx(),
        Role::Owner,
        &actor(),
        UserId::new(Uuid::new_v4()),
        room_rename_target(),
    )
    .await;
    match out {
        Ok(MutationOutcome::Applied) => {}
        other => panic!(
            "owner edit must apply immediately, got {}",
            outcome_tag(other.expect("err"))
        ),
    }
    assert_eq!(
        cr.applied.lock().expect("poisoned").len(),
        1,
        "apply_now called"
    );
    assert!(
        cr.pending.lock().expect("poisoned").is_empty(),
        "nothing queued"
    );
}

#[tokio::test]
async fn edit_entity__manager__queues_change_request_no_mutation() {
    let cr = FakeChangeRequestRepo::default();
    let out = edit_entity(
        &cr,
        &ctx(),
        Role::Manager,
        &actor(),
        UserId::new(Uuid::new_v4()),
        room_rename_target(),
    )
    .await;
    match out {
        Ok(MutationOutcome::Pending { change_request_id }) => {
            assert_ne!(change_request_id, Uuid::nil());
        }
        Ok(MutationOutcome::Applied) => panic!("manager edit must NEVER apply immediately"),
        Err(e) => panic!("manager edit failed: {e:?}"),
    }
    assert!(
        cr.applied.lock().expect("poisoned").is_empty(),
        "must not mutate"
    );
    assert_eq!(
        cr.pending.lock().expect("poisoned").len(),
        1,
        "queued for owner approval"
    );
}

#[tokio::test]
async fn edit_entity__pending_request__records_entity_and_requester() {
    let cr = FakeChangeRequestRepo::default();
    let requester = UserId::new(Uuid::new_v4());
    let target = ChangeTarget {
        entity_type: "branch".into(),
        entity_id: Uuid::new_v4(),
        operation: "delete".into(),
        payload: serde_json::Value::Null,
    };
    let r = edit_entity(&cr, &ctx(), Role::Manager, &actor(), requester, target).await;
    assert!(r.is_ok());
    let entry = cr.pending.lock().expect("poisoned")[0].clone();
    assert_eq!(entry.0, "branch:delete");
    assert_eq!(entry.1, requester.as_uuid());
}

#[tokio::test]
async fn edit_entity__tenant__forbidden() {
    let cr = FakeChangeRequestRepo::default();
    let out = edit_entity(
        &cr,
        &ctx(),
        Role::Tenant,
        &actor(),
        UserId::new(Uuid::new_v4()),
        room_rename_target(),
    )
    .await;
    assert!(
        matches!(out, Err(AppError::Forbidden)),
        "expected Forbidden, got {out:?}"
    );
}

#[tokio::test]
async fn edit_entity__update_and_delete__both_route_identically() {
    let cr = FakeChangeRequestRepo::default();
    let t_update = ChangeTarget {
        entity_type: "resident".into(),
        entity_id: Uuid::new_v4(),
        operation: "update".into(),
        payload: json!({"full_name": "New Name"}),
    };
    let t_delete = ChangeTarget {
        entity_type: "resident".into(),
        entity_id: Uuid::new_v4(),
        operation: "delete".into(),
        payload: serde_json::Value::Null,
    };
    assert!(edit_entity(
        &cr,
        &ctx(),
        Role::Manager,
        &actor(),
        UserId::new(Uuid::new_v4()),
        t_update
    )
    .await
    .is_ok());
    assert!(edit_entity(
        &cr,
        &ctx(),
        Role::Manager,
        &actor(),
        UserId::new(Uuid::new_v4()),
        t_delete
    )
    .await
    .is_ok());
    assert_eq!(cr.pending.lock().expect("poisoned").len(), 2);
    assert!(cr.applied.lock().expect("poisoned").is_empty());
}
