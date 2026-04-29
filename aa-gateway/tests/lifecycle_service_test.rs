//! Integration tests for the AgentLifecycleService gRPC endpoint.
//!
//! Starts a tonic server on a random TCP port, connects a client,
//! and exercises the full Register → Heartbeat → ControlStream → Deregister lifecycle.

use std::net::SocketAddr;
use std::sync::Arc;

use aa_gateway::registry::AgentRegistry;
use aa_gateway::service::AgentLifecycleServiceImpl;
use aa_proto::assembly::agent::v1::agent_lifecycle_service_client::AgentLifecycleServiceClient;
use aa_proto::assembly::agent::v1::agent_lifecycle_service_server::AgentLifecycleServiceServer;
use aa_proto::assembly::agent::v1::{ControlStreamRequest, DeregisterRequest, HeartbeatRequest, RegisterRequest};
use aa_proto::assembly::common::v1::AgentId as ProtoAgentId;
use tokio::net::TcpListener;
use tonic::transport::Server;

// ── Helpers ────────────────────────────────────────────────────────────────

/// Generate a hex-encoded Ed25519 public key for testing.
fn test_ed25519_public_key_hex() -> String {
    use ed25519_dalek::SigningKey;
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    hex::encode(signing_key.verifying_key().as_bytes())
}

fn test_agent_id() -> ProtoAgentId {
    ProtoAgentId {
        org_id: "org-test".into(),
        team_id: "team-test".into(),
        agent_id: "agent-lifecycle-1".into(),
    }
}

/// Start an AgentLifecycleService gRPC server and return the address + registry.
async fn start_server() -> (SocketAddr, Arc<AgentRegistry>) {
    let registry = Arc::new(AgentRegistry::new());
    let service = AgentLifecycleServiceImpl::new(Arc::clone(&registry));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let registry_clone = Arc::clone(&registry);
    tokio::spawn(async move {
        let _reg = registry_clone;
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        Server::builder()
            .add_service(AgentLifecycleServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (addr, registry)
}

// ── Full lifecycle test ────────────────────────────────────────────────────

#[tokio::test]
async fn full_lifecycle_register_heartbeat_control_stream_deregister() {
    let (addr, _registry) = start_server().await;
    let mut client = AgentLifecycleServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();

    let agent_id = test_agent_id();
    let public_key = test_ed25519_public_key_hex();

    // 1. Register
    let reg_resp = client
        .register(RegisterRequest {
            agent_id: Some(agent_id.clone()),
            name: "lifecycle-test-agent".into(),
            framework: "custom".into(),
            version: "1.0.0".into(),
            risk_tier: 0,
            tool_names: vec!["tool_a".into()],
            public_key: public_key.clone(),
            metadata: Default::default(),
        })
        .await
        .unwrap()
        .into_inner();

    let token = reg_resp.credential_token;
    assert!(!token.is_empty());
    assert_eq!(reg_resp.heartbeat_interval_sec, 30);

    // 2. Heartbeat
    let hb_resp = client
        .heartbeat(HeartbeatRequest {
            agent_id: Some(agent_id.clone()),
            credential_token: token.clone(),
            active_runs: 1,
            actions_count: 10,
        })
        .await
        .unwrap()
        .into_inner();

    assert!(!hb_resp.should_suspend);

    // 3. ControlStream — open a stream and verify it's alive
    let stream_resp = client
        .control_stream(ControlStreamRequest {
            agent_id: Some(agent_id.clone()),
            credential_token: token.clone(),
        })
        .await;
    assert!(stream_resp.is_ok());

    // 4. Deregister
    let dereg_resp = client
        .deregister(DeregisterRequest {
            agent_id: Some(agent_id.clone()),
            credential_token: token,
            reason: "test cleanup".into(),
        })
        .await
        .unwrap()
        .into_inner();

    assert!(dereg_resp.success);
}

// ── Error case tests ───────────────────────────────────────────────────────

#[tokio::test]
async fn register_with_invalid_public_key_returns_error() {
    let (addr, _registry) = start_server().await;
    let mut client = AgentLifecycleServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();

    let status = client
        .register(RegisterRequest {
            agent_id: Some(test_agent_id()),
            name: "bad-key-agent".into(),
            framework: "custom".into(),
            version: "1.0.0".into(),
            risk_tier: 0,
            tool_names: vec![],
            public_key: "not_valid_hex_key".into(),
            metadata: Default::default(),
        })
        .await
        .unwrap_err();

    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn heartbeat_with_wrong_token_returns_unauthenticated() {
    let (addr, _registry) = start_server().await;
    let mut client = AgentLifecycleServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();

    let agent_id = test_agent_id();

    // Register first
    client
        .register(RegisterRequest {
            agent_id: Some(agent_id.clone()),
            name: "auth-test-agent".into(),
            framework: "custom".into(),
            version: "1.0.0".into(),
            risk_tier: 0,
            tool_names: vec![],
            public_key: test_ed25519_public_key_hex(),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    // Heartbeat with wrong token
    let status = client
        .heartbeat(HeartbeatRequest {
            agent_id: Some(agent_id),
            credential_token: "wrong-token".into(),
            active_runs: 0,
            actions_count: 0,
        })
        .await
        .unwrap_err();

    assert_eq!(status.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn deregister_unregistered_agent_returns_not_found() {
    let (addr, _registry) = start_server().await;
    let mut client = AgentLifecycleServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();

    let status = client
        .deregister(DeregisterRequest {
            agent_id: Some(test_agent_id()),
            credential_token: "any-token".into(),
            reason: "test".into(),
        })
        .await
        .unwrap_err();

    assert_eq!(status.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn duplicate_register_returns_already_exists() {
    let (addr, _registry) = start_server().await;
    let mut client = AgentLifecycleServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();

    let req = RegisterRequest {
        agent_id: Some(test_agent_id()),
        name: "dup-agent".into(),
        framework: "custom".into(),
        version: "1.0.0".into(),
        risk_tier: 0,
        tool_names: vec![],
        public_key: test_ed25519_public_key_hex(),
        metadata: Default::default(),
    };

    client.register(req.clone()).await.unwrap();

    let status = client.register(req).await.unwrap_err();
    assert_eq!(status.code(), tonic::Code::AlreadyExists);
}

// ── Heartbeat suspend signaling ──────────────────────────────────────────

#[tokio::test]
async fn heartbeat_returns_should_suspend_true_for_suspended_agent() {
    use aa_gateway::registry::SuspendReason;

    let (addr, registry) = start_server().await;
    let mut client = AgentLifecycleServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();

    let agent_id = test_agent_id();
    let public_key = test_ed25519_public_key_hex();

    let reg_resp = client
        .register(RegisterRequest {
            agent_id: Some(agent_id.clone()),
            name: "suspend-test-agent".into(),
            framework: "custom".into(),
            version: "1.0.0".into(),
            risk_tier: 0,
            tool_names: vec![],
            public_key,
            metadata: Default::default(),
        })
        .await
        .unwrap()
        .into_inner();

    let token = reg_resp.credential_token;

    // Suspend the agent directly via the registry
    use aa_gateway::registry::convert::proto_agent_id_to_key;
    let agent_key = proto_agent_id_to_key(&agent_id);
    registry
        .suspend_agent(&agent_key, SuspendReason::BudgetExceeded)
        .unwrap();

    // Heartbeat should return should_suspend = true
    let hb_resp = client
        .heartbeat(HeartbeatRequest {
            agent_id: Some(agent_id),
            credential_token: token,
            active_runs: 0,
            actions_count: 0,
        })
        .await
        .unwrap()
        .into_inner();

    assert!(hb_resp.should_suspend);
}

#[tokio::test]
async fn heartbeat_returns_should_suspend_false_for_active_agent() {
    let (addr, _registry) = start_server().await;
    let mut client = AgentLifecycleServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();

    let agent_id = test_agent_id();
    let public_key = test_ed25519_public_key_hex();

    let reg_resp = client
        .register(RegisterRequest {
            agent_id: Some(agent_id.clone()),
            name: "active-test-agent".into(),
            framework: "custom".into(),
            version: "1.0.0".into(),
            risk_tier: 0,
            tool_names: vec![],
            public_key,
            metadata: Default::default(),
        })
        .await
        .unwrap()
        .into_inner();

    let token = reg_resp.credential_token;

    let hb_resp = client
        .heartbeat(HeartbeatRequest {
            agent_id: Some(agent_id),
            credential_token: token,
            active_runs: 0,
            actions_count: 0,
        })
        .await
        .unwrap()
        .into_inner();

    assert!(!hb_resp.should_suspend);
}
