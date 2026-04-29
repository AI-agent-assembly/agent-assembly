//! Integration tests for the WebSocket event streaming endpoint.

mod common;

use std::sync::atomic::Ordering;

use aa_api::models::{EventType, GovernanceEvent};
use aa_runtime::pipeline::event::{EnrichedEvent, EventSource, PipelineEvent};
use tokio::net::TcpListener;

struct TestHandle {
    state: aa_api::state::AppState,
    _server: tokio::task::JoinHandle<()>,
}

/// Start the server on a random port and return the base URL.
async fn start_server() -> (String, TestHandle) {
    let state = common::test_state();
    let app = aa_api::server::build_app(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let url = format!("ws://127.0.0.1:{}", addr.port());
    (
        url,
        TestHandle {
            state,
            _server: handle,
        },
    )
}

#[tokio::test]
async fn ws_upgrade_succeeds() {
    let (url, _handle) = start_server().await;
    let ws_url = format!("{url}/api/v1/ws/events");
    let (ws, response) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
    assert_eq!(response.status(), 101);
    drop(ws);
}

#[tokio::test]
async fn ws_receives_pipeline_event() {
    let (url, handle) = start_server().await;
    let ws_url = format!("{url}/api/v1/ws/events");
    let (mut ws, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    // Give the handler time to subscribe to broadcast channels.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Publish a pipeline event.
    let tx = handle.state.events.pipeline_sender();
    let event = PipelineEvent::Audit(Box::new(EnrichedEvent {
        inner: Default::default(),
        received_at_ms: 0,
        source: EventSource::Sdk,
        agent_id: "agent-1".to_string(),
        connection_id: 0,
        sequence_number: 0,
    }));
    tx.send(event).unwrap();

    // Read the event from the WebSocket.
    use futures::StreamExt;
    let msg = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
        .await
        .expect("timeout waiting for WS message")
        .expect("stream ended")
        .expect("ws error");

    let text = msg.into_text().unwrap();
    let gov_event: GovernanceEvent = serde_json::from_str(&text).unwrap();
    assert_eq!(gov_event.event_type, EventType::Violation);
    assert_eq!(gov_event.agent_id, "agent-1");
}

#[tokio::test]
async fn ws_type_filter_excludes_non_matching() {
    let (url, handle) = start_server().await;
    // Only subscribe to budget events.
    let ws_url = format!("{url}/api/v1/ws/events?types=budget");
    let (mut ws, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Publish a pipeline (violation) event — should be filtered out.
    let tx = handle.state.events.pipeline_sender();
    let event = PipelineEvent::Audit(Box::new(EnrichedEvent {
        inner: Default::default(),
        received_at_ms: 0,
        source: EventSource::Sdk,
        agent_id: "agent-1".to_string(),
        connection_id: 0,
        sequence_number: 0,
    }));
    tx.send(event).unwrap();

    // Should not receive the violation event within a short window.
    use futures::StreamExt;
    let result = tokio::time::timeout(std::time::Duration::from_millis(200), ws.next()).await;
    assert!(result.is_err(), "should not receive filtered-out event type");
}

#[tokio::test]
async fn ws_replay_sends_buffered_events() {
    let (url, handle) = start_server().await;

    // Pre-populate the replay buffer.
    use chrono::Utc;
    for i in 1..=3 {
        handle.state.replay_buffer.push(GovernanceEvent {
            id: i,
            event_type: EventType::Violation,
            agent_id: "agent-1".to_string(),
            payload: serde_json::json!({"seq": i}),
            timestamp: Utc::now(),
        });
    }
    // Set next_event_id past the buffered events.
    handle.state.next_event_id.store(4, Ordering::Relaxed);

    // Connect with since=1 — should replay events 2 and 3.
    let ws_url = format!("{url}/api/v1/ws/events?since=1");
    let (mut ws, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    use futures::StreamExt;
    let msg1 = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let ev1: GovernanceEvent = serde_json::from_str(&msg1.into_text().unwrap()).unwrap();
    assert_eq!(ev1.id, 2);

    let msg2 = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let ev2: GovernanceEvent = serde_json::from_str(&msg2.into_text().unwrap()).unwrap();
    assert_eq!(ev2.id, 3);
}
