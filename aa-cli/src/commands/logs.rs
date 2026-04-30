//! `aasm logs` — query audit logs and stream live events.

use std::process::ExitCode;

use clap::Args;
use comfy_table::{Cell, Table};
use serde::Deserialize;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for the `aasm logs` subcommand.
#[derive(Args)]
pub struct LogsArgs {
    /// Stream live events via WebSocket (like `tail -f`).
    #[arg(long, short)]
    pub follow: bool,

    /// Filter by agent ID.
    #[arg(long)]
    pub agent_id: Option<String>,

    /// Filter by event type (e.g. `violation`, `approval`, `budget`).
    #[arg(long)]
    pub event_type: Option<String>,

    /// Page number for paginated queries (default: 1).
    #[arg(long, default_value_t = 1)]
    pub page: u32,

    /// Items per page (default: 50, max: 100).
    #[arg(long, default_value_t = 50)]
    pub per_page: u32,
}

/// Paginated response envelope from the API.
#[derive(Debug, Deserialize)]
struct PaginatedLogs {
    items: Vec<LogEntry>,
    page: u32,
    per_page: u32,
    total: u64,
}

/// A single audit log entry returned by `GET /api/v1/logs`.
#[derive(Debug, Deserialize, serde::Serialize)]
struct LogEntry {
    seq: u64,
    timestamp: String,
    agent_id: String,
    session_id: String,
    event_type: String,
    payload: String,
}

/// Run the `aasm logs` command.
pub fn run(args: LogsArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    if args.follow {
        run_follow(args, ctx)
    } else {
        run_query(args, ctx, output)
    }
}

/// Query audit logs via REST API.
fn run_query(args: LogsArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async { fetch_logs(args, ctx, output).await })
}

/// Fetch paginated logs from the gateway and render output.
async fn fetch_logs(
    args: LogsArgs,
    ctx: &ResolvedContext,
    output: OutputFormat,
) -> ExitCode {
    let client = reqwest::Client::new();
    let mut url = format!(
        "{}/api/v1/logs?page={}&per_page={}",
        ctx.api_url, args.page, args.per_page,
    );
    if let Some(ref agent_id) = args.agent_id {
        url.push_str(&format!("&agent_id={agent_id}"));
    }
    if let Some(ref event_type) = args.event_type {
        url.push_str(&format!("&event_type={event_type}"));
    }

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: failed to reach gateway: {e}");
            return ExitCode::FAILURE;
        }
    };

    if !resp.status().is_success() {
        eprintln!("error: gateway returned {}", resp.status());
        return ExitCode::FAILURE;
    }

    let body: PaginatedLogs = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: invalid response: {e}");
            return ExitCode::FAILURE;
        }
    };

    match output {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&body.items).unwrap_or_default()
            );
        }
        OutputFormat::Yaml => {
            println!(
                "{}",
                serde_yaml::to_string(&body.items).unwrap_or_default()
            );
        }
        OutputFormat::Table => {
            print_logs_table(&body);
        }
    }

    ExitCode::SUCCESS
}

/// Render a paginated log response as a human-readable table.
fn print_logs_table(body: &PaginatedLogs) {
    if body.items.is_empty() {
        println!("No log entries found.");
        return;
    }

    let mut table = Table::new();
    table.set_header(vec!["SEQ", "TIMESTAMP", "AGENT", "SESSION", "TYPE", "PAYLOAD"]);

    for entry in &body.items {
        let short_agent = truncate(&entry.agent_id, 12);
        let short_session = truncate(&entry.session_id, 12);
        let short_payload = truncate(&entry.payload, 40);
        table.add_row(vec![
            Cell::new(entry.seq),
            Cell::new(&entry.timestamp),
            Cell::new(short_agent),
            Cell::new(short_session),
            Cell::new(&entry.event_type),
            Cell::new(short_payload),
        ]);
    }

    println!("{table}");
    println!(
        "Page {}/{} ({} total entries)",
        body.page,
        (body.total + u64::from(body.per_page) - 1) / u64::from(body.per_page),
        body.total,
    );
}

/// Truncate a string to `max_len` characters, appending `…` if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

/// Stream live events via WebSocket.
fn run_follow(_args: LogsArgs, _ctx: &ResolvedContext) -> ExitCode {
    // Implemented in the next commit.
    eprintln!("error: WebSocket follow not yet implemented");
    ExitCode::FAILURE
}
