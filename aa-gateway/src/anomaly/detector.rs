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

    // ── Detection methods ─────────────────────────────────────────────

    /// Detect behavior spike: current action rate exceeds baseline mean + N*stddev.
    ///
    /// Returns `Some(AnomalyEvent)` with [`AnomalyResponse::Pause`] when the
    /// agent's recent action count significantly exceeds its historical baseline.
    /// Requires at least 2 prior actions to establish a meaningful baseline.
    pub fn check_behavior_spike(&self, agent_id: AgentId) -> Option<AnomalyEvent> {
        let baseline = self.baselines.get(&agent_id)?;
        let (mean, stddev) = baseline.action_mean_stddev();
        if mean == 0.0 {
            return None;
        }
        let threshold = mean + self.config.spike_stddev_multiplier * stddev;
        let current = baseline.action_count() as f64;
        if current > threshold && stddev > 0.0 {
            Some(AnomalyEvent {
                anomaly_type: AnomalyType::BehaviorSpike,
                response: AnomalyResponse::default_for(AnomalyType::BehaviorSpike),
                agent_id,
                description: format!(
                    "Action count {current} exceeds threshold {threshold:.1} (mean={mean:.1}, stddev={stddev:.1})"
                ),
                detected_at: chrono::Utc::now(),
            })
        } else {
            None
        }
    }

    /// Detect unknown external connection: host not in the network allowlist.
    ///
    /// Returns `Some(AnomalyEvent)` with [`AnomalyResponse::Block`] when the
    /// URL's host is not present in the provided allowlist. An empty allowlist
    /// means all hosts are allowed (open policy).
    pub fn check_unknown_connection(
        &self,
        agent_id: AgentId,
        url: &str,
        allowlist: &[String],
    ) -> Option<AnomalyEvent> {
        if allowlist.is_empty() {
            return None;
        }
        let host = url
            .split_once("://")
            .map(|x| x.1)
            .unwrap_or(url)
            .split('/')
            .next()
            .unwrap_or("");
        if allowlist.iter().any(|entry| entry == host) {
            return None;
        }
        Some(AnomalyEvent {
            anomaly_type: AnomalyType::UnknownExternalConnection,
            response: AnomalyResponse::default_for(AnomalyType::UnknownExternalConnection),
            agent_id,
            description: format!("Connection to host '{host}' not in network allowlist"),
            detected_at: chrono::Utc::now(),
        })
    }

    /// Detect credential leak attempt: accumulated findings exceed threshold.
    ///
    /// Returns `Some(AnomalyEvent)` with [`AnomalyResponse::Alert`] when the
    /// agent has accumulated more credential findings in the current window
    /// than the configured threshold.
    pub fn check_credential_leak(&self, agent_id: AgentId) -> Option<AnomalyEvent> {
        let baseline = self.baselines.get(&agent_id)?;
        let count = baseline.credential_findings_count();
        if count >= self.config.credential_leak_threshold {
            Some(AnomalyEvent {
                anomaly_type: AnomalyType::CredentialLeakAttempt,
                response: AnomalyResponse::default_for(AnomalyType::CredentialLeakAttempt),
                agent_id,
                description: format!(
                    "Credential findings count {count} exceeds threshold {}",
                    self.config.credential_leak_threshold
                ),
                detected_at: chrono::Utc::now(),
            })
        } else {
            None
        }
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
