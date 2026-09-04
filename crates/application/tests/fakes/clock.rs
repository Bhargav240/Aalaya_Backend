//! A `Clock` that returns a fixed, test-controlled instant.
use pg_application::ports::Clock;
use std::sync::Mutex;
use time::OffsetDateTime;

pub struct FixedClock {
    now: Mutex<OffsetDateTime>,
}

impl FixedClock {
    pub fn new(now: OffsetDateTime) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// Convenience: a clock pinned at midnight of a given UTC date.
    pub fn at(date: time::Date) -> Self {
        Self::new(date.midnight().assume_utc())
    }

    /// Advance the pinned instant (e.g. past an OTP cooldown).
    pub fn advance(&self, duration: time::Duration) {
        let mut guard = self.now.lock().expect("clock poisoned");
        *guard += duration;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        *self.now.lock().expect("clock poisoned")
    }
}
