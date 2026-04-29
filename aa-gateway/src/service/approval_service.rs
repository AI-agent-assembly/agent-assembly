//! `ApprovalService` tonic trait implementation wiring gRPC RPCs to `ApprovalQueue`.

use std::pin::Pin;
use std::sync::Arc;

use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use aa_proto::assembly::approval::v1::approval_service_server::ApprovalService;
use aa_proto::assembly::approval::v1::{
    ApprovalEvent, DecideRequest, DecideResponse, ListPendingRequest, ListPendingResponse,
    WatchApprovalsRequest,
};
use aa_runtime::approval::ApprovalQueue;

use crate::service::convert;

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

#[tonic::async_trait]
impl ApprovalService for ApprovalServiceImpl {
    type WatchApprovalsStream =
        Pin<Box<dyn Stream<Item = Result<ApprovalEvent, Status>> + Send + 'static>>;

    async fn list_pending(
        &self,
        _request: Request<ListPendingRequest>,
    ) -> Result<Response<ListPendingResponse>, Status> {
        let pending = self.queue.list();
        let requests = pending.iter().map(convert::pending_to_proto).collect();
        Ok(Response::new(ListPendingResponse { requests }))
    }

    async fn decide(
        &self,
        _request: Request<DecideRequest>,
    ) -> Result<Response<DecideResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn watch_approvals(
        &self,
        _request: Request<WatchApprovalsRequest>,
    ) -> Result<Response<Self::WatchApprovalsStream>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }
}
