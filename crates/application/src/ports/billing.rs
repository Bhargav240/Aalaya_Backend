//! Billing computation port.
use pg_domain::PlanKind;
use time::Date;

/// Computes rent accrued for a plan between two dates (per-bed). Proration is
/// day-based (monthly ~ /30, weekly ~ /7). Real calendar proration is a later
/// refinement -- the port keeps callers stable either way.
pub trait BillingEngine: Send + Sync {
    fn accrued_paise(&self, plan: PlanKind, rate_paise: i64, from: Date, to: Date) -> i64;
}
