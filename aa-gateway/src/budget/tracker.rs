//! Per-agent and global LLM spend tracker.

use std::sync::Mutex;

use dashmap::DashMap;
use rust_decimal::Decimal;
use tokio::sync::broadcast;

use aa_core::AgentId;

use crate::budget::{
    pricing::PricingTable,
    types::{BudgetAlert, BudgetState},
};

const ALERT_CHANNEL_CAPACITY: usize = 64;

/// Per-agent and global budget tracker. All methods take `&self` — safe to share via `Arc`.
#[allow(dead_code)]
pub struct BudgetTracker {
    /// Per-agent daily spend. `pub(crate)` for test date manipulation.
    pub(crate) per_agent: DashMap<AgentId, BudgetState>,
    pub(crate) global: Mutex<BudgetState>,
    pricing: PricingTable,
    daily_limit_usd: Option<Decimal>,
    alert_tx: broadcast::Sender<BudgetAlert>,
}

impl BudgetTracker {
    /// Create a new tracker with no prior state.
    pub fn new(pricing: PricingTable, daily_limit_usd: Option<Decimal>) -> Self {
        let (alert_tx, _) = broadcast::channel(ALERT_CHANNEL_CAPACITY);
        Self {
            per_agent: DashMap::new(),
            global: Mutex::new(BudgetState::new_today()),
            pricing,
            daily_limit_usd,
            alert_tx,
        }
    }

    /// Create a tracker pre-loaded with persisted state (call after `load_from_disk`).
    pub fn with_state(
        pricing: PricingTable,
        daily_limit_usd: Option<Decimal>,
        initial: crate::budget::persistence::PersistedBudget,
    ) -> Self {
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
        }
    }

    /// Subscribe to budget threshold alert events (80% and 95% crossings).
    pub fn subscribe_alerts(&self) -> broadcast::Receiver<BudgetAlert> {
        self.alert_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::pricing::PricingTable;
    use rust_decimal::Decimal;

    fn new_tracker() -> BudgetTracker {
        BudgetTracker::new(PricingTable::default_table(), None)
    }

    #[test]
    fn new_tracker_has_empty_per_agent_map() {
        let t = new_tracker();
        assert!(t.per_agent.is_empty());
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
        };
        let t = BudgetTracker::with_state(PricingTable::default_table(), None, persisted);
        let entry = t.per_agent.get(&id).unwrap();
        assert_eq!(entry.spent_usd, state.spent_usd);
    }
}
