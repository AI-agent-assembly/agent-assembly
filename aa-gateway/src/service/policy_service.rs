//! `PolicyService` tonic trait implementation wiring gRPC RPCs to `PolicyEngine`.

use std::sync::Arc;

use crate::PolicyEngine;

/// gRPC service implementation wiring `CheckAction` / `BatchCheck` to [`PolicyEngine`].
pub struct PolicyServiceImpl {
    #[allow(dead_code)]
    engine: Arc<PolicyEngine>,
}

impl PolicyServiceImpl {
    /// Create a new service backed by the given policy engine.
    pub fn new(engine: Arc<PolicyEngine>) -> Self {
        Self { engine }
    }
}
