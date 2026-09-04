use crate::persistence::errmap::map_sqlx;
use async_trait::async_trait;
use pg_application::ports::repositories::ReceiptTemplateRepository;
use pg_application::ports::storage::ReceiptTemplate;
use pg_application::AppError;
use pg_domain::OrgId;
use sqlx::PgPool;

pub struct PgReceiptTemplateRepository {
    pool: PgPool,
}

impl PgReceiptTemplateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReceiptTemplateRepository for PgReceiptTemplateRepository {
    async fn resolve(&self, org_id: OrgId) -> Result<ReceiptTemplate, AppError> {
        let row: Option<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT display_name, address, footer, accent, logo_key \
                 FROM public.receipt_templates WHERE org_id = $1",
        )
        .bind(org_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(match row {
            Some((display_name, address, footer, accent, logo_key)) => ReceiptTemplate {
                display_name,
                address,
                footer,
                accent,
                logo_key,
            },
            None => ReceiptTemplate::default_template(),
        })
    }

    async fn upsert(&self, org_id: OrgId, t: &ReceiptTemplate) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO public.receipt_templates (org_id, display_name, address, footer, accent, logo_key) \
             VALUES ($1,$2,$3,$4,$5,$6) \
             ON CONFLICT (org_id) DO UPDATE SET display_name = EXCLUDED.display_name, \
             address = EXCLUDED.address, footer = EXCLUDED.footer, accent = EXCLUDED.accent, \
             logo_key = EXCLUDED.logo_key, updated_at = now()",
        )
        .bind(org_id.as_uuid())
        .bind(&t.display_name)
        .bind(&t.address)
        .bind(&t.footer)
        .bind(&t.accent)
        .bind(&t.logo_key)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}
