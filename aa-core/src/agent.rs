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

#[cfg(all(feature = "alloc", feature = "std"))]
impl AgentContext {
    /// Construct an [`AgentContext`] stamped at the current wall-clock time.
    ///
    /// `metadata` is initialised empty; insert entries after construction.
    pub fn now(agent_id: AgentId, session_id: SessionId, pid: u32) -> Self {
        Self {
            started_at: Timestamp::from(std::time::SystemTime::now()),
            agent_id,
            session_id,
            pid,
            metadata: BTreeMap::new(),
        }
    }
}
