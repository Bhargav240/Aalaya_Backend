use pg_application::ports::billing::BillingEngine;
use pg_domain::PlanKind;
use time::Date;

/// Day-based proration: monthly ~ rate/30 per day, weekly ~ rate/7, daily = rate.
pub struct SimpleBillingEngine;

impl BillingEngine for SimpleBillingEngine {
    fn accrued_paise(&self, plan: PlanKind, rate_paise: i64, from: Date, to: Date) -> i64 {
        let days = (to - from).whole_days().max(0);
        match plan {
            PlanKind::Monthly => rate_paise.saturating_mul(days) / 30,
            PlanKind::Weekly => rate_paise.saturating_mul(days) / 7,
            PlanKind::Daily => rate_paise.saturating_mul(days),
        }
    }
}
