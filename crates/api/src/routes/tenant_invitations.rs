use crate::{error::ApiError, extractors::AuthUser, state::AppState, support::resolve_ctx};
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use pg_application::ports::repositories::TenantInvitation;
use pg_application::services::{
    accept_tenant_invitation, approve_tenant_invitation, complete_digilocker,
    create_tenant_invitation, decline_tenant_invitation, get_invitation_verification,
    get_tenant_invitation, list_my_tenant_invitations, list_tenant_invitations, manual_kyc_review,
    resend_tenant_invitation, review_tenant_invitation, start_digilocker, tenant_onboarding_status,
    AcceptTenantInvitationCommand, ApproveTenantInvitationCommand, CompleteDigiLockerCommand,
    CreateTenantInvitationCommand, DeclineTenantInvitationCommand, GetTenantInvitationCommand,
    ListMyTenantInvitationsCommand, ListTenantInvitationsCommand, ManualKycReviewCommand,
    ResendTenantInvitationCommand, ReviewTenantInvitationCommand, StartDigiLockerCommand,
};
use pg_domain::{BedId, BranchId, InvitationStatus, RoomId};
use serde::{Deserialize, Serialize};
use time::{Date, Month};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tenant-invitations", get(list).post(create))
        .route("/tenant-invitations/{id}", get(get_one))
        .route("/tenant-invitations/{id}/accept", post(accept))
        .route("/tenant-invitations/{id}/decline", post(decline))
        .route("/tenant-invitations/{id}/approve", post(approve))
        .route("/tenant-invitations/{id}/reject", post(reject))
        .route("/tenant-invitations/{id}/cancel", post(cancel))
        .route("/tenant-invitations/{id}/resend", post(resend))
        .route(
            "/tenant-invitations/{id}/digilocker/start",
            post(digilocker_start),
        )
        .route("/tenant-invitations/{id}/verification", get(verification))
        .route(
            "/tenant-invitations/{id}/manual-review",
            post(manual_review),
        )
        .route(
            "/integrations/digilocker/callback",
            get(digilocker_callback),
        )
        .route(
            "/tenant-invitations/{id}/onboarding-status",
            get(onboarding_status),
        )
        .route("/me/tenant-invitations", get(list_my))
}

#[derive(Deserialize)]
struct CreateRequest {
    email: String,
    full_name: String,
    plan_kind: String,
    branch_id: Uuid,
    intended_room_id: Uuid,
    intended_bed_id: Option<Uuid>,
    monthly_rent_paise: Option<i64>,
    deposit_paise: Option<i64>,
    planned_move_in_date: Option<String>,
    require_identity_verification: Option<bool>,
    expires_in_days: Option<i64>,
}

#[derive(Serialize)]
struct InvitationResponse {
    id: Uuid,
    org_id: Uuid,
    branch_id: Uuid,
    intended_room_id: Option<Uuid>,
    intended_bed_id: Option<Uuid>,
    email: String,
    full_name: String,
    plan_kind: String,
    monthly_rent_paise: Option<i64>,
    deposit_paise: Option<i64>,
    planned_move_in_date: Option<String>,
    require_identity_verification: bool,
    status: InvitationStatus,
    expires_at: String,
    last_sent_at: String,
    accepted_by_user_id: Option<Uuid>,
    accepted_at: Option<String>,
    approved_at: Option<String>,
    approved_by_user_id: Option<Uuid>,
}

impl From<TenantInvitation> for InvitationResponse {
    fn from(invitation: TenantInvitation) -> Self {
        Self {
            id: invitation.id,
            org_id: invitation.org_id.as_uuid(),
            branch_id: invitation.branch_id.as_uuid(),
            intended_room_id: invitation.intended_room_id.map(|id| id.as_uuid()),
            intended_bed_id: invitation.intended_bed_id.map(|id| id.as_uuid()),
            email: invitation.email,
            full_name: invitation.full_name,
            plan_kind: invitation.plan_kind,
            monthly_rent_paise: invitation.monthly_rent_paise,
            deposit_paise: invitation.deposit_paise,
            planned_move_in_date: invitation.planned_move_in_date.map(|date| date.to_string()),
            require_identity_verification: invitation.require_identity_verification,
            status: invitation.status,
            expires_at: invitation.expires_at.to_string(),
            last_sent_at: invitation.last_sent_at.to_string(),
            accepted_by_user_id: invitation.accepted_by_user_id.map(|id| id.as_uuid()),
            accepted_at: invitation.accepted_at.map(|value| value.to_string()),
            approved_at: invitation.approved_at.map(|value| value.to_string()),
            approved_by_user_id: invitation.approved_by_user_id.map(|id| id.as_uuid()),
        }
    }
}

#[derive(Serialize)]
struct StatusResponse {
    status: InvitationStatus,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<InvitationResponse>>, ApiError> {
    let deps = state.deps()?;
    let invitations = list_tenant_invitations(ListTenantInvitationsCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        actor_id: claims.user_id,
        actor_org_id: claims.org_id,
        actor_role: claims.role,
    })
    .await?;
    Ok(Json(
        invitations
            .into_iter()
            .map(InvitationResponse::from)
            .collect(),
    ))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(request): Json<CreateRequest>,
) -> Result<Json<InvitationResponse>, ApiError> {
    let deps = state.deps()?;
    let planned_move_in_date = request
        .planned_move_in_date
        .as_deref()
        .map(parse_iso_date)
        .transpose()?;
    let invitation = create_tenant_invitation(CreateTenantInvitationCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        email_sender: deps.invitation_email_sender.as_ref(),
        actor_id: claims.user_id,
        actor_org_id: claims.org_id,
        actor_role: claims.role,
        email: request.email,
        full_name: request.full_name,
        plan_kind: request.plan_kind,
        branch_id: BranchId::new(request.branch_id),
        intended_room_id: Some(RoomId::new(request.intended_room_id)),
        intended_bed_id: request.intended_bed_id.map(BedId::new),
        monthly_rent_paise: request.monthly_rent_paise,
        deposit_paise: request.deposit_paise,
        planned_move_in_date,
        require_identity_verification: request.require_identity_verification.unwrap_or_else(|| {
            deps.onboarding_layout
                .as_ref()
                .is_some_and(|layout| layout.identity_verification.required_for_tenant_activation)
        }),
        expires_in_days: request.expires_in_days,
    })
    .await?;
    Ok(Json(invitation.into()))
}

fn parse_iso_date(value: &str) -> Result<Date, pg_application::AppError> {
    let mut parts = value.split('-');
    let year = parts.next().and_then(|part| part.parse::<i32>().ok());
    let month = parts
        .next()
        .and_then(|part| part.parse::<u8>().ok())
        .and_then(|month| Month::try_from(month).ok());
    let day = parts.next().and_then(|part| part.parse::<u8>().ok());
    if parts.next().is_some() {
        return Err(pg_application::AppError::Validation(
            "planned_move_in_date must use YYYY-MM-DD".into(),
        ));
    }
    match (year, month, day) {
        (Some(year), Some(month), Some(day)) => {
            Date::from_calendar_date(year, month, day).map_err(|_| {
                pg_application::AppError::Validation(
                    "planned_move_in_date must be a valid YYYY-MM-DD date".into(),
                )
            })
        }
        _ => Err(pg_application::AppError::Validation(
            "planned_move_in_date must use YYYY-MM-DD".into(),
        )),
    }
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<InvitationResponse>, ApiError> {
    let deps = state.deps()?;
    let invitation = get_tenant_invitation(GetTenantInvitationCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        actor_id: claims.user_id,
        actor_org_id: claims.org_id,
        actor_role: claims.role,
        invitation_id,
    })
    .await?;
    Ok(Json(invitation.into()))
}

async fn accept(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<StatusResponse>, ApiError> {
    let deps = state.deps()?;
    let status = accept_tenant_invitation(AcceptTenantInvitationCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        email_sender: deps.invitation_email_sender.as_ref(),
        actor_id: claims.user_id,
        actor_role: claims.role,
        invitation_id,
    })
    .await?;
    Ok(Json(StatusResponse { status }))
}

async fn list_my(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<InvitationResponse>>, ApiError> {
    let deps = state.deps()?;
    let invitations = list_my_tenant_invitations(ListMyTenantInvitationsCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        actor_id: claims.user_id,
        actor_role: claims.role,
    })
    .await?;
    Ok(Json(
        invitations
            .into_iter()
            .map(InvitationResponse::from)
            .collect(),
    ))
}

async fn resend(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<InvitationResponse>, ApiError> {
    let deps = state.deps()?;
    let invitation = resend_tenant_invitation(ResendTenantInvitationCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        email_sender: deps.invitation_email_sender.as_ref(),
        actor_id: claims.user_id,
        actor_org_id: claims.org_id,
        actor_role: claims.role,
        invitation_id,
        cooldown_secs: deps.invitation_resend_cooldown_secs,
    })
    .await?;
    Ok(Json(invitation.into()))
}

#[derive(Serialize)]
struct OnboardingStatusResponse {
    invitation_id: Uuid,
    status: InvitationStatus,
    require_identity_verification: bool,
    next_action: &'static str,
}

async fn onboarding_status(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<OnboardingStatusResponse>, ApiError> {
    let deps = state.deps()?;
    let status = tenant_onboarding_status(GetTenantInvitationCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        actor_id: claims.user_id,
        actor_org_id: claims.org_id,
        actor_role: claims.role,
        invitation_id,
    })
    .await?;
    Ok(Json(OnboardingStatusResponse {
        invitation_id: status.invitation_id,
        status: status.status,
        require_identity_verification: status.require_identity_verification,
        next_action: status.next_action,
    }))
}

async fn decline(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<StatusResponse>, ApiError> {
    let deps = state.deps()?;
    let status = decline_tenant_invitation(DeclineTenantInvitationCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        actor_id: claims.user_id,
        actor_role: claims.role,
        invitation_id,
    })
    .await?;
    Ok(Json(StatusResponse { status }))
}

async fn review(
    state: AppState,
    claims: pg_application::ports::auth::Claims,
    invitation_id: Uuid,
    target: InvitationStatus,
) -> Result<Json<StatusResponse>, ApiError> {
    let deps = state.deps()?;
    let status = review_tenant_invitation(ReviewTenantInvitationCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        email_sender: deps.invitation_email_sender.as_ref(),
        actor_id: claims.user_id,
        actor_org_id: claims.org_id,
        actor_role: claims.role,
        invitation_id,
        target,
    })
    .await?;
    Ok(Json(StatusResponse { status }))
}

async fn approve(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<StatusResponse>, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    approve_tenant_invitation(ApproveTenantInvitationCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        activation: deps.tenant_activation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        email_sender: deps.invitation_email_sender.as_ref(),
        ctx: &ctx,
        actor_id: claims.user_id,
        actor_org_id: claims.org_id,
        actor_role: claims.role,
        invitation_id,
        request_id: pg_application::current_request_id(),
    })
    .await?;
    Ok(Json(StatusResponse {
        status: InvitationStatus::Active,
    }))
}

async fn reject(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<StatusResponse>, ApiError> {
    review(state, claims, invitation_id, InvitationStatus::Rejected).await
}

async fn cancel(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<StatusResponse>, ApiError> {
    review(state, claims, invitation_id, InvitationStatus::Cancelled).await
}

#[derive(Serialize)]
struct DigiLockerStartResponse {
    authorization_url: String,
}

async fn digilocker_start(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<DigiLockerStartResponse>, ApiError> {
    let deps = state.deps()?;
    let client = deps
        .digilocker_client
        .as_deref()
        .ok_or_else(|| pg_application::AppError::Internal("DigiLocker is not configured".into()))?;
    let encryption_key = deps
        .digilocker_session_encryption_key
        .as_deref()
        .ok_or_else(|| pg_application::AppError::Internal("DigiLocker is not configured".into()))?;
    let authorization_url = start_digilocker(StartDigiLockerCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        verifications: deps.invitation_verification_repo.as_ref(),
        client,
        encryption_key,
        session_ttl_secs: deps.digilocker_session_ttl_secs,
        actor_id: claims.user_id,
        actor_role: claims.role,
        invitation_id,
    })
    .await?;
    Ok(Json(DigiLockerStartResponse { authorization_url }))
}

#[derive(Deserialize)]
struct DigiLockerCallbackQuery {
    state: String,
    code: Option<String>,
    error: Option<String>,
}

async fn digilocker_callback(
    State(state): State<AppState>,
    Query(query): Query<DigiLockerCallbackQuery>,
) -> Result<Response, ApiError> {
    let deps = state.deps()?;
    let client = deps
        .digilocker_client
        .as_deref()
        .ok_or_else(|| pg_application::AppError::Internal("DigiLocker is not configured".into()))?;
    let encryption_key = deps
        .digilocker_session_encryption_key
        .as_deref()
        .ok_or_else(|| pg_application::AppError::Internal("DigiLocker is not configured".into()))?;
    let result = complete_digilocker(CompleteDigiLockerCommand {
        verifications: deps.invitation_verification_repo.as_ref(),
        client,
        encryption_key,
        state: &query.state,
        code: query.code.as_deref(),
        provider_error: query.error.as_deref(),
    })
    .await?;
    let location = format!(
        "{}?invitation_id={}&status={}",
        deps.app_deep_link_base,
        result.invitation_id,
        result.status.as_str()
    );
    Ok(Redirect::temporary(&location).into_response())
}

#[derive(Serialize)]
struct VerificationResponse {
    provider: String,
    provider_reference: Option<String>,
    document_type: Option<String>,
    status: String,
    verified_at: Option<String>,
    safe_failure_reason: Option<String>,
}

async fn verification(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
) -> Result<Json<Option<VerificationResponse>>, ApiError> {
    let deps = state.deps()?;
    let verification = get_invitation_verification(
        deps.tenant_invitation_repo.as_ref(),
        deps.user_repo.as_ref(),
        deps.invitation_verification_repo.as_ref(),
        claims.user_id,
        claims.org_id,
        claims.role,
        invitation_id,
    )
    .await?;
    Ok(Json(verification.map(|record| VerificationResponse {
        provider: record.provider,
        provider_reference: record.provider_reference,
        document_type: record.document_type,
        status: record.status,
        verified_at: record.verified_at.map(|value| value.to_string()),
        safe_failure_reason: record.safe_failure_reason,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ManualReviewDecision {
    Approved,
    Rejected,
}

#[derive(Deserialize)]
struct ManualReviewRequest {
    decision: ManualReviewDecision,
    reason: String,
}

async fn manual_review(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(invitation_id): Path<Uuid>,
    Json(request): Json<ManualReviewRequest>,
) -> Result<Json<StatusResponse>, ApiError> {
    let deps = state.deps()?;
    let approved = matches!(request.decision, ManualReviewDecision::Approved);
    let manual_review_allowed = deps
        .onboarding_layout
        .as_ref()
        .is_none_or(|layout| layout.identity_verification.allow_manual_review);
    manual_kyc_review(ManualKycReviewCommand {
        invitations: deps.tenant_invitation_repo.as_ref(),
        users: deps.user_repo.as_ref(),
        verifications: deps.invitation_verification_repo.as_ref(),
        actor_id: claims.user_id,
        actor_org_id: claims.org_id,
        actor_role: claims.role,
        invitation_id,
        approved,
        reason: request.reason,
        request_id: pg_application::current_request_id(),
        manual_review_allowed,
    })
    .await?;
    Ok(Json(StatusResponse {
        status: if approved {
            InvitationStatus::ApprovalPending
        } else {
            InvitationStatus::KycFailed
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invitation_move_in_date_uses_android_iso_contract() {
        assert_eq!(
            parse_iso_date("2026-09-01")
                .expect("valid date")
                .to_string(),
            "2026-09-01",
        );
        assert!(parse_iso_date("2026-02-30").is_err());
        assert!(parse_iso_date("01/09/2026").is_err());
    }
}
