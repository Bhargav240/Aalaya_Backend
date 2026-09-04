//! Renders a sample receipt PDF straight to a file, with no database and no
//! object store, so the invoice layout can be eyeballed while it is being
//! changed.
//!
//!   cargo run -p pg-infrastructure --example render_receipt_preview -- /tmp/preview.pdf

use pg_application::ports::storage::{ReceiptData, ReceiptRenderer, ReceiptTemplate};
use pg_infrastructure::receipt::SimpleReceiptRenderer;

fn main() -> anyhow::Result<()> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./receipt-preview.pdf".to_string());
    // `--stress` renders the awkward end of the range instead of the happy
    // path: a long brand, a two-line address, six terms, and a lakh-scale
    // settlement. The blocks below it must move down, not be written over.
    let stress = std::env::args().any(|a| a == "--stress");

    if stress {
        let template = ReceiptTemplate {
            display_name: "Sri Lakshmi Venkateswara Premium Ladies Paying Guest Accommodation"
                .to_string(),
            address: Some("Plot 14, 3rd Cross, Indiranagar, Bengaluru 560038".to_string()),
            footer: Some(
                "Rent is payable on or before the 5th of each month; a late fee of \u{20b9}100 per day applies thereafter, and continued non-payment past the 20th may result in suspension of mess and laundry services until dues are cleared.\n\
                 Payments are adjusted against the earliest outstanding dues first.\n\
                 Report any discrepancy within 7 days; thereafter this invoice is deemed accepted.\n\
                 The security deposit is refundable after deduction of damages and pending dues.\n\
                 One calendar month of written notice is required before moving out.\n\
                 This receipt is valid only for the amount and period stated above."
                    .to_string(),
            ),
            accent: Some("#2E7D32".to_string()),
            logo_key: None,
        };
        let data = ReceiptData {
            receipt_number: "RCPT-000918".to_string(),
            invoice_number: "1786421093577".to_string(),
            resident_name: "Lakshmi Priya Venkataraghavan".to_string(),
            branch_name: Some("Sri Lakshmi Venkateswara Ladies PG".to_string()),
            amount_paise: 15_750_000, // INR 1,57,500.00
            issued_epoch: 1_786_421_093,
            tenant_phone: Some("+91 91773 20486".to_string()),
            room_number: Some("204-B".to_string()),
            plan_kind: "Fixed".to_string(),
            line_item: "Security deposit".to_string(),
            payment_method: "Bank transfer (NEFT)".to_string(),
            payment_reference: None,
            floor_label: None,
            recorded_by: "Priya Menon (Manager)".to_string(),
        };
        std::fs::write(&out, SimpleReceiptRenderer.render(&template, &data))?;
        println!("wrote {out}");
        return Ok(());
    }

    let template = ReceiptTemplate {
        display_name: "Hamsa Vilas Premium Women's PG".to_string(),
        address: Some(
            "Phase 1, H.No. 5-3/12, Plot No-12,\n\
             Opposite Dhanya Sree Digital Photo Studio,\n\
             Sai Tulasi Enclave, Gowlidoddy,\n\
             Vattinagulapalle, Hyderabad,\n\
             Telangana 500075\n\
             Mobile: +91 96032 41180"
                .to_string(),
        ),
        footer: Some(
            "Rent is payable on or before the 5th of each month; a late fee of \u{20b9}100 per day applies thereafter.\n\
             Payments are adjusted against the earliest outstanding dues first.\n\
             Report any discrepancy within 7 days at +91 96032 41180; thereafter this invoice is deemed accepted."
                .to_string(),
        ),
        accent: Some("#1F4FBF".to_string()),
        logo_key: None,
    };

    let data = ReceiptData {
        receipt_number: "1786421093577".to_string(),
        invoice_number: "1786421093577".to_string(),
        resident_name: "Sahithi Reddy".to_string(),
        branch_name: Some("Hamsa Vilas".to_string()),
        amount_paise: 1_250_000,
        issued_epoch: 1_786_421_093,
        tenant_phone: Some("+91 91773 20486".to_string()),
        room_number: Some("204".to_string()),
        plan_kind: "Fixed".to_string(),
        line_item: "Rent".to_string(),
        payment_method: "PhonePe".to_string(),
        payment_reference: None,
        floor_label: None,
        recorded_by: "Owner (Admin)".to_string(),
    };

    let bytes = SimpleReceiptRenderer.render(&template, &data);
    std::fs::write(&out, bytes)?;
    println!("wrote {out}");
    Ok(())
}
