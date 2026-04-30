//! WebSocket upgrade and event dispatch handler.

use std::time::Duration;

use crate::models::{EventType, GovernanceEvent};
use crate::state::AppState;
use crate::ws::params::WsQueryParams;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::Extension;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};

/// Interval between server-initiated ping frames.
const PING_INTERVAL: Duration = Duration::from_secs(30);

/// `GET /api/v1/ws/events` — upgrade to WebSocket and stream events.
///
/// Initiates a WebSocket connection for real-time governance event streaming.
///
/// ## Protocol
///
/// 1. Client sends an HTTP GET with `Upgrade: websocket` headers.
/// 2. Server responds with `101 Switching Protocols` and upgrades the connection.
/// 3. Server sends `GovernanceEvent` JSON objects as text frames.
/// 4. Server sends periodic ping frames (every 30s); client must respond with pong.
/// 5. Either side may close the connection with a close frame.
///
/// ## Replay
///
/// The server maintains a circular buffer of the last 1000 events. Pass the
/// `since` query parameter with a previously received event `id` to replay
/// all buffered events after that id before switching to live streaming.
///
/// ## Event Types
///
/// Filter events using the `types` query parameter (comma-separated):
/// - `violation` — audit / pipeline events (policy violations)
/// - `approval` — human-in-the-loop approval requests
/// - `budget` — budget threshold alerts
///
/// All types are streamed when the parameter is omitted.
#[utoipa::path(
    get,
    path = "/api/v1/ws/events",
    params(WsQueryParams),
    responses(
        (status = 101, description = "WebSocket upgrade successful. Server streams GovernanceEvent JSON text frames."),
        (status = 200, description = "Event message schema (delivered as WebSocket text frames, not as an HTTP response body).", body = GovernanceEvent),
        (status = 400, description = "Bad request (invalid query parameters)")
    ),
    tag = "events"
)]
pub async fn ws_events_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    Extension(state): Extension<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, params, state))
}

/// Drive a single WebSocket connection: replay, then stream live events.
async fn handle_socket(socket: WebSocket, params: WsQueryParams, state: AppState) {
    let (sender, mut receiver) = socket.split();
    let sender = std::sync::Arc::new(tokio::sync::Mutex::new(sender));

    let allowed_types = params.event_types();
    let agent_filter = params.agent_id.clone();

    // Replay buffered events if `since` was provided.
    if let Some(since_id) = params.since {
        let events = state.replay_buffer.events_since(since_id);
        let replay_sender = sender.clone();
        for event in events {
            if !matches_filter(&event, &allowed_types, agent_filter.as_deref()) {
                continue;
            }
            if send_event(&replay_sender, &event).await.is_err() {
                return;
            }
        }
    }

    // Subscribe to live broadcast channels.
    let mut pipeline_rx = state.events.subscribe_pipeline();
    let mut approval_rx = state.events.subscribe_approvals();
    let mut budget_rx = state.events.subscribe_budget();

    let live_sender = sender.clone();
    let ping_sender = sender.clone();

    // Spawn ping/pong keep-alive task.
    let pong_received = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let pong_flag = pong_received.clone();

    let ping_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(PING_INTERVAL).await;
            // Check that client responded to the previous ping.
            if !pong_flag.load(std::sync::atomic::Ordering::Relaxed) {
                tracing::debug!("pong timeout — closing WebSocket");
                let _ = ping_sender.lock().await.close().await;
                return;
            }
            pong_flag.store(false, std::sync::atomic::Ordering::Relaxed);
            if ping_sender.lock().await.send(Message::Ping(vec![])).await.is_err() {
                return;
            }
        }
    });

    // Spawn reader task to track pong responses and detect client close.
    let reader_pong = pong_received.clone();
    let reader_handle = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Pong(_) => {
                    reader_pong.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Event sequence counter for GovernanceEvent ids.
    let next_id = state.next_event_id.clone();

    // Main event dispatch loop.
    loop {
        let event = tokio::select! {
            Ok(pipeline_ev) = pipeline_rx.recv() => {
                let id = next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Some(GovernanceEvent {
                    id,
                    event_type: EventType::Violation,
                    agent_id: extract_pipeline_agent_id(&pipeline_ev),
                    payload: serde_json::to_value(format!("{pipeline_ev:?}")).unwrap_or_default(),
                    timestamp: chrono::Utc::now(),
                })
            }
            Ok(approval_ev) = approval_rx.recv() => {
                let id = next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Some(GovernanceEvent {
                    id,
                    event_type: EventType::Approval,
                    agent_id: approval_ev.agent_id.clone(),
                    payload: serde_json::to_value(format!("{approval_ev:?}")).unwrap_or_default(),
                    timestamp: chrono::Utc::now(),
                })
            }
            Ok(budget_ev) = budget_rx.recv() => {
                let id = next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Some(GovernanceEvent {
                    id,
                    event_type: EventType::Budget,
                    agent_id: format!("{:02x?}", budget_ev.agent_id.as_bytes()),
                    payload: serde_json::to_value(format!("{budget_ev:?}")).unwrap_or_default(),
                    timestamp: chrono::Utc::now(),
                })
            }
            else => None,
        };

        let Some(event) = event else { break };

        // Store in replay buffer before filtering.
        state.replay_buffer.push(event.clone());

        if !matches_filter(&event, &allowed_types, agent_filter.as_deref()) {
            continue;
        }

        if send_event(&live_sender, &event).await.is_err() {
            break;
        }
    }

    ping_handle.abort();
    reader_handle.abort();
}

/// Check whether an event passes the client's type and agent filters.
fn matches_filter(event: &GovernanceEvent, types: &[EventType], agent_id: Option<&str>) -> bool {
    if !types.contains(&event.event_type) {
        return false;
    }
    if let Some(filter_agent) = agent_id {
        if event.agent_id != filter_agent {
            return false;
        }
    }
    true
}

/// Extract the agent id from a pipeline event.
fn extract_pipeline_agent_id(ev: &aa_runtime::pipeline::event::PipelineEvent) -> String {
    match ev {
        aa_runtime::pipeline::event::PipelineEvent::Audit(enriched) => enriched.agent_id.clone(),
        aa_runtime::pipeline::event::PipelineEvent::LayerDegradation(info) => {
            format!("system:{}", info.layer)
        }
    }
}

/// Serialise a governance event and send it as a WebSocket text frame.
async fn send_event(
    sender: &std::sync::Arc<tokio::sync::Mutex<SplitSink<WebSocket, Message>>>,
    event: &GovernanceEvent,
) -> Result<(), ()> {
    let json = serde_json::to_string(event).map_err(|_| ())?;
    sender.lock().await.send(Message::Text(json)).await.map_err(|_| ())
}
