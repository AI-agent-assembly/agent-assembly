//! Integration tests for the PolicyService gRPC endpoint.
//!
//! Each test starts a tonic server on a random TCP port, connects a client,
//! sends requests, and asserts on responses.

use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;

use aa_gateway::service::PolicyServiceImpl;
use aa_gateway::PolicyEngine;
use aa_proto::assembly::common::v1::{ActionType, AgentId as ProtoAgentId, Decision};
use aa_proto::assembly::policy::v1::policy_service_client::PolicyServiceClient;
use aa_proto::assembly::policy::v1::policy_service_server::PolicyServiceServer;
use aa_proto::assembly::policy::v1::{
    action_context::Action, ActionContext, BatchCheckRequest, CheckActionRequest, ToolCallContext,
};
use tokio::net::TcpListener;
use tonic::transport::Server;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Start a PolicyService gRPC server on a random port and return the address.
async fn start_server(policy_yaml: &str) -> SocketAddr {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "{}", policy_yaml).unwrap();
    tmp.flush().unwrap();

    let engine = PolicyEngine::load_from_file(tmp.path()).unwrap();
    let service = PolicyServiceImpl::new(Arc::new(engine));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Keep the tempfile alive for the duration of the server.
    tokio::spawn(async move {
        let _tmp = tmp; // prevent drop
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        Server::builder()
            .add_service(PolicyServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    // Give the server a moment to start.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    addr
}

fn tool_call_request(tool_name: &str) -> CheckActionRequest {
    CheckActionRequest {
        agent_id: Some(ProtoAgentId {
            org_id: "org".into(),
            team_id: "team".into(),
            agent_id: "agent-1".into(),
        }),
        credential_token: "tok".into(),
        trace_id: "trace-1".into(),
        span_id: "span-1".into(),
        action_type: ActionType::ToolCall as i32,
        context: Some(ActionContext {
            action: Some(Action::ToolCall(ToolCallContext {
                tool_name: tool_name.into(),
                tool_source: "test".into(),
                args_json: b"{}".to_vec(),
                target_url: String::new(),
            })),
        }),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn check_action_allows_permitted_tool() {
    let addr = start_server(
        r#"
version: "1"
tools:
  web_search:
    allow: true
"#,
    )
    .await;

    let mut client = PolicyServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let resp = client
        .check_action(tool_call_request("web_search"))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(resp.decision, Decision::Allow as i32);
}

#[tokio::test]
async fn check_action_denies_blocked_tool() {
    let addr = start_server(
        r#"
version: "1"
tools:
  dangerous:
    allow: false
"#,
    )
    .await;

    let mut client = PolicyServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let resp = client
        .check_action(tool_call_request("dangerous"))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(resp.decision, Decision::Deny as i32);
    assert!(!resp.reason.is_empty());
}

#[tokio::test]
async fn check_action_returns_invalid_argument_on_missing_context() {
    let addr = start_server("version: \"1\"\n").await;

    let mut client = PolicyServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let bad_req = CheckActionRequest {
        agent_id: Some(ProtoAgentId {
            agent_id: "a".into(),
            ..Default::default()
        }),
        context: None,
        ..Default::default()
    };

    let err = client.check_action(bad_req).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn check_action_populates_latency_us() {
    let addr = start_server("version: \"1\"\n").await;

    let mut client = PolicyServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let resp = client
        .check_action(tool_call_request("any"))
        .await
        .unwrap()
        .into_inner();

    assert!(
        resp.decision_latency_us >= 0,
        "decision_latency_us should be non-negative"
    );
}

#[tokio::test]
async fn batch_check_returns_ordered_responses() {
    let addr = start_server(
        r#"
version: "1"
tools:
  allowed_tool:
    allow: true
  blocked_tool:
    allow: false
"#,
    )
    .await;

    let mut client = PolicyServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let batch = BatchCheckRequest {
        requests: vec![
            tool_call_request("allowed_tool"),
            tool_call_request("blocked_tool"),
            tool_call_request("allowed_tool"),
        ],
    };

    let resp = client.batch_check(batch).await.unwrap().into_inner();
    assert_eq!(resp.responses.len(), 3);
    assert_eq!(resp.responses[0].decision, Decision::Allow as i32);
    assert_eq!(resp.responses[1].decision, Decision::Deny as i32);
    assert_eq!(resp.responses[2].decision, Decision::Allow as i32);
}
