//! Agent registry — in-memory agent identity store and lifecycle tracking.
//!
//! This module maintains the set of registered agents, their identity records,
//! credential tokens, and heartbeat state. It is the server-side backing store
//! for the `AgentLifecycleService` gRPC service defined in `proto/agent.proto`.

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
