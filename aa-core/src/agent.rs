//! Agent execution context carrying identity, PID, and metadata.
//!
//! The primary type is [`AgentContext`], which flows through every governance
//! event in the system. Requires the `alloc` feature.

#[cfg(feature = "alloc")]
use alloc::{collections::BTreeMap, string::String};

#[cfg(feature = "alloc")]
use crate::{
    identity::{AgentId, SessionId},
    time::Timestamp,
    GovernanceLevel,
};

/// Identity carrier for an agent execution.
///
/// `AgentContext` flows through every governance event in the system.
/// It captures the stable agent identity, per-session identity, process ID,
/// start time, any additional runtime metadata, and optional topology/lineage
/// fields that describe the agent's position in a delegation hierarchy.
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
    ///
    /// Keys are owned `String` so the map is serde-compatible and accepts
    /// both string-literal keys and computed keys at runtime.
    pub metadata: BTreeMap<String, String>,
    /// Governance level (L0–L3) carried for level-conditional policy rules.
    ///
    /// Populated by the gateway from the agent's `AgentRecord` (defined in
    /// `aa-gateway`) at the boundary between transport and the policy
    /// engine. Defaults to [`GovernanceLevel::L0Discover`] so old serialised
    /// contexts — and callers that have not yet been updated — deserialise
    /// or construct without churn.
    #[cfg_attr(feature = "serde", serde(default))]
    pub governance_level: GovernanceLevel,
    /// The agent that spawned this one; `None` for root agents.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none", default))]
    pub parent_agent_id: Option<AgentId>,
    /// Team this agent belongs to; `None` if no team is assigned.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none", default))]
    pub team_id: Option<String>,
    /// Delegation depth — 0 for root agents, incremented by 1 per delegation level.
    #[cfg_attr(feature = "serde", serde(default))]
    pub depth: u32,
    /// Human-readable reason the parent delegated to this agent.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none", default))]
    pub delegation_reason: Option<String>,
    /// Tool or framework that triggered the spawn (e.g. `"langgraph.subgraph"`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none", default))]
    pub spawned_by_tool: Option<String>,
    /// Root of the delegation chain — the top-level agent that ultimately spawned this one.
    ///
    /// For root agents this equals `Some(agent_id)`.  For sub-agents it is set
    /// server-side to `parent.root_agent_id.unwrap_or(parent.agent_id)` so that
    /// any node in a delegation chain can resolve its root in O(1).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none", default))]
    pub root_agent_id: Option<AgentId>,
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
            governance_level: GovernanceLevel::default(),
            parent_agent_id: None,
            team_id: None,
            depth: 0,
            delegation_reason: None,
            spawned_by_tool: None,
            root_agent_id: None,
        }
    }

    /// Return a fresh [`AgentContextBuilder`] for topology-aware construction.
    pub fn builder() -> AgentContextBuilder {
        AgentContextBuilder::new()
    }
}

/// Fluent builder for [`AgentContext`] that allows populating optional topology
/// and lineage fields before stamping the context at construction time.
///
/// Obtain one via [`AgentContext::builder()`].
#[cfg(feature = "alloc")]
pub struct AgentContextBuilder {
    parent_agent_id: Option<AgentId>,
    team_id: Option<String>,
    depth: u32,
    delegation_reason: Option<String>,
    spawned_by_tool: Option<String>,
    root_agent_id: Option<AgentId>,
}

#[cfg(feature = "alloc")]
impl AgentContextBuilder {
    fn new() -> Self {
        Self {
            parent_agent_id: None,
            team_id: None,
            depth: 0,
            delegation_reason: None,
            spawned_by_tool: None,
            root_agent_id: None,
        }
    }

    /// Set the agent that spawned this one.
    pub fn parent_agent_id(mut self, id: AgentId) -> Self {
        self.parent_agent_id = Some(id);
        self
    }

    /// Set the team this agent belongs to.
    pub fn team_id(mut self, id: String) -> Self {
        self.team_id = Some(id);
        self
    }

    /// Set the delegation depth (0 = root).
    pub fn depth(mut self, d: u32) -> Self {
        self.depth = d;
        self
    }

    /// Set the human-readable reason the parent delegated to this agent.
    pub fn delegation_reason(mut self, r: String) -> Self {
        self.delegation_reason = Some(r);
        self
    }

    /// Set the tool or framework that triggered the spawn.
    pub fn spawned_by_tool(mut self, t: String) -> Self {
        self.spawned_by_tool = Some(t);
        self
    }

    /// Set the root agent of the delegation chain.
    pub fn root_agent_id(mut self, id: AgentId) -> Self {
        self.root_agent_id = Some(id);
        self
    }
}

#[cfg(all(feature = "alloc", feature = "std"))]
impl AgentContextBuilder {
    /// Consume the builder and construct an [`AgentContext`] stamped at the
    /// current wall-clock time. `metadata` is initialised empty.
    pub fn build(self, agent_id: AgentId, session_id: SessionId, pid: u32) -> AgentContext {
        AgentContext {
            started_at: Timestamp::from(std::time::SystemTime::now()),
            agent_id,
            session_id,
            pid,
            metadata: BTreeMap::new(),
            governance_level: GovernanceLevel::default(),
            parent_agent_id: self.parent_agent_id,
            team_id: self.team_id,
            depth: self.depth,
            delegation_reason: self.delegation_reason,
            spawned_by_tool: self.spawned_by_tool,
            root_agent_id: self.root_agent_id,
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
            governance_level: GovernanceLevel::default(),
            parent_agent_id: None,
            team_id: None,
            depth: 0,
            delegation_reason: None,
            spawned_by_tool: None,
            root_agent_id: None,
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

    #[cfg(feature = "std")]
    #[test]
    fn builder_defaults_give_root_agent() {
        let ctx = AgentContext::builder().build(
            AgentId::from_bytes(AGENT_BYTES),
            SessionId::from_bytes(SESSION_BYTES),
            42,
        );
        assert_eq!(ctx.depth, 0);
        assert!(ctx.parent_agent_id.is_none());
        assert!(ctx.team_id.is_none());
        assert!(ctx.delegation_reason.is_none());
        assert!(ctx.spawned_by_tool.is_none());
        assert!(ctx.root_agent_id.is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn builder_sets_parent_and_team() {
        let parent = AgentId::from_bytes([9u8; 16]);
        let ctx = AgentContext::builder()
            .parent_agent_id(parent)
            .team_id("team-alpha".into())
            .depth(1)
            .build(
                AgentId::from_bytes(AGENT_BYTES),
                SessionId::from_bytes(SESSION_BYTES),
                42,
            );
        assert_eq!(ctx.parent_agent_id, Some(parent));
        assert_eq!(ctx.team_id.as_deref(), Some("team-alpha"));
        assert_eq!(ctx.depth, 1);
    }

    #[cfg(feature = "std")]
    #[test]
    fn builder_sets_delegation_fields() {
        let ctx = AgentContext::builder()
            .delegation_reason("summarise results".into())
            .spawned_by_tool("langgraph.subgraph".into())
            .build(
                AgentId::from_bytes(AGENT_BYTES),
                SessionId::from_bytes(SESSION_BYTES),
                42,
            );
        assert_eq!(ctx.delegation_reason.as_deref(), Some("summarise results"));
        assert_eq!(ctx.spawned_by_tool.as_deref(), Some("langgraph.subgraph"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let original = make_context();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: AgentContext = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip_with_topology_fields() {
        let root = AgentId::from_bytes([7u8; 16]);
        let original = AgentContext {
            agent_id: AgentId::from_bytes(AGENT_BYTES),
            session_id: SessionId::from_bytes(SESSION_BYTES),
            pid: 42,
            started_at: Timestamp::from_nanos(1_000_000),
            metadata: BTreeMap::new(),
            governance_level: GovernanceLevel::default(),
            parent_agent_id: Some(AgentId::from_bytes([9u8; 16])),
            team_id: Some("team-alpha".into()),
            depth: 2,
            delegation_reason: Some("summarise results".into()),
            spawned_by_tool: Some("langgraph.subgraph".into()),
            root_agent_id: Some(root),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: AgentContext = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_backward_compat_missing_topology_fields() {
        // Contexts serialised before topology fields were added must still
        // deserialise — new fields must default to their zero values.
        let json = r#"{
            "agent_id":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
            "session_id":[2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2],
            "pid":42,
            "started_at":1000000,
            "metadata":{}
        }"#;
        let restored: AgentContext = serde_json::from_str(json).expect("deserialize");
        assert_eq!(restored.depth, 0);
        assert!(restored.parent_agent_id.is_none());
        assert!(restored.team_id.is_none());
        assert!(restored.delegation_reason.is_none());
        assert!(restored.spawned_by_tool.is_none());
        assert!(restored.root_agent_id.is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn agent_context_defaults_to_l0_when_governance_level_missing() {
        // Old serialised contexts written before `governance_level` was
        // added must still deserialise — the field must default to
        // `L0Discover`. This is the runtime-stable backward-compat
        // guarantee called out by AAASM-1041.
        let json = r#"{
            "agent_id":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
            "session_id":[2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2],
            "pid":42,
            "started_at":1000000,
            "metadata":{}
        }"#;
        let restored: AgentContext = serde_json::from_str(json).expect("deserialize");
        assert_eq!(restored.governance_level, GovernanceLevel::L0Discover);
    }
}
