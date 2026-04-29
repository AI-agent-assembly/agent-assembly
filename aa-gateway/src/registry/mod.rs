//! Agent registry — in-memory agent identity store and lifecycle tracking.
//!
//! This module maintains the set of registered agents, their identity records,
//! credential tokens, and heartbeat state. It is the server-side backing store
//! for the `AgentLifecycleService` gRPC service defined in `proto/agent.proto`.

pub mod store;
pub mod token;

pub use store::{AgentRecord, AgentRegistry};

/// Errors returned by [`AgentRegistry`](store::AgentRegistry) operations.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// Attempted to register an agent whose ID is already present.
    #[error("agent already registered: {0:?}")]
    AlreadyRegistered([u8; 16]),
    /// Referenced an agent ID that does not exist in the registry.
    #[error("agent not found: {0:?}")]
    NotFound([u8; 16]),
}

/// Runtime status of a registered agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is actively running and sending heartbeats.
    Active,
    /// Agent has been suspended by the gateway (e.g. budget exceeded, manual pause).
    Suspended,
    /// Agent has been removed from the registry (clean shutdown or forced removal).
    Deregistered,
}
