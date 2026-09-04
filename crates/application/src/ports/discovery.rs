//! Public discovery (marketplace) port. Reads/writes a marketing-only listings
//! projection in the control plane -- NEVER operational/tenant data.
use crate::error::AppError;
use async_trait::async_trait;
use pg_domain::{BranchId, OrgId};
use uuid::Uuid;

pub struct ListingInput {
    pub title: String,
    pub area: Option<String>,
    pub city: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub description: Option<String>,
    pub amenities: Option<String>,
}

/// The moderation status the Platform Operator gates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListingStatus {
    Draft,
    Pending,
    Live,
    Rejected,
    Unlisted,
}

impl ListingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Pending => "pending",
            Self::Live => "live",
            Self::Rejected => "rejected",
            Self::Unlisted => "unlisted",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "pending" => Some(Self::Pending),
            "live" => Some(Self::Live),
            "rejected" => Some(Self::Rejected),
            "unlisted" => Some(Self::Unlisted),
            _ => None,
        }
    }
}

pub struct Listing {
    pub branch_id: Uuid,
    pub org_id: Uuid,
    pub title: String,
    pub area: Option<String>,
    pub city: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub description: Option<String>,
    pub amenities: Option<String>,
    pub status: ListingStatus,
    /// Live, bed-derived vacancy: "3 beds free from ₹8,000". Never hand-typed.
    pub beds_available: Option<i32>,
    pub starting_price_paise: Option<i64>,
    pub distance_km: Option<f64>,
}

#[async_trait]
pub trait ListingRepository: Send + Sync {
    /// Create/update the prepared listing content. Never flips live — that's
    /// the operator's gate.
    async fn upsert_draft(
        &self,
        org_id: OrgId,
        branch_id: BranchId,
        input: &ListingInput,
    ) -> Result<(), AppError>;
    async fn set_status(
        &self,
        branch_id: BranchId,
        status: ListingStatus,
        note: Option<&str>,
    ) -> Result<(), AppError>;
    async fn list_by_org(&self, org_id: OrgId) -> Result<Vec<Listing>, AppError>;
    async fn list_by_status(&self, status: ListingStatus) -> Result<Vec<Listing>, AppError>;
    async fn list_published(&self, limit: i64) -> Result<Vec<Listing>, AppError>;
    async fn search(
        &self,
        q: Option<String>,
        city: Option<String>,
        limit: i64,
    ) -> Result<Vec<Listing>, AppError>;
    async fn nearby(&self, lat: f64, lng: f64, limit: i64) -> Result<Vec<Listing>, AppError>;
    async fn get(&self, branch_id: Uuid) -> Result<Option<Listing>, AppError>;
}
