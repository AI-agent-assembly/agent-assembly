#[cfg(feature = "alloc")]
use alloc::{collections::BTreeMap, string::String};

use crate::{
    identity::{AgentId, SessionId},
    time::Timestamp,
};

/// Identity carrier for an agent execution.
///
/// `AgentContext` flows through every governance event in the system.
/// It captures the stable agent identity, per-session identity, process ID,
/// start time, and any additional runtime metadata.
///
/// Requires the `alloc` feature.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentContext {
    /// Stable identifier for the agent (UUID v4 bytes).
    pub agent_id: AgentId,
    /// Per-execution session identifier (UUID v4 bytes).
    pub session_id: SessionId,
    /// OS process ID of the agent process.
    pub pid: u32,
    /// Nanoseconds since the Unix epoch when this context was created.
    pub started_at: Timestamp,
    /// Extensible key-value metadata attached to this execution context.
    pub metadata: BTreeMap<&'static str, String>,
}
