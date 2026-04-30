//! Follow mode: real-time event streaming via WebSocket.

use std::process::ExitCode;

use futures_util::StreamExt;
use serde::Deserialize;
use tokio_tungstenite::tungstenite::Message;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

use super::format::{format_log_json, format_log_line, LogLineData};
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

async fn stream_events(args: LogsArgs, ctx: &ResolvedContext) -> ExitCode {
    let url = build_ws_url(ctx, &args);

    let use_json = matches!(args.output, Some(OutputFormat::Json));
    let use_color = !args.no_color && !use_json;

    let (ws_stream, _) = match tokio_tungstenite::connect_async(&url).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("error: failed to connect to WebSocket at {url}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let (_write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let event: WsEvent = match serde_json::from_str(&text) {
                            Ok(e) => e,
                            Err(_) => continue,
                        };
                        let line_data = event.to_line_data();
                        if use_json {
                            println!("{}", format_log_json(&line_data));
                        } else {
                            println!("{}", format_log_line(&line_data, use_color));
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        eprintln!("WebSocket connection closed by server");
                        return ExitCode::FAILURE;
                    }
                    Some(Ok(_)) => {
                        // Ignore ping/pong/binary frames.
                    }
                    Some(Err(e)) => {
                        eprintln!("error: WebSocket read error: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
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
}
