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
}
