use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::{LoginRecord, MembershipContext, UserRepository};
use pg_application::AppError;
use pg_domain::{BranchId, OrgId, Role, UserId, UserStatus};
use sqlx::PgPool;
use uuid::Uuid;

/// Role precedence when one phone maps to several (platform,owner,manager,tenant,prospect).
const ROLE_ORDER: &str = "CASE r.role \
     WHEN 'platform_operator' THEN 0 \
     WHEN 'owner' THEN 1 \
     WHEN 'manager' THEN 2 \
     WHEN 'tenant' THEN 3 \
     ELSE 4 END";

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<LoginRecord>, AppError> {
        let sql = format!(
            "SELECT u.id, r.org_id, r.role, u.status, u.full_name \
             FROM public.platform_users u JOIN public.user_org_roles r ON r.user_id=u.id \
             WHERE u.deleted_at IS NULL AND r.membership_status='active' AND lower(u.email)=lower($1) \
             ORDER BY {ROLE_ORDER} LIMIT 1"
        );
        let row: Option<(Uuid, Uuid, String, String, Option<String>)> = sqlx::query_as(&sql)
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;
        row.map(|(id, org, role, status, full_name)| {
            Ok(LoginRecord {
                user_id: UserId::new(id),
                org_id: OrgId::new(org),
                role: Role::parse(&role)
                    .ok_or_else(|| AppError::Internal("unknown role".into()))?,
                status: UserStatus::parse(&status)
                    .ok_or_else(|| AppError::Internal("unknown user status".into()))?,
                full_name,
            })
        })
        .transpose()
    }

    async fn create_email_explorer(
        &self,
        id: UserId,
        platform_org_id: OrgId,
        email: &str,
    ) -> Result<LoginRecord, AppError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO public.platform_users (id,org_id,email,status) VALUES ($1,$2,$3,'active')",
        )
        .bind(id.as_uuid())
        .bind(platform_org_id.as_uuid())
        .bind(email)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO public.user_org_roles (user_id,org_id,role) VALUES ($1,$2,'prospect')",
        )
        .bind(id.as_uuid())
        .bind(platform_org_id.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(LoginRecord {
            user_id: id,
            org_id: platform_org_id,
            role: Role::Prospect,
            status: UserStatus::Active,
            full_name: None,
        })
    }

    async fn mark_email_verified(&self, user_id: UserId) -> Result<(), AppError> {
        sqlx::query("UPDATE public.platform_users SET email_verified_at=COALESCE(email_verified_at,now()) WHERE id=$1")
            .bind(user_id.as_uuid()).execute(&self.pool).await.map_err(map_sqlx)?;
        Ok(())
    }
    async fn verified_email_of(&self, user_id: UserId) -> Result<Option<String>, AppError> {
        sqlx::query_scalar("SELECT email FROM public.platform_users WHERE id=$1 AND deleted_at IS NULL AND email_verified_at IS NOT NULL")
            .bind(user_id.as_uuid()).fetch_optional(&self.pool).await.map_err(map_sqlx)
    }
    async fn create_active_owner(
        &self,
        id: UserId,
        org_id: OrgId,
        email: &str,
        phone: Option<&str>,
        full_name: &str,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO public.platform_users (id, org_id, email, phone, full_name, status) \
             VALUES ($1,$2,$3,$4,$5,'active')",
        )
        .bind(id.as_uuid())
        .bind(org_id.as_uuid())
        .bind(email)
        .bind(phone)
        .bind(full_name)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO public.user_org_roles (user_id, org_id, role) VALUES ($1,$2,'owner')",
        )
        .bind(id.as_uuid())
        .bind(org_id.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn auth_context(&self, user_id: UserId) -> Result<Option<(OrgId, Role)>, AppError> {
        let sql = format!(
            "SELECT r.org_id, r.role FROM public.user_org_roles r \
             JOIN public.platform_users u ON u.id = r.user_id \
             WHERE r.user_id = $1 AND u.deleted_at IS NULL AND r.membership_status='active' \
             ORDER BY {ROLE_ORDER} LIMIT 1"
        );
        let row: Option<(Uuid, String)> = sqlx::query_as(&sql)
            .bind(user_id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;
        match row {
            None => Ok(None),
            Some((org, role)) => {
                let role = Role::parse(&role).ok_or_else(|| {
                    AppError::Internal(format!("unknown platform role in user_org_roles: {role}"))
                })?;
                Ok(Some((OrgId::new(org), role)))
            }
        }
    }

    async fn has_active_membership(
        &self,
        user_id: UserId,
        org_id: OrgId,
        role: Role,
    ) -> Result<bool, AppError> {
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM public.user_org_roles r \
             JOIN public.platform_users u ON u.id = r.user_id \
             WHERE r.user_id = $1 AND r.org_id = $2 AND r.role = $3 \
               AND r.membership_status = 'active' AND u.status = 'active' \
               AND u.deleted_at IS NULL)",
        )
        .bind(user_id.as_uuid())
        .bind(org_id.as_uuid())
        .bind(role.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)
    }

    async fn list_active_memberships(
        &self,
        user_id: UserId,
    ) -> Result<Vec<MembershipContext>, AppError> {
        let rows: Vec<(Uuid, Uuid, String, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT r.id, r.org_id, o.name, r.role, r.branch_id \
             FROM public.user_org_roles r \
             JOIN public.organizations o ON o.id = r.org_id \
             JOIN public.platform_users u ON u.id = r.user_id \
             WHERE r.user_id = $1 AND r.membership_status = 'active' \
               AND o.deleted_at IS NULL AND u.deleted_at IS NULL AND u.status = 'active' \
             ORDER BY o.name, r.role",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;

        rows.into_iter()
            .map(
                |(membership_id, org_id, organization_name, role, branch_id)| {
                    Ok(MembershipContext {
                        membership_id,
                        org_id: OrgId::new(org_id),
                        organization_name,
                        role: Role::parse(&role).ok_or_else(|| {
                            AppError::Internal(format!(
                                "unknown platform role in user_org_roles: {role}"
                            ))
                        })?,
                        branch_id: branch_id.map(BranchId::new),
                    })
                },
            )
            .collect()
    }

    async fn find_by_google_sub(
        &self,
        sub: &str,
    ) -> Result<Option<(UserId, OrgId, Role)>, AppError> {
        let sql = format!(
            "SELECT u.id, r.org_id, r.role FROM public.platform_users u \
             JOIN public.user_org_roles r ON r.user_id = u.id \
             WHERE u.google_sub = $1 AND u.deleted_at IS NULL AND r.membership_status='active' \
             ORDER BY {ROLE_ORDER} LIMIT 1"
        );
        let row: Option<(Uuid, Uuid, String)> = sqlx::query_as(&sql)
            .bind(sub)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;
        match row {
            None => Ok(None),
            Some((uid, org, role)) => {
                let role = Role::parse(&role).ok_or_else(|| {
                    AppError::Internal(format!("unknown platform role in user_org_roles: {role}"))
                })?;
                Ok(Some((UserId::new(uid), OrgId::new(org), role)))
            }
        }
    }

    async fn link_google_sub(&self, user_id: UserId, sub: &str) -> Result<(), AppError> {
        let res = sqlx::query(
            "UPDATE public.platform_users SET google_sub = $1 WHERE id = $2 AND deleted_at IS NULL AND (google_sub IS NULL OR google_sub = $1)",
        )
        .bind(sub)
        .bind(user_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            return Err(AppError::Conflict(
                "this account is already linked to a different Google profile".into(),
            ));
        }
        Ok(())
    }

    async fn branch_of(&self, user_id: UserId) -> Result<Option<BranchId>, AppError> {
        let row: Option<(Option<Uuid>,)> = sqlx::query_as(
            "SELECT branch_id FROM public.user_org_roles WHERE user_id = $1 AND branch_id IS NOT NULL LIMIT 1",
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(|(b,)| b).map(BranchId::new))
    }

    async fn branch_in_org(
        &self,
        user_id: UserId,
        org_id: OrgId,
    ) -> Result<Option<BranchId>, AppError> {
        let branch: Option<Uuid> = sqlx::query_scalar(
            "SELECT branch_id FROM public.user_org_roles \
             WHERE user_id=$1 AND org_id=$2 AND membership_status='active' \
               AND branch_id IS NOT NULL ORDER BY role LIMIT 1",
        )
        .bind(user_id.as_uuid())
        .bind(org_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(branch.map(BranchId::new))
    }

    async fn expected_name(&self, user_id: UserId) -> Result<Option<String>, AppError> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT full_name FROM public.platform_users WHERE id = $1")
                .bind(user_id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(map_sqlx)?;
        Ok(row.and_then(|(n,)| n))
    }

    async fn mark_verified(&self, user_id: UserId) -> Result<(), AppError> {
        sqlx::query("UPDATE public.platform_users SET verified = true WHERE id = $1")
            .bind(user_id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn is_verified(&self, user_id: UserId) -> Result<bool, AppError> {
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT verified FROM public.platform_users WHERE id = $1")
                .bind(user_id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(map_sqlx)?;
        Ok(row.map(|(v,)| v).unwrap_or(false))
    }

    async fn update_profile(
        &self,
        user_id: UserId,
        full_name: Option<&str>,
        phone: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE public.platform_users \
             SET full_name = COALESCE($2, full_name), \
                 phone = COALESCE($3, phone) \
             WHERE id = $1",
        )
        .bind(user_id.as_uuid())
        .bind(full_name.filter(|s| !s.trim().is_empty()))
        .bind(phone.filter(|s| !s.trim().is_empty()))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}
