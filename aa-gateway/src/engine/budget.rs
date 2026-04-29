//! Daily and monthly spend tracker for per-agent budget enforcement.
//!
//! `BudgetTracker` maintains running daily and monthly spend totals per agent,
//! automatically resetting at the configured timezone's midnight/month boundary.

use chrono::{Datelike, NaiveDate};
use dashmap::DashMap;

fn today_in_tz(tz: chrono_tz::Tz) -> NaiveDate {
    chrono::Utc::now().with_timezone(&tz).date_naive()
}

fn month_tag(date: NaiveDate) -> u32 {
    date.year() as u32 * 100 + date.month()
}

/// Per-agent daily and monthly spend tracker with automatic reset.
pub(crate) struct BudgetTracker {
    /// DashMap<agent_id_bytes, (spent_today, date)>
    pub(crate) state: DashMap<[u8; 16], (f64, NaiveDate)>,
    /// DashMap<agent_id_bytes, (spent_this_month, month_tag)>
    pub(crate) monthly_state: DashMap<[u8; 16], (f64, u32)>,
    timezone: chrono_tz::Tz,
}

impl BudgetTracker {
    /// Create a new empty spend tracker using the given IANA timezone for daily reset.
    pub(crate) fn new(timezone: chrono_tz::Tz) -> Self {
        Self {
            state: DashMap::new(),
            monthly_state: DashMap::new(),
            timezone,
        }
    }

    /// Returns true if agent has already met or exceeded their daily limit.
    ///
    /// Automatically resets spend to 0 if the stored date is before today in the
    /// configured timezone. After any reset, returns `spent >= limit`.
    pub(crate) fn is_exceeded(&self, agent_id: &[u8; 16], limit: f64) -> bool {
        let today = today_in_tz(self.timezone);

        // If entry exists, check if date needs reset
        if let Some(mut entry) = self.state.get_mut(agent_id) {
            let (spent, recorded_date) = entry.value_mut();

            // Reset if the date has changed
            if *recorded_date < today {
                *spent = 0.0;
                *recorded_date = today;
            }

            *spent >= limit
        } else {
            // New agent: not exceeded
            false
        }
    }

    /// Add amount to this agent's running daily total.
    ///
    /// Automatically resets spend to 0 if the stored date is before today in the
    /// configured timezone. After any reset, adds `amount` to the spend total.
    pub(crate) fn record(&self, agent_id: &[u8; 16], amount: f64) {
        let today = today_in_tz(self.timezone);

        self.state
            .entry(*agent_id)
            .and_modify(|(spent, recorded_date)| {
                // Reset if the date has changed
                if *recorded_date < today {
                    *spent = 0.0;
                    *recorded_date = today;
                }
                *spent += amount;
            })
            .or_insert_with(|| (amount, today));
    }

    /// Returns true if agent has met or exceeded their monthly limit.
    pub(crate) fn is_monthly_exceeded(&self, agent_id: &[u8; 16], limit: f64) -> bool {
        let current_month = month_tag(today_in_tz(self.timezone));

        if let Some(mut entry) = self.monthly_state.get_mut(agent_id) {
            let (spent, stored_month) = entry.value_mut();
            if *stored_month != current_month {
                *spent = 0.0;
                *stored_month = current_month;
            }
            *spent >= limit
        } else {
            false
        }
    }

    /// Add amount to this agent's running monthly total.
    pub(crate) fn record_monthly(&self, agent_id: &[u8; 16], amount: f64) {
        let current_month = month_tag(today_in_tz(self.timezone));

        self.monthly_state
            .entry(*agent_id)
            .and_modify(|(spent, stored_month)| {
                if *stored_month != current_month {
                    *spent = 0.0;
                    *stored_month = current_month;
                }
                *spent += amount;
            })
            .or_insert_with(|| (amount, current_month));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_agent_is_not_exceeded() {
        let tracker = BudgetTracker::new(chrono_tz::UTC);
        let agent_id = [0u8; 16];

        assert!(!tracker.is_exceeded(&agent_id, 100.0));
    }

    #[test]
    fn record_accumulates_spend() {
        let tracker = BudgetTracker::new(chrono_tz::UTC);
        let agent_id = [1u8; 16];

        tracker.record(&agent_id, 0.5);
        tracker.record(&agent_id, 0.6);

        // 0.5 + 0.6 = 1.1, which is >= 1.0
        assert!(tracker.is_exceeded(&agent_id, 1.0));
    }

    #[test]
    fn exact_limit_is_exceeded() {
        let tracker = BudgetTracker::new(chrono_tz::UTC);
        let agent_id = [2u8; 16];

        tracker.record(&agent_id, 1.0);

        // 1.0 >= 1.0 is true (not strictly greater)
        assert!(tracker.is_exceeded(&agent_id, 1.0));
    }

    #[test]
    fn spend_resets_on_new_date() {
        let tracker = BudgetTracker::new(chrono_tz::UTC);
        let agent_id = [3u8; 16];

        tracker.record(&agent_id, 0.9);

        // Directly mutate the stored date to yesterday
        if let Some(mut entry) = tracker.state.get_mut(&agent_id) {
            entry.1 = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
        }

        // After reset, spend should be 0.0, so 0.0 < 1.0
        assert!(!tracker.is_exceeded(&agent_id, 1.0));
    }

    #[test]
    fn timezone_is_stored() {
        let tracker = BudgetTracker::new(chrono_tz::Asia::Tokyo);
        assert_eq!(tracker.timezone, chrono_tz::Asia::Tokyo);
    }

    #[test]
    fn new_agent_monthly_is_not_exceeded() {
        let tracker = BudgetTracker::new(chrono_tz::UTC);
        let agent_id = [10u8; 16];
        assert!(!tracker.is_monthly_exceeded(&agent_id, 100.0));
    }

    #[test]
    fn record_monthly_accumulates() {
        let tracker = BudgetTracker::new(chrono_tz::UTC);
        let agent_id = [11u8; 16];

        tracker.record_monthly(&agent_id, 3.0);
        tracker.record_monthly(&agent_id, 4.0);

        assert!(tracker.is_monthly_exceeded(&agent_id, 7.0));
        assert!(!tracker.is_monthly_exceeded(&agent_id, 8.0));
    }

    #[test]
    fn monthly_resets_on_month_change() {
        use chrono::Datelike;
        let tracker = BudgetTracker::new(chrono_tz::UTC);
        let agent_id = [12u8; 16];

        tracker.record_monthly(&agent_id, 5.0);

        // Backdate to a different month
        if let Some(mut entry) = tracker.monthly_state.get_mut(&agent_id) {
            let today = chrono::Utc::now().date_naive();
            let last_month = today - chrono::Duration::days(32);
            entry.1 = last_month.year() as u32 * 100 + last_month.month();
        }

        // After month change, spend resets — should not be exceeded
        assert!(!tracker.is_monthly_exceeded(&agent_id, 5.0));
    }
}
