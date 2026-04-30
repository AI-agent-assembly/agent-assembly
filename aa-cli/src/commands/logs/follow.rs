//! Follow mode: real-time event streaming via WebSocket.

use std::collections::VecDeque;
use std::process::ExitCode;
use std::sync::Arc;

use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::{Mutex, Notify};
use tokio_tungstenite::tungstenite::Message;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

use super::format::{
    format_log_json, format_log_line, is_within_time_range, parse_since, LogLineData,
};
use super::LogsArgs;

/// A governance event as received from the WebSocket stream.
#[derive(Debug, Deserialize)]
struct WsEvent {
    #[allow(dead_code)]
    id: u64,
    event_type: String,
    agent_id: String,
    payload: serde_json::Value,
    timestamp: String,
}

impl WsEvent {
    fn to_line_data(&self) -> LogLineData {
        let message = match &self.payload {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        LogLineData {
            timestamp: self.timestamp.clone(),
            event_type: self.event_type.clone(),
            agent_id: self.agent_id.clone(),
            message,
        }
    }
}

/// Convert the resolved HTTP API URL to a WebSocket URL and append
/// the `/api/v1/ws/events` path with filter query parameters.
pub fn build_ws_url(ctx: &ResolvedContext, args: &LogsArgs) -> String {
    let base = ctx
        .api_url
        .replacen("https://", "wss://", 1)
        .replacen("http://", "ws://", 1);

    let mut url = format!("{base}/api/v1/ws/events");

    let mut params: Vec<String> = Vec::new();

    if let Some(ref types) = args.r#type {
        let type_str: Vec<&str> = types.iter().map(|t| t.as_api_str()).collect();
        params.push(format!("types={}", type_str.join(",")));
    }

    if let Some(ref agent) = args.agent {
        params.push(format!("agent_id={agent}"));
    }

    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    url
}

/// Stream events in real-time via WebSocket `/api/v1/ws/events`.
///
/// Connects to the gateway WebSocket endpoint, receives governance
/// events, formats them, and prints to stdout until Ctrl+C.
pub fn run(args: LogsArgs, ctx: &ResolvedContext) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(stream_events(args, ctx))
}

/// Maximum number of events buffered between the WebSocket receiver
/// and the stdout writer. When the buffer is full the oldest event
/// is dropped so the receiver never blocks.
const EVENT_BUFFER_CAPACITY: usize = 1000;

async fn stream_events(args: LogsArgs, ctx: &ResolvedContext) -> ExitCode {
    let url = build_ws_url(ctx, &args);

    let use_json = matches!(args.output, Some(OutputFormat::Json));
    let use_color = !args.no_color && !use_json;

    if args.until.is_some() {
        eprintln!("warning: --until is ignored in follow mode (real-time stream has no end bound)");
    }

    let since = args.since.as_deref().and_then(parse_since);

    let (ws_stream, _) = match tokio_tungstenite::connect_async(&url).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("error: failed to connect to WebSocket at {url}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let (_write, mut read) = ws_stream.split();

    // Ring buffer: when full the oldest event is evicted so the WS
    // reader never blocks and the display always shows the latest events.
    let buf: Arc<Mutex<VecDeque<LogLineData>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(EVENT_BUFFER_CAPACITY)));
    let notify = Arc::new(Notify::new());
    let ws_closed = Arc::new(Notify::new());

    let buf_tx = Arc::clone(&buf);
    let notify_tx = Arc::clone(&notify);
    let ws_closed_tx = Arc::clone(&ws_closed);

    // Spawn WS reader task — deserialises events and pushes into the ring buffer.
    let ws_reader = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let event: WsEvent = match serde_json::from_str(&text) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    let line_data = event.to_line_data();
                    let mut ring = buf_tx.lock().await;
                    if ring.len() >= EVENT_BUFFER_CAPACITY {
                        ring.pop_front(); // drop oldest
                    }
                    ring.push_back(line_data);
                    drop(ring);
                    notify_tx.notify_one();
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => {} // ping/pong/binary
                Err(_) => break,
            }
        }
        ws_closed_tx.notify_one();
    });

    // Main loop: drain the ring buffer and print, with Ctrl+C handling.
    loop {
        tokio::select! {
            _ = notify.notified() => {
                let mut ring = buf.lock().await;
                while let Some(line_data) = ring.pop_front() {
                    if !is_within_time_range(&line_data.timestamp, since.as_ref(), None) {
                        continue;
                    }
                    if use_json {
                        println!("{}", format_log_json(&line_data));
                    } else {
                        println!("{}", format_log_line(&line_data, use_color));
                    }
                }
            }
            _ = ws_closed.notified() => {
                eprintln!("WebSocket connection closed by server");
                return ExitCode::FAILURE;
            }
            _ = tokio::signal::ctrl_c() => {
                ws_reader.abort();
                return ExitCode::SUCCESS;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::logs::types::LogEventType;

    fn default_ctx() -> ResolvedContext {
        ResolvedContext {
            name: None,
            api_url: "http://localhost:8080".to_string(),
            api_key: None,
        }
    }

    fn default_args() -> LogsArgs {
        LogsArgs {
            follow: true,
            agent: None,
            r#type: None,
            since: None,
            until: None,
            limit: 50,
            no_color: false,
            output: None,
        }
    }

    #[test]
    fn build_ws_url_no_filters() {
        let url = build_ws_url(&default_ctx(), &default_args());
        assert_eq!(url, "ws://localhost:8080/api/v1/ws/events");
    }

    #[test]
    fn build_ws_url_https_becomes_wss() {
        let ctx = ResolvedContext {
            name: None,
            api_url: "https://api.example.com".to_string(),
            api_key: None,
        };
        let url = build_ws_url(&ctx, &default_args());
        assert!(url.starts_with("wss://api.example.com"));
    }

    #[test]
    fn build_ws_url_with_agent_filter() {
        let mut args = default_args();
        args.agent = Some("aa001".to_string());
        let url = build_ws_url(&default_ctx(), &args);
        assert!(url.contains("agent_id=aa001"));
    }

    #[test]
    fn build_ws_url_with_type_filter() {
        let mut args = default_args();
        args.r#type = Some(vec![LogEventType::Violation, LogEventType::Budget]);
        let url = build_ws_url(&default_ctx(), &args);
        assert!(url.contains("types=violation,budget"));
    }

    #[test]
    fn build_ws_url_with_combined_filters() {
        let mut args = default_args();
        args.agent = Some("aa002".to_string());
        args.r#type = Some(vec![LogEventType::Approval]);
        let url = build_ws_url(&default_ctx(), &args);
        assert!(url.contains("types=approval"));
        assert!(url.contains("agent_id=aa002"));
        assert!(url.contains('?'));
        assert!(url.contains('&'));
    }

    #[test]
    fn ws_event_deserializes_from_json() {
        let json = r#"{
            "id": 42,
            "event_type": "violation",
            "agent_id": "aa001",
            "payload": "policy denied tool call",
            "timestamp": "2026-04-30T10:00:00Z"
        }"#;
        let event: WsEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, 42);
        assert_eq!(event.event_type, "violation");
        assert_eq!(event.agent_id, "aa001");
    }

    #[test]
    fn ws_event_to_line_data_string_payload() {
        let event = WsEvent {
            id: 1,
            event_type: "budget".to_string(),
            agent_id: "aa002".to_string(),
            payload: serde_json::Value::String("threshold exceeded".to_string()),
            timestamp: "2026-04-30T11:00:00Z".to_string(),
        };
        let data = event.to_line_data();
        assert_eq!(data.message, "threshold exceeded");
        assert_eq!(data.event_type, "budget");
    }

    #[test]
    fn ws_event_to_line_data_object_payload() {
        let event = WsEvent {
            id: 2,
            event_type: "approval".to_string(),
            agent_id: "aa003".to_string(),
            payload: serde_json::json!({"action": "refund", "amount": 250}),
            timestamp: "2026-04-30T12:00:00Z".to_string(),
        };
        let data = event.to_line_data();
        // Object payloads are serialised to string.
        assert!(data.message.contains("refund"));
        assert!(data.message.contains("250"));
    }
}
