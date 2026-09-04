//! Renders the PDF for every `ready` receipt that has no stored object yet.
//!
//! `supabase/seeds/sample_owner_manager_tenant.sql` inserts payments and
//! receipts directly, which skips the render-and-store step the payment route
//! normally performs. Running this afterwards fills that gap using the SAME
//! renderer and object store the server uses, so the seeded receipts download
//! exactly like real ones.
//!
//!   cargo run -p pg-infrastructure --example seed_sample_receipts
//!
//! Reads DATABASE_URL and STORAGE_DIR from the environment (or .env).

use pg_application::ports::storage::{ObjectStore, ReceiptData, ReceiptRenderer, ReceiptTemplate};
use pg_infrastructure::receipt::SimpleReceiptRenderer;
use pg_infrastructure::storage::LocalFsObjectStore;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

/// Only this org's receipts are touched, so an accidental run cannot rewrite
/// PDFs belonging to a different organization.
const SAMPLE_ORG: &str = "cb68eae7-2955-5cae-e440-0a0190731fe1";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let database_url = std::env::var("DATABASE_URL")?;
    let storage_dir = std::env::var("STORAGE_DIR").unwrap_or_else(|_| "./data/objectstore".into());

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;
    let store = LocalFsObjectStore::new(&storage_dir);
    let renderer = SimpleReceiptRenderer;

    let org_id: Uuid = SAMPLE_ORG.parse()?;

    let template_row = sqlx::query(
        "SELECT display_name, address, footer, accent, logo_key \
         FROM public.receipt_templates WHERE org_id = $1",
    )
    .bind(org_id)
    .fetch_optional(&pool)
    .await?;
    let template = match template_row {
        Some(r) => ReceiptTemplate {
            display_name: r.get("display_name"),
            address: r.get("address"),
            footer: r.get("footer"),
            accent: r.get("accent"),
            logo_key: r.get("logo_key"),
        },
        None => ReceiptTemplate::default_template(),
    };

    // The pooled tenant schema is behind RLS keyed on app.current_org, so the
    // read runs inside a transaction that sets it, exactly like the server.
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT set_config('app.current_org', $1, true)")
        .bind(org_id.to_string())
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL search_path TO pool, public, extensions")
        .execute(&mut *tx)
        .await?;

    // A single-branch org gets a branch-free receipt, matching services::money.
    let branch_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM branches WHERE org_id = $1 AND deleted_at IS NULL",
    )
    .bind(org_id)
    .fetch_one(&mut *tx)
    .await?;

    let rows = sqlx::query(
        "SELECT rc.storage_key, rc.receipt_number, rc.invoice_number, rc.amount_paise, \
                EXTRACT(EPOCH FROM rc.issued_at)::bigint AS issued_epoch, \
                res.full_name, res.phone, res.plan_kind, \
                b.name AS branch_name, rm.number AS room_number, \
                pay.method, pay.kind, \
                COALESCE(u.full_name || ' (Owner)', 'Owner') AS recorded_by \
         FROM receipts rc \
         JOIN residents res ON res.id = rc.resident_id AND res.org_id = rc.org_id \
         JOIN branches b ON b.id = res.branch_id AND b.org_id = res.org_id \
         JOIN payments pay ON pay.id = rc.payment_id AND pay.org_id = rc.org_id \
         LEFT JOIN beds bd ON bd.id = res.bed_id AND bd.org_id = res.org_id \
         LEFT JOIN rooms rm ON rm.id = bd.room_id AND rm.org_id = res.org_id \
         LEFT JOIN public.platform_users u ON u.id = pay.recorded_by \
         WHERE rc.org_id = $1 AND rc.deleted_at IS NULL AND rc.storage_key IS NOT NULL \
         ORDER BY rc.issued_at",
    )
    .bind(org_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    if rows.is_empty() {
        println!("no receipts found for org {SAMPLE_ORG}");
        return Ok(());
    }

    let mut written = 0usize;
    for row in &rows {
        let key: String = row.get("storage_key");
        let issued_epoch: i64 = row.get("issued_epoch");
        let branch_name: String = row.get("branch_name");

        let data = ReceiptData {
            receipt_number: row.get("receipt_number"),
            invoice_number: row.get("invoice_number"),
            resident_name: row.get("full_name"),
            branch_name: (branch_count > 1).then_some(branch_name),
            amount_paise: row.get("amount_paise"),
            issued_epoch,
            // These MUST mirror services::money::record_payment exactly, or
            // seeded receipts look unlike anything the running app produces.
            tenant_phone: row.get("phone"),
            room_number: row.get("room_number"),
            plan_kind: row.get("plan_kind"),
            line_item: match row.get::<String, _>("kind").as_str() {
                "deposit" => "Security deposit".to_string(),
                _ => "Rent".to_string(),
            },
            payment_method: "Cash".to_string(),
            payment_reference: None,
            floor_label: None,
            recorded_by: row.get("recorded_by"),
        };

        let pdf = renderer.render(&template, &data);
        let bytes = pdf.len();
        store.put(&key, pdf, "application/pdf").await?;
        written += 1;

        let issued = OffsetDateTime::from_unix_timestamp(issued_epoch)
            .map(|d| d.date().to_string())
            .unwrap_or_else(|_| "?".into());
        println!(
            "  {}  {}  {:>10}  {} bytes  -> {}",
            data.invoice_number,
            issued,
            format_rupees(data.amount_paise),
            bytes,
            key
        );
    }

    println!("\nrendered {written} receipt PDF(s) under {storage_dir}");
    Ok(())
}

fn format_rupees(paise: i64) -> String {
    format!("Rs {}.{:02}", paise / 100, paise % 100)
}
