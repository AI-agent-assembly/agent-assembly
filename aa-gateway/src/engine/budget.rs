//! Daily spend tracker for per-agent budget enforcement.
//!
//! `BudgetTracker` maintains running daily spend totals per agent,
//! automatically resetting at the configured timezone's midnight boundary.

use chrono::NaiveDate;
use dashmap::DashMap;

fn today_in_tz(tz: chrono_tz::Tz) -> NaiveDate {
    chrono::Utc::now().with_timezone(&tz).date_naive()
}

/// Per-agent daily spend tracker with automatic midnight reset.
pub(crate) struct BudgetTracker {
    /// DashMap<agent_id_bytes, (spent_today, date)>
    /// Maps agent UUID (16 bytes) to (cumulative spend, date recorded).
    /// When date differs from today's date in the configured timezone, spend is reset to 0.
    pub(crate) state: DashMap<[u8; 16], (f64, NaiveDate)>,
    timezone: chrono_tz::Tz,
}

impl BudgetTracker {
    /// Create a new empty spend tracker using the given IANA timezone for daily reset.
    pub(crate) fn new(timezone: chrono_tz::Tz) -> Self {
        Self { state: DashMap::new(), timezone }
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
}
