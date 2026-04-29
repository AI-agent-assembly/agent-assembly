//! `AuditService` tonic trait implementation wiring gRPC RPCs to [`AuditWriter`].
//!
//! [`AuditWriter`]: crate::audit::AuditWriter

use tonic::{Request, Response, Status};

use aa_proto::assembly::audit::v1::audit_service_server::AuditService;
use aa_proto::assembly::audit::v1::{
    AuditEvent, ReportEventsRequest, ReportEventsResponse, StreamEventsResponse,
};

/// gRPC service implementation wiring `ReportEvents` / `StreamEvents` to the
/// audit writer channel.
pub struct AuditServiceImpl;

#[tonic::async_trait]
impl AuditService for AuditServiceImpl {
    async fn report_events(
        &self,
        _request: Request<ReportEventsRequest>,
    ) -> Result<Response<ReportEventsResponse>, Status> {
        Err(Status::unimplemented("ReportEvents not yet implemented"))
    }

    async fn stream_events(
        &self,
        _request: Request<tonic::Streaming<AuditEvent>>,
    ) -> Result<Response<StreamEventsResponse>, Status> {
        Err(Status::unimplemented("StreamEvents not yet implemented"))
    }
}
