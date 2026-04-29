//! `AgentLifecycleService` tonic trait implementation wiring gRPC RPCs to [`AgentRegistry`].

use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;

use chrono::Utc;
use tonic::{Request, Response, Status};

use aa_proto::assembly::agent::v1::agent_lifecycle_service_server::AgentLifecycleService;
use aa_proto::assembly::agent::v1::{
    ControlCommand, ControlStreamRequest, DeregisterRequest, DeregisterResponse, HeartbeatRequest, HeartbeatResponse,
    RegisterRequest, RegisterResponse,
};

use crate::registry::convert::{proto_agent_id_to_key, validate_proto_agent_id};
use crate::registry::store::AgentRecord;
use crate::registry::token::{generate_credential_token, validate_token};
use crate::registry::{AgentRegistry, AgentStatus};

/// Default heartbeat interval returned to agents at registration (seconds).
const DEFAULT_HEARTBEAT_INTERVAL_SEC: i64 = 30;

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

type ControlStreamOutput = Pin<Box<dyn tokio_stream::Stream<Item = Result<ControlCommand, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl AgentLifecycleService for AgentLifecycleServiceImpl {
    async fn register(&self, request: Request<RegisterRequest>) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();

        let proto_id = req.agent_id.as_ref().ok_or_else(|| Status::invalid_argument("missing agent_id"))?;
        validate_proto_agent_id(proto_id).map_err(|e| Status::invalid_argument(e.to_string()))?;

        if req.public_key.is_empty() {
            return Err(Status::invalid_argument("missing public_key"));
        }

        // Validate that public_key is a valid Ed25519 public key (32 bytes, hex-encoded).
        let pk_bytes = hex::decode(&req.public_key)
            .map_err(|_| Status::invalid_argument("public_key is not valid hex"))?;
        ed25519_dalek::VerifyingKey::from_bytes(
            pk_bytes
                .as_slice()
                .try_into()
                .map_err(|_| Status::invalid_argument("public_key must be 32 bytes (64 hex chars)"))?,
        )
        .map_err(|_| Status::invalid_argument("invalid Ed25519 public key"))?;

        let agent_key = proto_agent_id_to_key(proto_id);
        let credential_token = generate_credential_token();
        let now = Utc::now();

        let record = AgentRecord {
            agent_id: agent_key,
            name: req.name,
            framework: req.framework,
            version: req.version,
            risk_tier: req.risk_tier,
            tool_names: req.tool_names,
            public_key: req.public_key,
            credential_token: credential_token.clone(),
            metadata: BTreeMap::from_iter(req.metadata),
            registered_at: now,
            last_heartbeat: now,
            status: AgentStatus::Active,
        };

        self.registry
            .register(record)
            .map_err(|e| Status::already_exists(e.to_string()))?;

        tracing::info!(agent_id = ?proto_id.agent_id, "agent registered");

        Ok(Response::new(RegisterResponse {
            credential_token,
            assigned_policy: String::new(),
            heartbeat_interval_sec: DEFAULT_HEARTBEAT_INTERVAL_SEC,
        }))
    }

    async fn heartbeat(&self, request: Request<HeartbeatRequest>) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();

        let proto_id = req.agent_id.as_ref().ok_or_else(|| Status::invalid_argument("missing agent_id"))?;
        let agent_key = proto_agent_id_to_key(proto_id);

        validate_token(&self.registry, &agent_key, &req.credential_token)
            .map_err(|_| Status::unauthenticated("invalid credential token"))?;

        self.registry
            .update_heartbeat(&agent_key)
            .map_err(|e| Status::not_found(e.to_string()))?;

        tracing::debug!(agent_id = ?proto_id.agent_id, "heartbeat received");

        Ok(Response::new(HeartbeatResponse {
            policy_updated: false,
            should_suspend: false,
        }))
    }

    async fn deregister(&self, request: Request<DeregisterRequest>) -> Result<Response<DeregisterResponse>, Status> {
        let req = request.into_inner();

        let proto_id = req.agent_id.as_ref().ok_or_else(|| Status::invalid_argument("missing agent_id"))?;
        let agent_key = proto_agent_id_to_key(proto_id);

        validate_token(&self.registry, &agent_key, &req.credential_token)
            .map_err(|_| Status::unauthenticated("invalid credential token"))?;

        self.registry
            .deregister(&agent_key)
            .map_err(|e| Status::not_found(e.to_string()))?;

        tracing::info!(agent_id = ?proto_id.agent_id, reason = %req.reason, "agent deregistered");

        Ok(Response::new(DeregisterResponse {
            success: true,
            agent_id: proto_id.agent_id.clone(),
        }))
    }

    type ControlStreamStream = ControlStreamOutput;

    async fn control_stream(
        &self,
        _request: Request<ControlStreamRequest>,
    ) -> Result<Response<Self::ControlStreamStream>, Status> {
        todo!("AAASM-136: implement ControlStream RPC")
    }
}
