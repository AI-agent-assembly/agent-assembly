//! Agent registry store — `AgentRecord` and `AgentRegistry` backed by `DashMap`.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tokio::sync::mpsc;
use tonic::Status;

use aa_proto::assembly::agent::v1::control_command::Command;
use aa_proto::assembly::agent::v1::{ControlCommand, SuspendCommand};

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

/// Channel sender type for pushing [`ControlCommand`]s to an agent's control stream.
pub type ControlSender = mpsc::Sender<Result<ControlCommand, Status>>;

/// Channel receiver type returned to the gRPC `ControlStream` response.
pub type ControlReceiver = mpsc::Receiver<Result<ControlCommand, Status>>;

/// Thread-safe in-memory agent registry backed by [`DashMap`].
///
/// Keyed by the raw 16-byte `agent_id` UUID. Concurrent reads and writes
/// are safe without external locking.
pub struct AgentRegistry {
    agents: DashMap<[u8; 16], AgentRecord>,
    /// Per-agent control stream senders. Created when an agent opens a `ControlStream`.
    control_senders: DashMap<[u8; 16], ControlSender>,
}

impl AgentRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
            control_senders: DashMap::new(),
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
    ///
    /// Also removes any associated control stream sender.
    pub fn deregister(&self, agent_id: &[u8; 16]) -> Result<AgentRecord, RegistryError> {
        self.control_senders.remove(agent_id);
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

    /// Open a control stream for a registered agent.
    ///
    /// Creates an `mpsc` channel, stores the sender side in the registry,
    /// and returns the receiver to be used as the gRPC response stream.
    /// Returns an error if the agent is not registered.
    pub fn open_control_stream(&self, agent_id: &[u8; 16]) -> Result<ControlReceiver, RegistryError> {
        if !self.agents.contains_key(agent_id) {
            return Err(RegistryError::NotFound(*agent_id));
        }
        let (tx, rx) = mpsc::channel(32);
        self.control_senders.insert(*agent_id, tx);
        Ok(rx)
    }

    /// Send a [`ControlCommand`] to an agent's open control stream.
    ///
    /// Returns an error if the agent has no active control stream.
    pub async fn send_command(&self, agent_id: &[u8; 16], cmd: ControlCommand) -> Result<(), RegistryError> {
        let sender = self
            .control_senders
            .get(agent_id)
            .ok_or(RegistryError::NotFound(*agent_id))?;
        sender
            .send(Ok(cmd))
            .await
            .map_err(|_| RegistryError::NotFound(*agent_id))
    }

    /// Return a snapshot of all currently registered agents.
    pub fn list(&self) -> Vec<AgentRecord> {
        self.agents.iter().map(|r| r.value().clone()).collect()
    }

    /// Suspend an agent with the given reason.
    pub fn suspend_agent(&self, agent_id: &[u8; 16], reason: super::SuspendReason) -> Result<(), RegistryError> {
        let mut entry = self
            .agents
            .get_mut(agent_id)
            .ok_or(RegistryError::NotFound(*agent_id))?;
        entry.status = AgentStatus::Suspended(reason);
        Ok(())
    }

    /// Suspend an agent and send a [`SuspendCommand`] via the control stream.
    ///
    /// Sets the agent status to `Suspended(reason)` and, if a control stream
    /// is open, pushes a `SuspendCommand` with the given reason string.
    /// The control stream send is best-effort: if the stream is closed or full,
    /// the suspension still takes effect.
    pub async fn suspend_and_notify(
        &self,
        agent_id: &[u8; 16],
        reason: super::SuspendReason,
        reason_text: &str,
    ) -> Result<(), RegistryError> {
        self.suspend_agent(agent_id, reason)?;

        let cmd = ControlCommand {
            command: Some(Command::Suspend(SuspendCommand {
                reason: reason_text.to_string(),
            })),
        };
        // Best-effort: ignore errors if the stream is not open.
        let _ = self.send_command(agent_id, cmd).await;
        Ok(())
    }

    /// Resume a suspended agent back to Active status.
    pub fn resume_agent(&self, agent_id: &[u8; 16]) -> Result<(), RegistryError> {
        let mut entry = self
            .agents
            .get_mut(agent_id)
            .ok_or(RegistryError::NotFound(*agent_id))?;
        entry.status = AgentStatus::Active;
        Ok(())
    }

    /// Query the current status of an agent.
    pub fn agent_status(&self, agent_id: &[u8; 16]) -> Result<AgentStatus, RegistryError> {
        self.agents
            .get(agent_id)
            .map(|r| r.status)
            .ok_or(RegistryError::NotFound(*agent_id))
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
