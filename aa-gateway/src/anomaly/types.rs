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
