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

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;

    const AGENT_BYTES: [u8; 16] = [1; 16];
    const SESSION_BYTES: [u8; 16] = [2; 16];

    fn make_context() -> AgentContext {
        AgentContext {
            agent_id: AgentId::from_bytes(AGENT_BYTES),
            session_id: SessionId::from_bytes(SESSION_BYTES),
            pid: 42,
            started_at: Timestamp::from_nanos(1_000_000),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn field_access() {
        let ctx = make_context();
        assert_eq!(ctx.agent_id.as_bytes(), &AGENT_BYTES);
        assert_eq!(ctx.session_id.as_bytes(), &SESSION_BYTES);
        assert_eq!(ctx.pid, 42);
        assert_eq!(ctx.started_at.as_nanos(), 1_000_000);
        assert!(ctx.metadata.is_empty());
    }

    #[test]
    fn clone_equals_original() {
        let ctx = make_context();
        assert_eq!(ctx.clone(), ctx);
    }

    #[test]
    fn equality() {
        let a = make_context();
        let b = make_context();
        assert_eq!(a, b);
    }

    #[test]
    fn inequality_on_different_pid() {
        let a = make_context();
        let mut b = make_context();
        b.pid = 99;
        assert_ne!(a, b);
    }

    #[cfg(feature = "std")]
    #[test]
    fn now_constructor_sets_nonzero_timestamp() {
        let ctx = AgentContext::now(
            AgentId::from_bytes(AGENT_BYTES),
            SessionId::from_bytes(SESSION_BYTES),
            std::process::id(),
        );
        assert!(ctx.started_at.as_nanos() > 0);
        assert!(ctx.metadata.is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let original = make_context();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: AgentContext = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored);
    }
}
