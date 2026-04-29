//! Integration tests for the ApprovalService gRPC endpoint.
//!
//! Starts a tonic server on a random TCP port, connects a client,
//! and exercises ListPending, Decide, and WatchApprovals RPCs.

use std::net::SocketAddr;
use std::sync::Arc;

use aa_proto::assembly::approval::v1::approval_service_client::ApprovalServiceClient;
use aa_proto::assembly::approval::v1::approval_service_server::ApprovalServiceServer;
use aa_proto::assembly::approval::v1::{
    ApprovalDecisionType, DecideRequest, ListPendingRequest, WatchApprovalsRequest,
};
use aa_gateway::service::ApprovalServiceImpl;
use aa_runtime::approval::{ApprovalQueue, ApprovalRequest};
use tokio::net::TcpListener;
use tonic::transport::Server;
use uuid::Uuid;

/// Start an ApprovalService gRPC server and return the address + queue handle.
async fn start_server() -> (SocketAddr, Arc<ApprovalQueue>) {
    let queue = ApprovalQueue::new();
    let service = ApprovalServiceImpl::new(Arc::clone(&queue));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        Server::builder()
            .add_service(ApprovalServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (addr, queue)
}

fn make_test_request() -> ApprovalRequest {
    ApprovalRequest {
        request_id: Uuid::new_v4(),
        agent_id: "agent-test".to_string(),
        action: "deploy to production".to_string(),
        condition_triggered: "requires-tech-lead-approval".to_string(),
        submitted_at: 1_700_000_000,
        timeout_secs: 300,
        fallback: aa_core::PolicyResult::Deny {
            reason: "timed out".to_string(),
        },
    }
}

#[tokio::test]
async fn list_pending_returns_empty_initially() {
    let (addr, _queue) = start_server().await;
    let mut client = ApprovalServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();

    let resp = client
        .list_pending(ListPendingRequest {})
        .await
        .unwrap()
        .into_inner();

    assert!(resp.requests.is_empty());
}
