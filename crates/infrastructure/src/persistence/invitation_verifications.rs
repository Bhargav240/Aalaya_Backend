use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::{
    DigiLockerSession, InvitationVerification, InvitationVerificationRepository,
    NewDigiLockerSession,
};
use pg_application::AppError;
use pg_domain::UserId;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PgInvitationVerificationRepository {
    pool: PgPool,
}

impl PgInvitationVerificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvitationVerificationRepository for PgInvitationVerificationRepository {
    async fn create_digilocker_session(
        &self,
        session: NewDigiLockerSession<'_>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.digilocker_oauth_sessions \
             (id, invitation_id, user_id, state_hash, nonce_hash, \
              pkce_verifier_ciphertext, expires_at) \
             VALUES ($1,$2,$3,$4,$5,pgp_sym_encrypt($6,$7),$8)",
        )
        .bind(Uuid::new_v4())
        .bind(session.invitation_id)
        .bind(session.user_id.as_uuid())
        .bind(session.state_hash)
        .bind(session.nonce_hash)
        .bind(session.pkce_verifier)
        .bind(session.encryption_key)
        .bind(session.expires_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn consume_digilocker_session(
        &self,
        state_hash: &str,
        encryption_key: &str,
    ) -> Result<Option<DigiLockerSession>, AppError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
        let row: Option<(Uuid, Uuid, String, String)> = sqlx::query_as(
            "SELECT invitation_id, user_id, nonce_hash, \
                    pgp_sym_decrypt(pkce_verifier_ciphertext,$2) \
             FROM public.digilocker_oauth_sessions \
             WHERE state_hash = $1 AND consumed_at IS NULL AND expires_at > now() \
             FOR UPDATE",
        )
        .bind(state_hash)
        .bind(encryption_key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if row.is_some() {
            sqlx::query(
                "UPDATE public.digilocker_oauth_sessions SET consumed_at = now() \
                 WHERE state_hash = $1 AND consumed_at IS NULL",
            )
            .bind(state_hash)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(
            |(invitation_id, user_id, nonce_hash, pkce_verifier)| DigiLockerSession {
                invitation_id,
                user_id: UserId::new(user_id),
                nonce_hash,
                pkce_verifier,
            },
        ))
    }

    async fn complete_digilocker(
        &self,
        invitation_id: Uuid,
        provider_reference: Option<&str>,
        document_type: Option<&str>,
        verified: bool,
        safe_failure_reason: Option<&str>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
        let current: Option<String> = sqlx::query_scalar(
            "SELECT status FROM public.tenant_invitations WHERE id = $1 FOR UPDATE",
        )
        .bind(invitation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if current.as_deref() != Some("kyc_pending") {
            return Err(AppError::Conflict(
                "invitation is not awaiting identity verification".into(),
            ));
        }
        let status = if verified { "verified" } else { "failed" };
        sqlx::query(
            "INSERT INTO public.invitation_identity_verifications \
             (id, invitation_id, provider, provider_reference, document_type, status, \
              verified_at, safe_failure_reason) \
             VALUES ($1,$2,'digilocker',$3,$4,$5,CASE WHEN $5='verified' THEN now() END,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(invitation_id)
        .bind(provider_reference)
        .bind(document_type)
        .bind(status)
        .bind(safe_failure_reason)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "UPDATE public.tenant_invitations SET status = $2, updated_at = now() WHERE id = $1",
        )
        .bind(invitation_id)
        .bind(if verified {
            "approval_pending"
        } else {
            "kyc_failed"
        })
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn latest(
        &self,
        invitation_id: Uuid,
    ) -> Result<Option<InvitationVerification>, AppError> {
        let row: Option<(
            String,
            Option<String>,
            Option<String>,
            String,
            Option<OffsetDateTime>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT provider, provider_reference, document_type, status, verified_at, \
                    safe_failure_reason \
             FROM public.invitation_identity_verifications \
             WHERE invitation_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(invitation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(
            |(
                provider,
                provider_reference,
                document_type,
                status,
                verified_at,
                safe_failure_reason,
            )| InvitationVerification {
                provider,
                provider_reference,
                document_type,
                status,
                verified_at,
                safe_failure_reason,
            },
        ))
    }

    async fn manual_review(
        &self,
        invitation_id: Uuid,
        reviewer_user_id: UserId,
        approved: bool,
        reason: &str,
        request_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
        let current: Option<String> = sqlx::query_scalar(
            "SELECT status FROM public.tenant_invitations WHERE id = $1 FOR UPDATE",
        )
        .bind(invitation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if !matches!(current.as_deref(), Some("kyc_pending" | "kyc_failed")) {
            return Err(AppError::Conflict(
                "invitation is not eligible for manual identity review".into(),
            ));
        }
        let decision = if approved { "approved" } else { "rejected" };
        sqlx::query(
            "INSERT INTO public.manual_kyc_reviews \
             (id, invitation_id, reviewer_user_id, decision, reason, request_id) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(invitation_id)
        .bind(reviewer_user_id.as_uuid())
        .bind(decision)
        .bind(reason)
        .bind(request_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO public.invitation_identity_verifications \
             (id, invitation_id, provider, status, verified_at, safe_failure_reason) \
             VALUES ($1,$2,'manual',$3,CASE WHEN $3='verified' THEN now() END,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(invitation_id)
        .bind(if approved { "verified" } else { "failed" })
        .bind(if approved { None } else { Some(reason) })
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "UPDATE public.tenant_invitations SET status = $2, updated_at = now() WHERE id = $1",
        )
        .bind(invitation_id)
        .bind(if approved {
            "approval_pending"
        } else {
            "kyc_failed"
        })
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }
}
