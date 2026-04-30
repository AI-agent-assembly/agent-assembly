//! Integration test: `aasm logs --follow` WebSocket streaming.
//!
//! Spins up a lightweight WebSocket server that sends a single
//! governance event, then verifies a tokio-tungstenite client
//! receives the expected JSON frame.

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

/// A governance event matching the shape sent by `GET /api/v1/ws/events`.
fn sample_event() -> serde_json::Value {
    json!({
        "id": 1,
        "event_type": "violation",
        "agent_id": "agent-abc",
        "payload": {"detail": "blocked tool call"},
        "timestamp": "2025-06-01T12:00:00Z"
    })
}

#[tokio::test]
async fn client_receives_event_from_mock_ws_server() {
    // Bind to an ephemeral port.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn the mock WebSocket server.
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        // Send one event.
        let event_json = serde_json::to_string(&sample_event()).unwrap();
        ws.send(Message::Text(event_json.into())).await.unwrap();

        // Close gracefully.
        ws.close(None).await.ok();
    });

    // Connect as a client (mimicking what `aasm logs --follow` does internally).
    let url = format!("ws://127.0.0.1:{}", addr.port());
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("client should connect to mock server");

    // Read the event frame.
    let msg = ws_stream
        .next()
        .await
        .expect("should receive a message")
        .expect("message should be Ok");

    match msg {
        Message::Text(text) => {
            let event: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(event["id"], 1);
            assert_eq!(event["event_type"], "violation");
            assert_eq!(event["agent_id"], "agent-abc");
        }
        other => panic!("expected Text frame, got {other:?}"),
    }

    server.await.unwrap();
}

#[tokio::test]
async fn client_handles_server_close() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        // Close immediately without sending any events.
        ws.close(None).await.ok();
    });

    let url = format!("ws://127.0.0.1:{}", addr.port());
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("client should connect");

    // The next message should be Close or None.
    let msg = ws_stream.next().await;
    match msg {
        Some(Ok(Message::Close(_))) | None => { /* expected */ }
        other => panic!("expected Close or end of stream, got {other:?}"),
    }

    server.await.unwrap();
}
