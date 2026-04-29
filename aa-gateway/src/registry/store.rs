//! Agent registry store — `AgentRecord` and `AgentRegistry` backed by `DashMap`.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;

use super::AgentStatus;

/// Identity and runtime state record for a single registered agent.
#[derive(Debug, Clone)]
pub struct AgentRecord {
    /// Raw 16-byte UUID identifying this agent.
    pub agent_id: [u8; 16],
    /// Human-readable agent name.
    pub name: String,
    /// Agent framework (e.g. "langgraph", "crewai", "custom").
    pub framework: String,
    /// Semver version of the agent process.
    pub version: String,
    /// Risk tier as the proto enum integer value.
    pub risk_tier: i32,
    /// Tools the agent declared at registration.
    pub tool_names: Vec<String>,
    /// Ed25519 public key (base64 or hex encoded).
    pub public_key: String,
    /// Short-lived credential token issued at registration.
    pub credential_token: String,
    /// Arbitrary key-value metadata (team, owner, environment, etc.).
    pub metadata: BTreeMap<String, String>,
    /// Timestamp when the agent was registered.
    pub registered_at: DateTime<Utc>,
    /// Timestamp of the most recent heartbeat.
    pub last_heartbeat: DateTime<Utc>,
    /// Current runtime status of the agent.
    pub status: AgentStatus,
}

/// Thread-safe in-memory agent registry backed by [`DashMap`].
///
/// Keyed by the raw 16-byte `agent_id` UUID. Concurrent reads and writes
/// are safe without external locking.
pub struct AgentRegistry {
    agents: DashMap<[u8; 16], AgentRecord>,
}

impl AgentRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
        }
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
