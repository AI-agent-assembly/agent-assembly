//! `AgentLifecycleService` tonic trait implementation wiring gRPC RPCs to [`AgentRegistry`].

use std::sync::Arc;

use crate::registry::AgentRegistry;

/// gRPC service implementation wiring `Register` / `Heartbeat` / `Deregister` /
/// `ControlStream` to the in-memory [`AgentRegistry`].
pub struct AgentLifecycleServiceImpl {
    registry: Arc<AgentRegistry>,
}

impl AgentLifecycleServiceImpl {
    /// Create a new service backed by the given agent registry.
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self { registry }
    }
}
