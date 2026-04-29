//! `ApprovalService` tonic trait implementation wiring gRPC RPCs to `ApprovalQueue`.

use std::sync::Arc;

use aa_runtime::approval::ApprovalQueue;

/// gRPC service implementation wiring approval RPCs to [`ApprovalQueue`].
pub struct ApprovalServiceImpl {
    queue: Arc<ApprovalQueue>,
}

impl ApprovalServiceImpl {
    /// Create a new service backed by the given approval queue.
    pub fn new(queue: Arc<ApprovalQueue>) -> Self {
        Self { queue }
    }
}
