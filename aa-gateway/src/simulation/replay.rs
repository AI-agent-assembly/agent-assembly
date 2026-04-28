//! Historical audit log replay for dry-run policy simulation.

use serde::Deserialize;

/// A single event extracted from an audit log for simulation replay.
///
/// This is a deserialized subset of `aa_core::AuditEntry` — only the fields
/// needed for policy re-evaluation.
#[derive(Debug, Clone, Deserialize)]
pub struct SimulationEvent {
    /// The audit event type (e.g. "ToolCallIntercepted", "PolicyViolation").
    pub event_type: String,
    /// The agent identifier that produced this event.
    pub agent_id: String,
    /// Pre-serialized JSON payload from the original audit entry.
    pub payload: String,
}
