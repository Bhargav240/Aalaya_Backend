//! Renders a resident's deposit and rent receipts to PDF files, with no
//! database and no object store.
//!
//!   cargo run -p pg-infrastructure --example render_tenant_receipts -- <out-dir>
//!
//! This is a PREVIEW renderer, the same one the server uses, driven by
//! hand-supplied data. It writes no payment row, no receipt row and nothing to
//! the object store, so the figures below are not a record that money changed
//! hands — they are demo documents. A receipt that is a real record can only be
//! produced by `record_payment`, which requires an actual payment.
//!
//! The template values mirror `public.receipt_templates` for the Hamsa Vilas PG
//! org so the output carries the same branding a real receipt would.

use pg_application::ports::storage::{ReceiptData, ReceiptRenderer, ReceiptTemplate};
use pg_infrastructure::receipt::SimpleReceiptRenderer;

fn main() -> anyhow::Result<()> {
    let out_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./receipts".to_string());
    std::fs::create_dir_all(&out_dir)?;

    let template = ReceiptTemplate {
        display_name: "Hamsa Vilas PG".to_string(),
        address: Some(
            "#42, 27th Main Road, HSR Layout Sector 2, Bengaluru, Karnataka 560102".to_string(),
        ),
        footer: Some(
            "Rent is payable on or before the 5th of each month; a late fee of INR 100 per day applies thereafter.\n\
             Payments are adjusted against the earliest outstanding dues first.\n\
             Report any discrepancy within 7 days at +91 90000 10101; thereafter this invoice is deemed accepted."
                .to_string(),
        ),
        accent: Some("#2E7D32".to_string()),
        logo_key: None,
    };

    // 2026-08-31 12:00 IST. Ashrith's tenancy starts on the 31st, so that is
    // the only date in August a document for him can honestly carry.
    let issued_epoch = 1_788_157_800;

    // A single-branch org prints no branch line: `record_payment` sets
    // `branch_name` only when `branch_count > 1`, and Hamsa Vilas PG has one
    // branch. Phone is absent because the resident record has none. Method is
    // "Cash" and the actor string is "<name> (<role>)", matching how
    // `record_payment` builds a real receipt.
    let document = |number: &str, line_item: &str, amount_paise: i64| ReceiptData {
        receipt_number: number.to_string(),
        invoice_number: number.to_string(),
        resident_name: "Ashrith Bade".to_string(),
        branch_name: None,
        amount_paise,
        issued_epoch,
        tenant_phone: None,
        room_number: Some("101".to_string()),
        plan_kind: "monthly".to_string(),
        line_item: line_item.to_string(),
        payment_method: "Cash".to_string(),
        payment_reference: None,
        floor_label: None,
        recorded_by: "Bhargavraj (Owner)".to_string(),
    };

    let documents = [
        ("deposit", "RC-8F42C1A7", "Security deposit", 2_500_000i64),
        ("rent-august", "RC-2D9B54E0", "Rent", 1_250_000i64),
    ];

    for (slug, number, line_item, amount_paise) in documents {
        let data = document(number, line_item, amount_paise);
        let path = format!("{out_dir}/ashrith-bade-{slug}.pdf");
        std::fs::write(&path, SimpleReceiptRenderer.render(&template, &data))?;
        println!("wrote {path}  ({line_item}, {amount_paise} paise)");
    }

    Ok(())
}
