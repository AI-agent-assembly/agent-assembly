//! `AgentLifecycleService` tonic trait implementation wiring gRPC RPCs to [`AgentRegistry`].

use std::pin::Pin;
use std::sync::Arc;

use tonic::{Request, Response, Status};

use aa_proto::assembly::agent::v1::agent_lifecycle_service_server::AgentLifecycleService;
use aa_proto::assembly::agent::v1::{
    ControlCommand, ControlStreamRequest, DeregisterRequest, DeregisterResponse, HeartbeatRequest,
    HeartbeatResponse, RegisterRequest, RegisterResponse,
};

use crate::registry::AgentRegistry;

/// gRPC service implementation wiring `Register` / `Heartbeat` / `Deregister` /
/// `ControlStream` to the in-memory [`AgentRegistry`].
pub struct AgentLifecycleServiceImpl {
    #[allow(dead_code)] // will be used when RPC stubs are implemented
    registry: Arc<AgentRegistry>,
}

impl AgentLifecycleServiceImpl {
    /// Create a new service backed by the given agent registry.
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self { registry }
    }
}

type ControlStreamOutput =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<ControlCommand, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl AgentLifecycleService for AgentLifecycleServiceImpl {
    async fn register(
        &self,
        _request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        todo!("AAASM-136: implement Register RPC")
    }

    async fn heartbeat(
        &self,
        _request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        todo!("AAASM-136: implement Heartbeat RPC")
    }

    async fn deregister(
        &self,
        _request: Request<DeregisterRequest>,
    ) -> Result<Response<DeregisterResponse>, Status> {
        todo!("AAASM-136: implement Deregister RPC")
    }

    type ControlStreamStream = ControlStreamOutput;

    async fn control_stream(
        &self,
        _request: Request<ControlStreamRequest>,
    ) -> Result<Response<Self::ControlStreamStream>, Status> {
        todo!("AAASM-136: implement ControlStream RPC")
    }
}
