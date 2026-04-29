//! Agent registry store — `AgentRecord` and `AgentRegistry` backed by `DashMap`.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;

use super::{AgentStatus, RegistryError};

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

    /// Insert a new agent record. Returns an error if the ID is already registered.
    pub fn register(&self, record: AgentRecord) -> Result<(), RegistryError> {
        use dashmap::mapref::entry::Entry;
        match self.agents.entry(record.agent_id) {
            Entry::Occupied(_) => Err(RegistryError::AlreadyRegistered(record.agent_id)),
            Entry::Vacant(v) => {
                v.insert(record);
                Ok(())
            }
        }
    }

    /// Look up an agent by ID. Returns `None` if not found.
    pub fn get(&self, agent_id: &[u8; 16]) -> Option<AgentRecord> {
        self.agents.get(agent_id).map(|r| r.clone())
    }

    /// Remove an agent from the registry. Returns the removed record.
    pub fn deregister(&self, agent_id: &[u8; 16]) -> Result<AgentRecord, RegistryError> {
        self.agents
            .remove(agent_id)
            .map(|(_, record)| record)
            .ok_or(RegistryError::NotFound(*agent_id))
    }

    /// Update the `last_heartbeat` timestamp for an agent to now.
    pub fn update_heartbeat(&self, agent_id: &[u8; 16]) -> Result<(), RegistryError> {
        let mut entry = self
            .agents
            .get_mut(agent_id)
            .ok_or(RegistryError::NotFound(*agent_id))?;
        entry.last_heartbeat = Utc::now();
        Ok(())
    }

    /// Return a snapshot of all currently registered agents.
    pub fn list(&self) -> Vec<AgentRecord> {
        self.agents.iter().map(|r| r.value().clone()).collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
