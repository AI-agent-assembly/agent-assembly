//! Anomaly detection logic.
//!
//! Compares current agent activity against the per-agent behavioral baseline
//! to identify each of the seven anomaly types defined in the Governance
//! Gateway epic (AAASM-8 AC #5).

use dashmap::DashMap;
use sha2::{Digest, Sha256};

use aa_core::AgentId;

use super::baseline::AgentBaseline;
use super::types::{AnomalyConfig, AnomalyEvent, AnomalyResponse, AnomalyType};

/// Anomaly detection engine maintaining per-agent baselines.
///
/// Thread-safe: all methods take `&self`. Per-agent state is stored in a
/// `DashMap` (same pattern as `BudgetTracker`).
pub struct AnomalyDetector {
    baselines: DashMap<AgentId, AgentBaseline>,
    config: AnomalyConfig,
}

impl AnomalyDetector {
    /// Create a new detector with the given configuration.
    pub fn new(config: AnomalyConfig) -> Self {
        Self {
            baselines: DashMap::new(),
            config,
        }
    }

    /// Record an action for an agent, updating its baseline.
    pub fn record_action(&self, agent_id: AgentId, now_ms: u64) {
        self.baselines
            .entry(agent_id)
            .or_insert_with(|| AgentBaseline::new(self.config.baseline_window_secs))
            .record_action(now_ms);
    }

    /// Record a tool call for an agent, updating its baseline with the
    /// tool+args hash.
    pub fn record_tool_call(&self, agent_id: AgentId, tool_name: &str, args: &str, now_ms: u64) {
        let tool_hash = Self::hash_tool_call(tool_name, args);
        self.baselines
            .entry(agent_id)
            .or_insert_with(|| AgentBaseline::new(self.config.baseline_window_secs))
            .record_tool_call(tool_hash, now_ms);
    }

    /// Record a credential finding for an agent.
    pub fn record_credential_finding(&self, agent_id: AgentId) {
        self.baselines
            .entry(agent_id)
            .or_insert_with(|| AgentBaseline::new(self.config.baseline_window_secs))
            .record_credential_finding();
    }

    /// Compute a stable hash for a (tool_name, args) pair.
    fn hash_tool_call(tool_name: &str, args: &str) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(tool_name.as_bytes());
        hasher.update(b":");
        hasher.update(args.as_bytes());
        let result = hasher.finalize();
        u64::from_le_bytes(result[..8].try_into().unwrap())
    }
}
