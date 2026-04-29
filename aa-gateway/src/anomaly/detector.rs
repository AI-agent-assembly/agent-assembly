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

    /// Detect child process execution: any `ProcessExec` action is flagged.
    ///
    /// Returns `Some(AnomalyEvent)` with [`AnomalyResponse::Block`]. Child
    /// process execution is default-deny — agents should not spawn subprocesses
    /// unless explicitly allowed by policy.
    pub fn check_child_process(&self, agent_id: AgentId, command: &str) -> Option<AnomalyEvent> {
        Some(AnomalyEvent {
            anomaly_type: AnomalyType::ChildProcessExecution,
            response: AnomalyResponse::default_for(AnomalyType::ChildProcessExecution),
            agent_id,
            description: format!("Unauthorized child process execution: {command}"),
            detected_at: chrono::Utc::now(),
        })
    }

    /// Detect data exfiltration attempt: PII/credential findings present in a
    /// payload that is being sent to an external host via `NetworkRequest`.
    ///
    /// Returns `Some(AnomalyEvent)` with [`AnomalyResponse::Block`] when
    /// sensitive data is detected in outbound network traffic.
    pub fn check_data_exfiltration(
        &self,
        agent_id: AgentId,
        has_pii: bool,
        url: &str,
    ) -> Option<AnomalyEvent> {
        if !has_pii {
            return None;
        }
        Some(AnomalyEvent {
            anomaly_type: AnomalyType::DataExfiltrationAttempt,
            response: AnomalyResponse::default_for(AnomalyType::DataExfiltrationAttempt),
            agent_id,
            description: format!("PII detected in payload destined for external host: {url}"),
            detected_at: chrono::Utc::now(),
        })
    }

    /// Detect loop runaway: same tool+args called more than N times within
    /// the sliding window.
    ///
    /// Returns `Some(AnomalyEvent)` with [`AnomalyResponse::Pause`] when
    /// identical tool invocations exceed the configured threshold.
    pub fn check_loop_runaway(
        &self,
        agent_id: AgentId,
        tool_name: &str,
        args: &str,
    ) -> Option<AnomalyEvent> {
        let tool_hash = Self::hash_tool_call(tool_name, args);
        let baseline = self.baselines.get(&agent_id)?;
        let count = baseline.tool_call_count(tool_hash);
        if count >= self.config.loop_threshold {
            Some(AnomalyEvent {
                anomaly_type: AnomalyType::LoopRunaway,
                response: AnomalyResponse::default_for(AnomalyType::LoopRunaway),
                agent_id,
                description: format!(
                    "Tool '{tool_name}' called {count} times (threshold: {})",
                    self.config.loop_threshold
                ),
                detected_at: chrono::Utc::now(),
            })
        } else {
            None
        }
    }

    /// Detect cross-agent identity spoofing: the claimed agent ID does not
    /// match the credential owner's agent ID.
    ///
    /// Returns `Some(AnomalyEvent)` with [`AnomalyResponse::Alert`] when
    /// an agent presents credentials belonging to a different agent.
    pub fn check_identity_spoofing(
        &self,
        claimed_agent_id: AgentId,
        credential_owner_id: AgentId,
    ) -> Option<AnomalyEvent> {
        if claimed_agent_id == credential_owner_id {
            return None;
        }
        Some(AnomalyEvent {
            anomaly_type: AnomalyType::CrossAgentIdentitySpoofing,
            response: AnomalyResponse::default_for(AnomalyType::CrossAgentIdentitySpoofing),
            agent_id: claimed_agent_id,
            description: format!(
                "Agent {:?} presented credentials belonging to agent {:?}",
                claimed_agent_id.as_bytes(),
                credential_owner_id.as_bytes()
            ),
            detected_at: chrono::Utc::now(),
        })
    }

    /// Run all applicable anomaly checks for the given action and return the
    /// first detected anomaly (short-circuit, highest severity first).
    ///
    /// Checks are ordered by severity: Block responses before Pause before Alert.
    ///
    /// # Arguments
    ///
    /// * `agent_id` — the agent performing the action
    /// * `action` — the governance action being evaluated
    /// * `has_pii` — whether PII/credential findings were detected in the payload
    /// * `network_allowlist` — the agent's network allowlist from policy
    /// * `credential_owner_id` — if known, the agent ID that owns the credential
    pub fn detect(
        &self,
        agent_id: AgentId,
        action: &aa_core::GovernanceAction,
        has_pii: bool,
        network_allowlist: &[String],
        credential_owner_id: Option<AgentId>,
    ) -> Option<AnomalyEvent> {
        // 1. Child process execution (Block) — highest priority
        if let aa_core::GovernanceAction::ProcessExec { command } = action {
            if let Some(event) = self.check_child_process(agent_id, command) {
                return Some(event);
            }
        }

        // 2. Unknown external connection (Block)
        if let aa_core::GovernanceAction::NetworkRequest { url, .. } = action {
            if let Some(event) = self.check_unknown_connection(agent_id, url, network_allowlist) {
                return Some(event);
            }
        }

        // 3. Data exfiltration attempt (Block)
        if let aa_core::GovernanceAction::NetworkRequest { url, .. } = action {
            if let Some(event) = self.check_data_exfiltration(agent_id, has_pii, url) {
                return Some(event);
            }
        }

        // 4. Loop runaway (Pause)
        if let aa_core::GovernanceAction::ToolCall { name, args } = action {
            if let Some(event) = self.check_loop_runaway(agent_id, name, args) {
                return Some(event);
            }
        }

        // 5. Behavior spike (Pause)
        if let Some(event) = self.check_behavior_spike(agent_id) {
            return Some(event);
        }

        // 6. Credential leak attempt (Alert)
        if let Some(event) = self.check_credential_leak(agent_id) {
            return Some(event);
        }

        // 7. Identity spoofing (Alert)
        if let Some(owner_id) = credential_owner_id {
            if let Some(event) = self.check_identity_spoofing(agent_id, owner_id) {
                return Some(event);
            }
        }

        None
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
