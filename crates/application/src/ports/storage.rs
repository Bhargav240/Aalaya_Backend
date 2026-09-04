//! Storage engine and receipt rendering ports.
use crate::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), AppError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, AppError>;
    /// A short-lived signed URL for direct download, if the backend supports it
    /// (e.g. Supabase/S3). Returns None for backends without signing (local FS),
    /// in which case the caller streams the bytes instead.
    async fn signed_url(&self, key: &str, ttl_secs: i64) -> Result<Option<String>, AppError>;
}

/// Per-org branding for receipts. Same internal data, different look per org.
pub struct ReceiptTemplate {
    pub display_name: String,
    pub address: Option<String>,
    pub footer: Option<String>,
    pub accent: Option<String>,
    pub logo_key: Option<String>,
}

impl ReceiptTemplate {
    /// System default, used when an org has not set its own template.
    pub fn default_template() -> Self {
        Self {
            display_name: "PG Management".to_string(),
            address: None,
            footer: Some("Cash payment received. Thank you.".to_string()),
            accent: None,
            logo_key: None,
        }
    }
}

pub struct ReceiptData {
    pub receipt_number: String,
    pub invoice_number: String,
    pub resident_name: String,
    /// The branch/PG line -- present ONLY for multi-branch orgs; a single PG
    /// gets a branch-free receipt.
    pub branch_name: Option<String>,
    pub amount_paise: i64,
    pub issued_epoch: i64,
    pub tenant_phone: Option<String>,
    pub room_number: Option<String>,
    pub plan_kind: String,
    /// What the money was for, as printed on the invoice line -- "Rent" or
    /// "Security deposit". A deposit receipt must not read as rent.
    pub line_item: String,
    /// How the money arrived, taken from `payments.method` rather than assumed.
    pub payment_method: String,
    /// The provider's reference for a non-cash payment (`payments.upi_ref`).
    /// Cash has none, so this is absent rather than a placeholder.
    pub payment_reference: Option<String>,
    /// The room's floor, when one can be established. Resolved from the room's
    /// assigned floor, falling back to the room-numbering convention.
    pub floor_label: Option<String>,
    pub recorded_by: String,
}

/// Renders a receipt (template + data) to bytes (PDF). One engine, many
/// templates -- swap the engine behind this trait without touching callers.
pub trait ReceiptRenderer: Send + Sync {
    fn render(&self, template: &ReceiptTemplate, data: &ReceiptData) -> Vec<u8>;
}
