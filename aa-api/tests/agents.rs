//! Integration tests for the agent endpoints.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::collections::BTreeMap;
use tower::ServiceExt;

use aa_gateway::registry::{AgentRecord, AgentStatus};

/// Build a test `AgentRecord` with a known 16-byte ID.
fn test_agent(id_byte: u8) -> AgentRecord {
    AgentRecord {
        agent_id: [id_byte; 16],
        name: format!("test-agent-{id_byte}"),
        framework: "langgraph".to_string(),
        version: "0.1.0".to_string(),
        risk_tier: 1,
        tool_names: vec!["read_file".to_string(), "write_file".to_string()],
        public_key: "test-pubkey".to_string(),
        credential_token: "test-token".to_string(),
        metadata: BTreeMap::new(),
        registered_at: chrono::Utc::now(),
        last_heartbeat: chrono::Utc::now(),
        status: AgentStatus::Active,
        pid: None,
        session_count: 0,
        last_event: None,
        policy_violations_count: 0,
    }
}

/// Convert a single-byte ID to the 32-char hex string the API expects.
fn hex_id(id_byte: u8) -> String {
    format!("{id_byte:02x}").repeat(16)
}

#[tokio::test]
async fn list_agents_returns_200_empty() {
    let app = common::test_app();

    let response = app
        .oneshot(Request::builder().uri("/api/v1/agents").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 0);
    assert!(json["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_agents_returns_registered_agents() {
    let state = common::test_state();
    state.agent_registry.register(test_agent(0xAA)).unwrap();
    state.agent_registry.register(test_agent(0xBB)).unwrap();

    let app = aa_api::server::build_app(state);

    let response = app
        .oneshot(Request::builder().uri("/api/v1/agents").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 2);

    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    // Each agent should have expected fields
    for item in items {
        assert!(item["id"].as_str().is_some());
        assert!(item["name"].as_str().is_some());
        assert_eq!(item["framework"], "langgraph");
        assert_eq!(item["status"], "Active");
    }
}

#[tokio::test]
async fn get_agent_returns_200_for_registered_agent() {
    let state = common::test_state();
    state.agent_registry.register(test_agent(0xAA)).unwrap();

    let app = aa_api::server::build_app(state);
    let id = hex_id(0xAA);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/agents/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "test-agent-170");
    assert_eq!(json["framework"], "langgraph");
    assert_eq!(json["version"], "0.1.0");
    assert_eq!(json["status"], "Active");
    assert_eq!(json["tool_names"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_agent_returns_404_for_unknown_id() {
    let app = common::test_app();
    let id = hex_id(0xFF);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/agents/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_agent_returns_400_for_invalid_id() {
    let app = common::test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents/not-a-hex-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_agent_returns_204_for_registered_agent() {
    let state = common::test_state();
    state.agent_registry.register(test_agent(0xCC)).unwrap();

    let app = aa_api::server::build_app(state);
    let id = hex_id(0xCC);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/agents/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn delete_agent_returns_404_for_unknown_id() {
    let app = common::test_app();
    let id = hex_id(0xFF);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/agents/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_agent_returns_400_for_invalid_id() {
    let app = common::test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/agents/bad-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_agents_pagination_works() {
    let state = common::test_state();
    for i in 0u8..5 {
        state.agent_registry.register(test_agent(i)).unwrap();
    }

    let app = aa_api::server::build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents?page=1&per_page=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 5);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_agent_response_includes_new_fields() {
    let state = common::test_state();
    let mut agent = test_agent(0xDD);
    agent.pid = Some(9876);
    agent.session_count = 7;
    agent.last_event = Some(chrono::Utc::now());
    agent.policy_violations_count = 2;
    state.agent_registry.register(agent).unwrap();

    let app = aa_api::server::build_app(state);
    let id = hex_id(0xDD);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/agents/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["pid"], 9876);
    assert_eq!(json["session_count"], 7);
    assert!(json["last_event"].as_str().is_some());
    assert_eq!(json["policy_violations_count"], 2);
}

#[tokio::test]
async fn get_agent_response_null_optional_fields() {
    let state = common::test_state();
    state.agent_registry.register(test_agent(0xEE)).unwrap();

    let app = aa_api::server::build_app(state);
    let id = hex_id(0xEE);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/agents/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["pid"].is_null());
    assert_eq!(json["session_count"], 0);
    assert!(json["last_event"].is_null());
    assert_eq!(json["policy_violations_count"], 0);
}
