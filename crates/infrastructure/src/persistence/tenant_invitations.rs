use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::{TenantInvitation, TenantInvitationRepository};
use pg_application::AppError;
use pg_domain::{BedId, BranchId, InvitationStatus, OrgId, RoomId, UserId};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct InvitationRow {
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
    planned_move_in_date: Option<Date>,
    require_identity_verification: bool,
    status: String,
    accepted_by_user_id: Option<Uuid>,
    expires_at: OffsetDateTime,
    last_sent_at: OffsetDateTime,
    accepted_at: Option<OffsetDateTime>,
    approved_at: Option<OffsetDateTime>,
    approved_by_user_id: Option<Uuid>,
}

const INVITATION_COLUMNS: &str = "id, org_id, branch_id, intended_room_id, intended_bed_id, \
    email, full_name, plan_kind, monthly_rent_paise, deposit_paise, planned_move_in_date, \
    require_identity_verification, status, accepted_by_user_id, expires_at, last_sent_at, \
    accepted_at, approved_at, approved_by_user_id";

pub struct PgTenantInvitationRepository {
    pool: PgPool,
}

impl PgTenantInvitationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantInvitationRepository for PgTenantInvitationRepository {
    async fn create(
        &self,
        invitation: &TenantInvitation,
        invited_by: UserId,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.tenant_invitations \
             (id, org_id, branch_id, intended_room_id, intended_bed_id, email, full_name, plan_kind, \
              monthly_rent_paise, deposit_paise, planned_move_in_date, \
              require_identity_verification, status, invited_by_user_id, accepted_by_user_id, \
              expires_at, accepted_at, approved_at, approved_by_user_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)",
        )
        .bind(invitation.id)
        .bind(invitation.org_id.as_uuid())
        .bind(invitation.branch_id.as_uuid())
        .bind(invitation.intended_room_id.map(|id| id.as_uuid()))
        .bind(invitation.intended_bed_id.map(|id| id.as_uuid()))
        .bind(&invitation.email)
        .bind(&invitation.full_name)
        .bind(&invitation.plan_kind)
        .bind(invitation.monthly_rent_paise)
        .bind(invitation.deposit_paise)
        .bind(invitation.planned_move_in_date)
        .bind(invitation.require_identity_verification)
        .bind(invitation.status.as_str())
        .bind(invited_by.as_uuid())
        .bind(invitation.accepted_by_user_id.map(|id| id.as_uuid()))
        .bind(invitation.expires_at)
        .bind(invitation.accepted_at)
        .bind(invitation.approved_at)
        .bind(invitation.approved_by_user_id.map(|id| id.as_uuid()))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<TenantInvitation>, AppError> {
        let query =
            format!("SELECT {INVITATION_COLUMNS} FROM public.tenant_invitations WHERE id = $1");
        let row: Option<InvitationRow> = sqlx::query_as(&query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;
        row.map(decode_invitation).transpose()
    }

    async fn list_for_org(&self, org_id: OrgId) -> Result<Vec<TenantInvitation>, AppError> {
        let query = format!(
            "SELECT {INVITATION_COLUMNS} FROM public.tenant_invitations \
             WHERE org_id = $1 ORDER BY created_at DESC LIMIT 200"
        );
        let rows: Vec<InvitationRow> = sqlx::query_as(&query)
            .bind(org_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;
        rows.into_iter().map(decode_invitation).collect()
    }

    async fn list_for_email(&self, email: &str) -> Result<Vec<TenantInvitation>, AppError> {
        let query = format!(
            "SELECT {INVITATION_COLUMNS} FROM public.tenant_invitations \
             WHERE lower(email) = lower($1) ORDER BY created_at DESC LIMIT 200"
        );
        let rows: Vec<InvitationRow> = sqlx::query_as(&query)
            .bind(email)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;
        rows.into_iter().map(decode_invitation).collect()
    }

    async fn accept(&self, id: Uuid, user_id: UserId, email: &str) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE public.tenant_invitations \
             SET status = 'email_verified', accepted_by_user_id = $2, accepted_at = now(), \
                 updated_at = now() \
             WHERE id = $1 AND lower(email) = lower($3) AND status = 'invited' \
               AND expires_at > now()",
        )
        .bind(id)
        .bind(user_id.as_uuid())
        .bind(email)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected() == 1)
    }

    async fn decline(
        &self,
        id: Uuid,
        email: &str,
        from: InvitationStatus,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE public.tenant_invitations SET status = 'cancelled', updated_at = now() \
             WHERE id = $1 AND lower(email) = lower($2) AND status = $3",
        )
        .bind(id)
        .bind(email)
        .bind(from.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected() == 1)
    }

    async fn transition(
        &self,
        id: Uuid,
        from: InvitationStatus,
        to: InvitationStatus,
        actor: UserId,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE public.tenant_invitations \
             SET status = $3, \
                 approved_by_user_id = CASE WHEN $3 = 'approved' THEN $4 \
                                            ELSE approved_by_user_id END, \
                 approved_at = CASE WHEN $3 = 'approved' THEN now() ELSE approved_at END, \
                 updated_at = now() \
             WHERE id = $1 AND status = $2",
        )
        .bind(id)
        .bind(from.as_str())
        .bind(to.as_str())
        .bind(actor.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected() == 1)
    }

    async fn resend(
        &self,
        id: Uuid,
        expected_status: InvitationStatus,
        expires_at: OffsetDateTime,
        not_sent_after: OffsetDateTime,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE public.tenant_invitations \
             SET expires_at = $3, last_sent_at = now(), updated_at = now() \
             WHERE id = $1 AND status = $2 AND last_sent_at <= $4",
        )
        .bind(id)
        .bind(expected_status.as_str())
        .bind(expires_at)
        .bind(not_sent_after)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected() == 1)
    }
}

fn decode_invitation(row: InvitationRow) -> Result<TenantInvitation, AppError> {
    let InvitationRow {
        id,
        org_id,
        branch_id,
        intended_room_id,
        intended_bed_id,
        email,
        full_name,
        plan_kind,
        monthly_rent_paise,
        deposit_paise,
        planned_move_in_date,
        require_identity_verification,
        status,
        accepted_by_user_id,
        expires_at,
        last_sent_at,
        accepted_at,
        approved_at,
        approved_by_user_id,
    } = row;
    let status = InvitationStatus::parse(&status)
        .ok_or_else(|| AppError::Persistence("tenant invitation has an unknown status".into()))?;

    Ok(TenantInvitation {
        id,
        org_id: OrgId::new(org_id),
        branch_id: BranchId::new(branch_id),
        intended_room_id: intended_room_id.map(RoomId::new),
        intended_bed_id: intended_bed_id.map(BedId::new),
        email,
        full_name,
        plan_kind,
        monthly_rent_paise,
        deposit_paise,
        planned_move_in_date,
        require_identity_verification,
        status,
        accepted_by_user_id: accepted_by_user_id.map(UserId::new),
        expires_at,
        last_sent_at,
        accepted_at,
        approved_at,
        approved_by_user_id: approved_by_user_id.map(UserId::new),
    })
}
