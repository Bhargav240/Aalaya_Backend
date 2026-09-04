use crate::persistence::errmap::map_sqlx;
use crate::persistence::scope::scoped_tx;
use async_trait::async_trait;
use pg_application::ports::repositories::{
    ActivateTenantRecord, TenantActivationRepository, TenantActivationResult,
};
use pg_application::ports::tenancy::TenantContext;
use pg_application::AppError;
use pg_domain::{InvitationStatus, ResidentId};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct ActivationInvitationRow {
    org_id: Uuid,
    branch_id: Uuid,
    intended_room_id: Option<Uuid>,
    intended_bed_id: Option<Uuid>,
    email: String,
    full_name: String,
    plan_kind: String,
    monthly_rent_paise: Option<i64>,
    deposit_paise: Option<i64>,
    require_identity_verification: bool,
    status: String,
    accepted_by_user_id: Option<Uuid>,
}

pub struct PgTenantActivationRepository {
    pool: PgPool,
}

impl PgTenantActivationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantActivationRepository for PgTenantActivationRepository {
    async fn activate(
        &self,
        ctx: &TenantContext,
        record: &ActivateTenantRecord,
    ) -> Result<TenantActivationResult, AppError> {
        let org_id = ctx.org_id.as_uuid();
        let mut tx = scoped_tx(&self.pool, ctx).await?;
        let invitation: ActivationInvitationRow = sqlx::query_as(
            "SELECT org_id, branch_id, intended_room_id, intended_bed_id, email, full_name, \
                    plan_kind, monthly_rent_paise, deposit_paise, require_identity_verification, status, \
                    accepted_by_user_id \
             FROM public.tenant_invitations WHERE id = $1 FOR UPDATE",
        )
        .bind(record.invitation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .ok_or(AppError::NotFound)?;

        if invitation.org_id != org_id {
            return Err(AppError::Forbidden);
        }
        if InvitationStatus::parse(&invitation.status) != Some(InvitationStatus::ApprovalPending) {
            return Err(AppError::Conflict(
                "invitation is not ready for approval".into(),
            ));
        }
        if invitation.require_identity_verification {
            // A KYC-required invitation reaches approval_pending only after a
            // verified provider/manual decision. This guard catches corrupt
            // state rather than silently activating it.
            let verified: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM public.invitation_identity_verifications \
                 WHERE invitation_id = $1 AND status = 'verified')",
            )
            .bind(record.invitation_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap_or(false);
            if !verified {
                return Err(AppError::Conflict(
                    "identity verification is required before approval".into(),
                ));
            }
        }

        let accepted_user_id = invitation
            .accepted_by_user_id
            .ok_or_else(|| AppError::Conflict("invitation has not been accepted".into()))?;
        let verified_email: Option<String> = sqlx::query_scalar(
            "SELECT email FROM public.platform_users \
             WHERE id = $1 AND deleted_at IS NULL AND email_verified_at IS NOT NULL",
        )
        .bind(accepted_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .flatten();
        if verified_email
            .as_deref()
            .is_none_or(|email| !email.eq_ignore_ascii_case(&invitation.email))
        {
            return Err(AppError::Forbidden);
        }

        let branch_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM branches \
             WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL)",
        )
        .bind(invitation.branch_id)
        .bind(org_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if !branch_exists {
            return Err(AppError::NotFound);
        }

        if let Some(room_id) = invitation.intended_room_id {
            let room_branch: Option<Uuid> = sqlx::query_scalar(
                "SELECT branch_id FROM rooms \
                 WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
            )
            .bind(room_id)
            .bind(org_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            if room_branch != Some(invitation.branch_id) {
                return Err(AppError::Validation(
                    "selected room does not belong to the invitation branch".into(),
                ));
            }
        }

        if let Some(bed_id) = invitation.intended_bed_id {
            let bed: Option<(Uuid, Uuid)> = sqlx::query_as(
                "SELECT b.room_id, r.branch_id FROM beds b \
                 JOIN rooms r ON r.id = b.room_id AND r.org_id = b.org_id \
                 WHERE b.id = $1 AND b.org_id = $2 AND b.deleted_at IS NULL \
                   AND r.deleted_at IS NULL FOR UPDATE OF b",
            )
            .bind(bed_id)
            .bind(org_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            let (bed_room_id, bed_branch_id) = bed.ok_or(AppError::NotFound)?;
            if bed_branch_id != invitation.branch_id
                || invitation
                    .intended_room_id
                    .is_some_and(|room_id| room_id != bed_room_id)
            {
                return Err(AppError::Validation(
                    "selected bed does not belong to the invitation branch and room".into(),
                ));
            }
            let occupied: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM residents \
                 WHERE org_id = $1 AND bed_id = $2 AND deleted_at IS NULL \
                   AND status <> 'moved_out')",
            )
            .bind(org_id)
            .bind(bed_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            if occupied {
                return Err(AppError::Conflict("residents_one_live_per_bed_uidx".into()));
            }
        }

        let membership_id: Uuid = sqlx::query_scalar(
            "INSERT INTO public.user_org_roles \
             (user_id, org_id, role, branch_id, membership_status, activated_at, ended_at) \
             VALUES ($1,$2,'tenant',$3,'active',now(),NULL) \
             ON CONFLICT (user_id, org_id, role) DO UPDATE \
             SET branch_id = EXCLUDED.branch_id, membership_status = 'active', \
                 activated_at = now(), ended_at = NULL \
             RETURNING id",
        )
        .bind(accepted_user_id)
        .bind(org_id)
        .bind(invitation.branch_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        let existing_resident: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, bed_id FROM residents \
             WHERE org_id = $1 AND deleted_at IS NULL AND status <> 'moved_out' \
               AND (membership_id = $2 OR user_id = $3) \
             ORDER BY CASE WHEN membership_id = $2 THEN 0 ELSE 1 END \
             LIMIT 1 FOR UPDATE",
        )
        .bind(org_id)
        .bind(membership_id)
        .bind(accepted_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        let resident_id = if let Some((resident_id, existing_bed_id)) = existing_resident {
            if invitation.intended_bed_id.is_none() {
                validate_existing_bed_branch(
                    &mut tx,
                    org_id,
                    existing_bed_id,
                    invitation.branch_id,
                )
                .await?;
            }
            sqlx::query(
                "UPDATE residents SET membership_id = $2, user_id = $3, branch_id = $4, \
                 bed_id = COALESCE($5, bed_id), full_name = $6, status = 'active', \
                 plan_kind = $7, \
                 rent_paise = COALESCE($8, rent_paise), \
                 deposit_paise = COALESCE($9, deposit_paise), moved_out_at = NULL \
                 WHERE id = $1 AND org_id = $10 AND deleted_at IS NULL",
            )
            .bind(resident_id)
            .bind(membership_id)
            .bind(accepted_user_id)
            .bind(invitation.branch_id)
            .bind(invitation.intended_bed_id)
            .bind(&invitation.full_name)
            .bind(&invitation.plan_kind)
            .bind(invitation.monthly_rent_paise)
            .bind(invitation.deposit_paise)
            .bind(org_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            resident_id
        } else {
            let resident_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO residents \
                 (id, org_id, branch_id, bed_id, full_name, plan_kind, status, rent_paise, \
                  deposit_paise, user_id, membership_id) \
                 VALUES ($1,$2,$3,$4,$5,$6,'active',$7,$8,$9,$10)",
            )
            .bind(resident_id)
            .bind(org_id)
            .bind(invitation.branch_id)
            .bind(invitation.intended_bed_id)
            .bind(&invitation.full_name)
            .bind(&invitation.plan_kind)
            .bind(invitation.monthly_rent_paise.unwrap_or(0))
            .bind(invitation.deposit_paise.unwrap_or(0))
            .bind(accepted_user_id)
            .bind(membership_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            resident_id
        };

        let updated = sqlx::query(
            "UPDATE public.tenant_invitations \
             SET status = 'active', approved_at = now(), approved_by_user_id = $2, \
                 updated_at = now() WHERE id = $1 AND status = 'approval_pending'",
        )
        .bind(record.invitation_id)
        .bind(record.approved_by_user_id.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if updated.rows_affected() != 1 {
            return Err(AppError::Conflict(
                "invitation changed while it was being approved".into(),
            ));
        }

        sqlx::query(
            "INSERT INTO audit_log \
             (org_id, actor_id, action, entity_type, entity_id, after, request_id) \
             VALUES ($1,$2,'activate_tenant','resident',$3, \
                     jsonb_build_object('invitation_id',$4,'membership_id',$5),$6)",
        )
        .bind(org_id)
        .bind(record.approved_by_user_id.as_uuid())
        .bind(resident_id)
        .bind(record.invitation_id)
        .bind(membership_id)
        .bind(record.request_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        tx.commit().await.map_err(map_sqlx)?;
        Ok(TenantActivationResult {
            membership_id,
            resident_id: ResidentId::new(resident_id),
        })
    }
}

async fn validate_existing_bed_branch(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    existing_bed_id: Option<Uuid>,
    invitation_branch_id: Uuid,
) -> Result<(), AppError> {
    let Some(bed_id) = existing_bed_id else {
        return Ok(());
    };
    let branch_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT r.branch_id FROM beds b JOIN rooms r \
         ON r.id = b.room_id AND r.org_id = b.org_id \
         WHERE b.id = $1 AND b.org_id = $2 AND b.deleted_at IS NULL \
           AND r.deleted_at IS NULL",
    )
    .bind(bed_id)
    .bind(org_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx)?;
    if branch_id == Some(invitation_branch_id) {
        Ok(())
    } else {
        Err(AppError::Conflict(
            "existing resident belongs to a different branch".into(),
        ))
    }
}
