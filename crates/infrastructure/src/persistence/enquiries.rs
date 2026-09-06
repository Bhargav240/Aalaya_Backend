use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::{EnquiryRecord, EnquiryRepository};
use pg_application::AppError;
use pg_domain::{BranchId, OrgId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgEnquiryRepository {
    pool: PgPool,
}

impl PgEnquiryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type Row = (
    Uuid,
    Uuid,
    String,
    Option<String>,
    String,
    i64,
    Option<String>,
    Option<String>,
);
fn row(r: Row) -> EnquiryRecord {
    EnquiryRecord {
        id: r.0,
        branch_id: r.1,
        phone: r.2,
        note: r.3,
        status: r.4,
        created_epoch: r.5,
        prospect_name: r.6,
        prospect_email: r.7,
    }
}

#[async_trait]
impl EnquiryRepository for PgEnquiryRepository {
    async fn create(
        &self,
        id: Uuid,
        org_id: OrgId,
        branch_id: BranchId,
        prospect_id: UserId,
        phone: &str,
        note: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.enquiries (id, org_id, branch_id, prospect_id, phone, note) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(org_id.as_uuid())
        .bind(branch_id.as_uuid())
        .bind(prospect_id.as_uuid())
        .bind(phone)
        .bind(note)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn list_for_branch(&self, branch_id: BranchId) -> Result<Vec<EnquiryRecord>, AppError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT e.id, e.branch_id, \
                    COALESCE(NULLIF(e.phone, 'email-authenticated'), u.phone, u.email, 'No contact') as phone, \
                    e.note, e.status, EXTRACT(EPOCH FROM e.created_at)::bigint, \
                    u.full_name, u.email \
             FROM public.enquiries e \
             LEFT JOIN public.platform_users u ON u.id = e.prospect_id \
             WHERE e.branch_id=$1 AND e.status != 'closed' \
             ORDER BY e.created_at DESC LIMIT 500",
        )
        .bind(branch_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(row).collect())
    }

    async fn list_for_org(&self, org_id: OrgId) -> Result<Vec<EnquiryRecord>, AppError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT e.id, e.branch_id, \
                    COALESCE(NULLIF(e.phone, 'email-authenticated'), u.phone, u.email, 'No contact') as phone, \
                    e.note, e.status, EXTRACT(EPOCH FROM e.created_at)::bigint, \
                    u.full_name, u.email \
             FROM public.enquiries e \
             LEFT JOIN public.platform_users u ON u.id = e.prospect_id \
             WHERE e.org_id=$1 AND e.status != 'closed' \
             ORDER BY e.created_at DESC LIMIT 500",
        )
        .bind(org_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(row).collect())
    }

    async fn set_status(&self, id: Uuid, status: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE public.enquiries SET status=$2 WHERE id=$1")
            .bind(id)
            .bind(status)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }

    async fn org_of_branch(&self, branch_id: BranchId) -> Result<Option<OrgId>, AppError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT org_id FROM public.public_listings WHERE branch_id = $1 AND status='live'",
        )
        .bind(branch_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(|(o,)| OrgId::new(o)))
    }
}
