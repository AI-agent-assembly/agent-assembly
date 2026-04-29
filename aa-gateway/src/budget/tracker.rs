//! Per-agent and global LLM spend tracker.

use std::sync::Mutex;

use dashmap::DashMap;
use rust_decimal::Decimal;
use tokio::sync::broadcast;

use aa_core::AgentId;

use rust_decimal::prelude::ToPrimitive;

use crate::budget::{
    pricing::PricingTable,
    types::{BudgetAlert, BudgetState, BudgetStatus},
};

const ALERT_CHANNEL_CAPACITY: usize = 64;
const ALERT_PCT_HIGH: u8 = 95;
const ALERT_PCT_LOW: u8 = 80;

fn compute_status(spent: Decimal, limit: Decimal) -> BudgetStatus {
    if spent >= limit {
        return BudgetStatus::LimitExceeded;
    }
    let pct = (spent / limit * Decimal::ONE_HUNDRED)
        .round_dp(0)
        .to_u8()
        .unwrap_or(100);
    let spent_f = spent.to_f64().unwrap_or(0.0);
    let limit_f = limit.to_f64().unwrap_or(0.0);
    if pct >= ALERT_PCT_HIGH {
        BudgetStatus::ThresholdAlert { pct: ALERT_PCT_HIGH }
    } else if pct >= ALERT_PCT_LOW {
        BudgetStatus::ThresholdAlert { pct: ALERT_PCT_LOW }
    } else {
        BudgetStatus::WithinBudget {
            spent_usd: spent_f,
            remaining_usd: limit_f - spent_f,
        }
    }
}

fn today_in_tz(tz: chrono_tz::Tz) -> chrono::NaiveDate {
    chrono::Utc::now().with_timezone(&tz).date_naive()
}

/// Per-agent and global budget tracker. All methods take `&self` — safe to share via `Arc`.
pub struct BudgetTracker {
    /// Per-agent daily spend. `pub(crate)` for test date manipulation.
    pub(crate) per_agent: DashMap<AgentId, BudgetState>,
    pub(crate) global: Mutex<BudgetState>,
    pricing: PricingTable,
    daily_limit_usd: Option<Decimal>,
    alert_tx: broadcast::Sender<BudgetAlert>,
    timezone: chrono_tz::Tz,
}

impl BudgetTracker {
    /// Create a new tracker with no prior state.
    pub fn new(pricing: PricingTable, daily_limit_usd: Option<Decimal>, timezone: chrono_tz::Tz) -> Self {
        let (alert_tx, _) = broadcast::channel(ALERT_CHANNEL_CAPACITY);
        Self {
            per_agent: DashMap::new(),
            global: Mutex::new(BudgetState::new_for_date(today_in_tz(timezone))),
            pricing,
            daily_limit_usd,
            alert_tx,
            timezone,
        }
    }

    /// Create a tracker pre-loaded with persisted state (call after `load_from_disk`).
    pub fn with_state(
        pricing: PricingTable,
        daily_limit_usd: Option<Decimal>,
        initial: crate::budget::persistence::PersistedBudget,
    ) -> Self {
        let timezone = initial.timezone;
        let (alert_tx, _) = broadcast::channel(ALERT_CHANNEL_CAPACITY);
        let per_agent: DashMap<AgentId, BudgetState> = initial
            .per_agent
            .into_iter()
            .filter_map(|e| {
                crate::budget::persistence::hex_to_agent_id(&e.agent_id_hex)
                    .ok()
                    .map(|id| (id, e.state))
            })
            .collect();
        Self {
            per_agent,
            global: Mutex::new(initial.global),
            pricing,
            daily_limit_usd,
            alert_tx,
            timezone,
        }
    }

    /// Subscribe to budget threshold alert events (80% and 95% crossings).
    pub fn subscribe_alerts(&self) -> broadcast::Receiver<BudgetAlert> {
        self.alert_tx.subscribe()
    }

    /// Returns the configured timezone for daily reset boundaries.
    pub fn timezone(&self) -> chrono_tz::Tz {
        self.timezone
    }

    /// Record token usage and return the resulting [`BudgetStatus`].
    pub fn record_usage(
        &self,
        agent_id: AgentId,
        provider: crate::budget::types::Provider,
        model: crate::budget::types::Model,
        input_tokens: u64,
        output_tokens: u64,
    ) -> BudgetStatus {
        let cost = self.pricing.cost_usd(provider, model, input_tokens, output_tokens);

        self.per_agent
            .entry(agent_id)
            .and_modify(|s| {
                s.maybe_reset(today_in_tz(self.timezone));
                s.spent_usd += cost;
            })
            .or_insert_with(|| {
                let mut s = BudgetState::new_for_date(today_in_tz(self.timezone));
                s.spent_usd += cost;
                s
            });

        let spent = self.per_agent.get(&agent_id).map(|s| s.spent_usd).unwrap_or(cost);

        if let Ok(mut g) = self.global.lock() {
            g.maybe_reset(today_in_tz(self.timezone));
            g.spent_usd += cost;
        }

        let status = match self.daily_limit_usd {
            None => BudgetStatus::WithinBudget {
                spent_usd: spent.to_f64().unwrap_or(0.0),
                remaining_usd: f64::INFINITY,
            },
            Some(limit) => compute_status(spent, limit),
        };

        if let Some(limit) = self.daily_limit_usd {
            if let BudgetStatus::ThresholdAlert { pct } = &status {
                let _ = self.alert_tx.send(BudgetAlert {
                    agent_id,
                    threshold_pct: *pct,
                    spent_usd: spent.to_f64().unwrap_or(0.0),
                    limit_usd: limit.to_f64().unwrap_or(0.0),
                });
            }
        }

        status
    }

    /// Return a snapshot of the current global (all-agents combined) budget state.
    pub fn global_state(&self) -> BudgetState {
        self.global
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| BudgetState::new_for_date(today_in_tz(self.timezone)))
    }

    /// Snapshot the full tracker state for disk persistence.
    pub fn snapshot(&self) -> crate::budget::persistence::PersistedBudget {
        let per_agent = self
            .per_agent
            .iter()
            .map(|entry| crate::budget::persistence::PersistedAgentEntry {
                agent_id_hex: crate::budget::persistence::agent_id_to_hex(entry.key()),
                state: entry.value().clone(),
            })
            .collect();
        crate::budget::persistence::PersistedBudget {
            per_agent,
            global: self.global_state(),
            timezone: self.timezone,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::pricing::PricingTable;
    use rust_decimal::Decimal;

    fn new_tracker() -> BudgetTracker {
        BudgetTracker::new(PricingTable::default_table(), None, chrono_tz::UTC)
    }

    fn agent(b: u8) -> AgentId {
        AgentId::from_bytes([b; 16])
    }

    fn tracker_with_limit(s: &str) -> BudgetTracker {
        BudgetTracker::new(PricingTable::default_table(), Some(s.parse().unwrap()), chrono_tz::UTC)
    }

    #[test]
    fn new_tracker_has_empty_per_agent_map() {
        let t = new_tracker();
        assert!(t.per_agent.is_empty());
    }

    #[test]
    fn compute_status_returns_within_budget_below_80() {
        use crate::budget::types::BudgetStatus;
        fn d(s: &str) -> Decimal {
            s.parse().unwrap()
        }
        let status = compute_status(d("7.00"), d("10.00")); // 70%
        assert!(matches!(status, BudgetStatus::WithinBudget { .. }));
    }

    #[test]
    fn compute_status_returns_alert_at_80() {
        use crate::budget::types::BudgetStatus;
        fn d(s: &str) -> Decimal {
            s.parse().unwrap()
        }
        let status = compute_status(d("8.00"), d("10.00")); // exactly 80%
        assert_eq!(status, BudgetStatus::ThresholdAlert { pct: 80 });
    }

    #[test]
    fn compute_status_returns_alert_at_95() {
        use crate::budget::types::BudgetStatus;
        fn d(s: &str) -> Decimal {
            s.parse().unwrap()
        }
        let status = compute_status(d("9.50"), d("10.00")); // exactly 95%
        assert_eq!(status, BudgetStatus::ThresholdAlert { pct: 95 });
    }

    #[test]
    fn compute_status_returns_limit_exceeded_at_100() {
        use crate::budget::types::BudgetStatus;
        fn d(s: &str) -> Decimal {
            s.parse().unwrap()
        }
        assert_eq!(compute_status(d("10.00"), d("10.00")), BudgetStatus::LimitExceeded);
        assert_eq!(compute_status(d("11.00"), d("10.00")), BudgetStatus::LimitExceeded);
    }

    #[test]
    fn record_usage_no_limit_returns_within_budget() {
        use crate::budget::types::{BudgetStatus, Model, Provider};
        let t = new_tracker();
        let s = t.record_usage(agent(1), Provider::OpenAi, Model::Gpt4o, 100, 100);
        assert!(matches!(s, BudgetStatus::WithinBudget { .. }));
    }

    #[test]
    fn record_usage_over_limit_returns_limit_exceeded() {
        use crate::budget::types::{BudgetStatus, Model, Provider};
        // GPT-4o: 100k input=$0.50 + 40k output=$0.60 = $1.10 > $1.00 limit
        let t = tracker_with_limit("1.00");
        let s = t.record_usage(agent(2), Provider::OpenAi, Model::Gpt4o, 100_000, 40_000);
        assert_eq!(s, BudgetStatus::LimitExceeded);
    }

    #[test]
    fn record_usage_alert_at_80_pct() {
        use crate::budget::types::{BudgetStatus, Model, Provider};
        // 100k input=$0.50 + 20k output=$0.30 = $0.80 = 80% of $1.00
        let t = tracker_with_limit("1.00");
        let s = t.record_usage(agent(3), Provider::OpenAi, Model::Gpt4o, 100_000, 20_000);
        assert_eq!(s, BudgetStatus::ThresholdAlert { pct: 80 });
    }

    #[test]
    fn record_usage_resets_on_old_date() {
        use crate::budget::types::{BudgetStatus, Model, Provider};
        let t = tracker_with_limit("1.00");
        let id = agent(4);
        t.record_usage(id, Provider::OpenAi, Model::Gpt4o, 100_000, 30_000); // $0.95
        t.per_agent.alter(&id, |_, mut s| {
            s.date = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
            s
        });
        let s = t.record_usage(id, Provider::OpenAi, Model::Gpt4o, 100, 0);
        assert!(matches!(s, BudgetStatus::WithinBudget { .. }));
    }

    #[test]
    fn subscribe_alerts_returns_receiver() {
        let t = new_tracker();
        let _rx = t.subscribe_alerts(); // compiles and doesn't panic
    }

    #[test]
    fn with_state_restores_per_agent_entries() {
        use crate::budget::persistence::{agent_id_to_hex, PersistedAgentEntry, PersistedBudget};
        let id = AgentId::from_bytes([42u8; 16]);
        let state = BudgetState {
            spent_usd: "5.00".parse::<Decimal>().unwrap(),
            date: chrono::Utc::now().date_naive(),
        };
        let persisted = PersistedBudget {
            per_agent: vec![PersistedAgentEntry {
                agent_id_hex: agent_id_to_hex(&id),
                state: state.clone(),
            }],
            global: BudgetState::new_today(),
            timezone: chrono_tz::UTC,
        };
        let t = BudgetTracker::with_state(PricingTable::default_table(), None, persisted);
        let entry = t.per_agent.get(&id).unwrap();
        assert_eq!(entry.spent_usd, state.spent_usd);
        assert_eq!(t.timezone(), chrono_tz::UTC);
    }

    #[test]
    fn snapshot_includes_per_agent_and_global() {
        use crate::budget::types::{Model, Provider};
        let t = new_tracker();
        let id = agent(7);
        t.record_usage(id, Provider::OpenAi, Model::Gpt4o, 1_000, 0);
        let snap = t.snapshot();
        assert_eq!(snap.per_agent.len(), 1);
        assert_eq!(snap.global.spent_usd, snap.per_agent[0].state.spent_usd);
    }

    #[test]
    fn global_state_accumulates_all_agents() {
        use crate::budget::types::{Model, Provider};
        let t = new_tracker();
        t.record_usage(agent(5), Provider::OpenAi, Model::Gpt4o, 1_000, 0); // $0.005
        t.record_usage(agent(6), Provider::OpenAi, Model::Gpt4o, 1_000, 0); // $0.005
        let g = t.global_state();
        let expected: Decimal = "0.010".parse().unwrap();
        assert_eq!(g.spent_usd, expected);
    }

    #[test]
    fn record_usage_timezone_offset_resets_at_local_midnight() {
        use crate::budget::types::{BudgetStatus, Model, Provider};
        // Use UTC+9 (Asia/Tokyo). We simulate a stale "yesterday in Tokyo" entry
        // to verify that maybe_reset triggers when the configured timezone's date has advanced.
        let tz = chrono_tz::Asia::Tokyo;
        let t = BudgetTracker::new(PricingTable::default_table(), Some("1.00".parse().unwrap()), tz);
        let id = agent(10);
        // First call — establishes the agent entry
        t.record_usage(id, Provider::OpenAi, Model::Gpt4o, 100_000, 30_000); // $0.95
                                                                             // Backdate the agent entry by 1 day in the Tokyo timezone
        let yesterday_tokyo = today_in_tz(tz) - chrono::Duration::days(1);
        t.per_agent.alter(&id, |_, mut s| {
            s.date = yesterday_tokyo;
            s
        });
        // Next call should reset (yesterday < today in Tokyo)
        let s = t.record_usage(id, Provider::OpenAi, Model::Gpt4o, 100, 0);
        assert!(
            matches!(s, BudgetStatus::WithinBudget { .. }),
            "Expected reset after Tokyo midnight, got: {:?}",
            s
        );
    }
}
