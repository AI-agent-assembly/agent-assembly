//! Core domain types for the anomaly detection engine.

/// Classification of anomalous agent behavior.
///
/// Each variant corresponds to one of the seven anomaly types defined in the
/// Governance Gateway epic (AAASM-8 AC #5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyType {
    /// Action rate suddenly exceeds historical baseline (e.g. 5/hr to 200/hr).
    BehaviorSpike,
    /// Agent attempts connection to a host/IP not in the network allowlist.
    UnknownExternalConnection,
    /// Repeated credential patterns detected in agent payloads.
    CredentialLeakAttempt,
    /// Agent spawns a child process (e.g. `bash -c "curl ..."`).
    ChildProcessExecution,
    /// PII detected in a payload destined for an external API.
    DataExfiltrationAttempt,
    /// Same tool+args invoked repeatedly within a short window.
    LoopRunaway,
    /// Agent A presents credentials belonging to Agent B.
    CrossAgentIdentitySpoofing,
}

impl AnomalyType {
    /// Human-readable description of this anomaly type.
    pub fn description(&self) -> &'static str {
        match self {
            Self::BehaviorSpike => "Action rate spike exceeding behavioral baseline",
            Self::UnknownExternalConnection => "Connection attempt to host not in network allowlist",
            Self::CredentialLeakAttempt => "Credential pattern detected in agent payload",
            Self::ChildProcessExecution => "Unauthorized child process execution",
            Self::DataExfiltrationAttempt => "PII detected in payload to external API",
            Self::LoopRunaway => "Repeated identical tool invocations in short window",
            Self::CrossAgentIdentitySpoofing => "Agent presenting another agent's credentials",
        }
    }
}

/// Automated response action triggered when an anomaly is detected.
///
/// Each response maps to an enforcement action that the gateway executes
/// without human intervention. The mapping from [`AnomalyType`] to default
/// response follows the Governance Gateway epic anomaly table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyResponse {
    /// Temporarily suspend the agent; it can be resumed after review.
    Pause,
    /// Immediately block the current action and deny further actions.
    Block,
    /// Emit an alert notification without interrupting the agent.
    Alert,
    /// Isolate the agent: block all actions and flag for security review.
    Quarantine,
}

impl AnomalyResponse {
    /// Returns the default response for a given anomaly type, per the epic
    /// anomaly table (AAASM-8).
    ///
    /// | Anomaly | Default Response |
    /// |---------|-----------------|
    /// | BehaviorSpike | Pause |
    /// | UnknownExternalConnection | Block |
    /// | CredentialLeakAttempt | Alert |
    /// | ChildProcessExecution | Block |
    /// | DataExfiltrationAttempt | Block |
    /// | LoopRunaway | Pause |
    /// | CrossAgentIdentitySpoofing | Alert |
    pub fn default_for(anomaly_type: AnomalyType) -> Self {
        match anomaly_type {
            AnomalyType::BehaviorSpike => Self::Pause,
            AnomalyType::UnknownExternalConnection => Self::Block,
            AnomalyType::CredentialLeakAttempt => Self::Alert,
            AnomalyType::ChildProcessExecution => Self::Block,
            AnomalyType::DataExfiltrationAttempt => Self::Block,
            AnomalyType::LoopRunaway => Self::Pause,
            AnomalyType::CrossAgentIdentitySpoofing => Self::Alert,
        }
    }
}

/// An anomaly detection event emitted when the engine identifies suspicious
/// agent behavior.
///
/// Carries the anomaly classification, the chosen response action, and enough
/// context to populate an [`AlertTriggered`](proto) message once the event bus
/// (AAASM-141) is wired up.
#[derive(Debug, Clone)]
pub struct AnomalyEvent {
    /// What kind of anomaly was detected.
    pub anomaly_type: AnomalyType,
    /// The response action that was (or will be) executed.
    pub response: AnomalyResponse,
    /// The agent that triggered the anomaly.
    pub agent_id: aa_core::AgentId,
    /// Human-readable explanation of the detection.
    pub description: String,
    /// When the anomaly was detected.
    pub detected_at: chrono::DateTime<chrono::Utc>,
}
