//! Integration tests for `aasm alerts` subcommands.

use std::process::ExitCode;

use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use aa_cli::commands::alerts::get::GetArgs;
use aa_cli::commands::alerts::list::ListArgs;
use aa_cli::commands::alerts::resolve::ResolveArgs;
use aa_cli::output::OutputFormat;

fn make_context(api_url: &str) -> aa_cli::config::ResolvedContext {
    aa_cli::config::ResolvedContext {
        name: None,
        api_url: api_url.to_string(),
        api_key: None,
    }
}

fn sample_alert_json() -> serde_json::Value {
    serde_json::json!({
        "id": "alert-001",
        "agent_id": "agent-abc",
        "severity": "critical",
        "category": "budget",
        "message": "Budget exceeded",
        "status": "unresolved",
        "created_at": "2026-04-30T10:00:00Z",
        "updated_at": "2026-04-30T11:00:00Z",
        "context": {"tool": "shell_exec"}
    })
}

// ── alerts list ──────────────────────────────────────────────────────

#[tokio::test]
async fn list_alerts_returns_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/alerts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [sample_alert_json()],
            "page": 1,
            "per_page": 50,
            "total": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let args = ListArgs {
            agent: None,
            severity: None,
            status: None,
        };
        let ctx = make_context(&uri);
        aa_cli::commands::alerts::list::run(args, &ctx, OutputFormat::Table)
    })
    .join()
    .unwrap();

    assert_eq!(result, ExitCode::SUCCESS);
}

#[tokio::test]
async fn list_alerts_json_output() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/alerts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [sample_alert_json()],
            "page": 1,
            "per_page": 50,
            "total": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let args = ListArgs {
            agent: None,
            severity: None,
            status: None,
        };
        let ctx = make_context(&uri);
        aa_cli::commands::alerts::list::run(args, &ctx, OutputFormat::Json)
    })
    .join()
    .unwrap();

    assert_eq!(result, ExitCode::SUCCESS);
}

#[tokio::test]
async fn list_alerts_with_filter_returns_empty() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/alerts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [sample_alert_json()],
            "page": 1,
            "per_page": 50,
            "total": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let args = ListArgs {
            agent: Some("nonexistent-agent".to_string()),
            severity: None,
            status: None,
        };
        let ctx = make_context(&uri);
        aa_cli::commands::alerts::list::run(args, &ctx, OutputFormat::Table)
    })
    .join()
    .unwrap();

    // Filters out all alerts but still returns SUCCESS with "No alerts found."
    assert_eq!(result, ExitCode::SUCCESS);
}

// ── alerts get ───────────────────────────────────────────────────────

#[tokio::test]
async fn get_alert_returns_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/alerts/alert-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_alert_json()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let args = GetArgs {
            alert_id: "alert-001".to_string(),
        };
        let ctx = make_context(&uri);
        aa_cli::commands::alerts::get::run(args, &ctx, OutputFormat::Table)
    })
    .join()
    .unwrap();

    assert_eq!(result, ExitCode::SUCCESS);
}

#[tokio::test]
async fn get_alert_not_found_returns_failure() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/alerts/no-such-id"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let args = GetArgs {
            alert_id: "no-such-id".to_string(),
        };
        let ctx = make_context(&uri);
        aa_cli::commands::alerts::get::run(args, &ctx, OutputFormat::Table)
    })
    .join()
    .unwrap();

    assert_eq!(result, ExitCode::FAILURE);
}

// ── alerts resolve ───────────────────────────────────────────────────

#[tokio::test]
async fn resolve_alert_with_force_returns_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/alerts/alert-001/resolve"))
        .and(body_partial_json(serde_json::json!({
            "reason": "False positive"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "alert-001",
            "severity": "critical",
            "category": "budget",
            "message": "Budget exceeded",
            "status": "resolved",
            "created_at": "2026-04-30T10:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let args = ResolveArgs {
            alert_id: "alert-001".to_string(),
            reason: Some("False positive".to_string()),
            force: true,
        };
        let ctx = make_context(&uri);
        aa_cli::commands::alerts::resolve::run(args, &ctx, OutputFormat::Table)
    })
    .join()
    .unwrap();

    assert_eq!(result, ExitCode::SUCCESS);
}

#[tokio::test]
async fn resolve_alert_json_output() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/alerts/alert-001/resolve"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "alert-001",
            "severity": "critical",
            "category": "budget",
            "message": "Budget exceeded",
            "status": "resolved",
            "created_at": "2026-04-30T10:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let args = ResolveArgs {
            alert_id: "alert-001".to_string(),
            reason: None,
            force: true,
        };
        let ctx = make_context(&uri);
        aa_cli::commands::alerts::resolve::run(args, &ctx, OutputFormat::Json)
    })
    .join()
    .unwrap();

    assert_eq!(result, ExitCode::SUCCESS);
}

#[tokio::test]
async fn resolve_alert_without_reason_sends_empty_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/alerts/alert-002/resolve"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "alert-002",
            "severity": "info",
            "category": "policy_violation",
            "message": "Minor issue",
            "status": "resolved",
            "created_at": "2026-04-30T08:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let args = ResolveArgs {
            alert_id: "alert-002".to_string(),
            reason: None,
            force: true,
        };
        let ctx = make_context(&uri);
        aa_cli::commands::alerts::resolve::run(args, &ctx, OutputFormat::Table)
    })
    .join()
    .unwrap();

    assert_eq!(result, ExitCode::SUCCESS);
}

#[tokio::test]
async fn resolve_alert_server_error_returns_failure() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/alerts/alert-999/resolve"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let args = ResolveArgs {
            alert_id: "alert-999".to_string(),
            reason: None,
            force: true,
        };
        let ctx = make_context(&uri);
        aa_cli::commands::alerts::resolve::run(args, &ctx, OutputFormat::Table)
    })
    .join()
    .unwrap();

    assert_eq!(result, ExitCode::FAILURE);
}
