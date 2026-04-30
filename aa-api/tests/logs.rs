//! Integration tests for the audit log endpoint.

mod common;

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use aa_core::audit::{AuditEntry, AuditEventType};
use aa_core::{AgentId, SessionId};
use aa_gateway::AuditReader;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

const AGENT_BYTES: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
const SESSION_BYTES: [u8; 16] = [17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32];
const GENESIS_HASH: [u8; 32] = [0u8; 32];

/// Counter for unique temp directories.
static DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a test state with a custom audit directory.
fn test_app_with_audit_dir(audit_dir: &std::path::Path) -> axum::Router {
    let mut state = common::test_state();
    state.audit_reader = Arc::new(AuditReader::new(audit_dir.to_path_buf()));
    aa_api::server::build_app(state)
}

/// Write JSONL audit entries to a file in the given directory.
fn write_entries_to_dir(dir: &std::path::Path, entries: &[AuditEntry]) {
    let filename = format!("{}-{}.jsonl", hex::encode(AGENT_BYTES), hex::encode(SESSION_BYTES));
    let path = dir.join(filename);
    let mut contents = String::new();
    for entry in entries {
        contents.push_str(&serde_json::to_string(entry).unwrap());
        contents.push('\n');
    }
    std::fs::write(path, contents).unwrap();
}

/// Build a chain of N audit entries.
fn make_entry_chain(n: usize, event_type: AuditEventType) -> Vec<AuditEntry> {
    let mut entries = Vec::with_capacity(n);
    let mut prev_hash = GENESIS_HASH;
    for i in 0..n {
        let entry = AuditEntry::new(
            i as u64,
            1_714_222_134_000_000_000 + (i as u64 * 1_000_000_000),
            event_type,
            AgentId::from_bytes(AGENT_BYTES),
            SessionId::from_bytes(SESSION_BYTES),
            format!("{{\"seq\":{i}}}"),
            prev_hash,
        );
        prev_hash = *entry.entry_hash();
        entries.push(entry);
    }
    entries
}

fn unique_audit_dir() -> std::path::PathBuf {
    let id = DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("aa-logs-test-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn list_logs_returns_200_empty() {
    let app = common::test_app();

    let response = app
        .oneshot(Request::builder().uri("/api/v1/logs").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 0);
    assert!(json["items"].as_array().unwrap().is_empty());
    assert_eq!(json["page"], 1);
}

#[tokio::test]
async fn list_logs_respects_pagination_params() {
    let app = common::test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/logs?page=2&per_page=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["page"], 2);
    assert_eq!(json["per_page"], 10);
}

#[tokio::test]
async fn list_logs_returns_entries_after_write() {
    let dir = unique_audit_dir();
    let entries = make_entry_chain(3, AuditEventType::ToolCallIntercepted);
    write_entries_to_dir(&dir, &entries);

    let app = test_app_with_audit_dir(&dir);

    let response = app
        .oneshot(Request::builder().uri("/api/v1/logs").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 3);

    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);

    // Entries should be in reverse chronological order (newest first).
    let first_seq = items[0]["seq"].as_u64().unwrap();
    let last_seq = items[2]["seq"].as_u64().unwrap();
    assert!(first_seq > last_seq);

    // Verify fields are present.
    assert!(items[0]["timestamp"].as_str().unwrap().contains("2024-"));
    assert_eq!(items[0]["event_type"], "ToolCallIntercepted");
    assert_eq!(items[0]["agent_id"], hex::encode(AGENT_BYTES));
    assert_eq!(items[0]["session_id"], hex::encode(SESSION_BYTES));
}

#[tokio::test]
async fn list_logs_pagination_works() {
    let dir = unique_audit_dir();
    let entries = make_entry_chain(5, AuditEventType::PolicyViolation);
    write_entries_to_dir(&dir, &entries);

    let app = test_app_with_audit_dir(&dir);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/logs?page=1&per_page=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 5);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn list_logs_filters_by_agent_id() {
    let dir = unique_audit_dir();

    // Write entries for the standard agent.
    let entries = make_entry_chain(3, AuditEventType::ToolCallIntercepted);
    write_entries_to_dir(&dir, &entries);

    // Write entries for a different agent in a separate file.
    let other_agent: [u8; 16] = [99; 16];
    let other_session: [u8; 16] = [88; 16];
    let other_entry = AuditEntry::new(
        0,
        1_714_222_134_000_000_000,
        AuditEventType::PolicyViolation,
        AgentId::from_bytes(other_agent),
        SessionId::from_bytes(other_session),
        String::from("{}"),
        GENESIS_HASH,
    );
    let other_file = dir.join(format!(
        "{}-{}.jsonl",
        hex::encode(other_agent),
        hex::encode(other_session)
    ));
    std::fs::write(
        other_file,
        format!("{}\n", serde_json::to_string(&other_entry).unwrap()),
    )
    .unwrap();

    let app = test_app_with_audit_dir(&dir);

    // Filter by the standard agent — should return 3 entries.
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/logs?agent_id={}", hex::encode(AGENT_BYTES)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 3);
}
