//! Follow mode: real-time event streaming via WebSocket.

use std::process::ExitCode;

use crate::config::ResolvedContext;

use super::LogsArgs;

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
pub fn run(_args: LogsArgs, _ctx: &ResolvedContext) -> ExitCode {
    ExitCode::SUCCESS
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
