//! `aasm agent inspect` — show detailed agent information.

use std::process::ExitCode;

use clap::Args;
use comfy_table::{Cell, Color, Table};

use super::AgentResponse;
use crate::client;
use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for `aasm agent inspect`.
#[derive(Args)]
pub struct InspectArgs {
    /// Hex-encoded agent UUID to inspect.
    pub agent_id: String,
}

/// Render a detailed key-value view of an agent.
fn render_detail(agent: &AgentResponse) {
    let mut table = Table::new();
    table.set_header(vec!["Field", "Value"]);

    table.add_row(vec!["ID", &agent.id]);
    table.add_row(vec!["Name", &agent.name]);
    table.add_row(vec!["Framework", &agent.framework]);
    table.add_row(vec!["Version", &agent.version]);
    let status_color = match agent.status.to_lowercase().as_str() {
        "active" => Color::Green,
        s if s.starts_with("suspended") => Color::Yellow,
        "deregistered" => Color::Red,
        _ => Color::Reset,
    };
    table.add_row(vec![Cell::new("Status"), Cell::new(&agent.status).fg(status_color)]);

    let tools = if agent.tool_names.is_empty() {
        "(none)".to_string()
    } else {
        agent.tool_names.join(", ")
    };
    table.add_row(vec!["Tools".to_string(), tools]);

    let pid_str = agent.pid.map_or("-".to_string(), |p| p.to_string());
    table.add_row(vec!["PID".to_string(), pid_str]);

    let sessions_str = agent.session_count.map_or("-".to_string(), |s| s.to_string());
    table.add_row(vec!["Sessions".to_string(), sessions_str]);

    let last_event_str = agent.last_event.as_deref().unwrap_or("-").to_string();
    table.add_row(vec!["Last Event".to_string(), last_event_str]);

    let violations_str = agent.policy_violations_count.map_or("-".to_string(), |v| v.to_string());
    table.add_row(vec!["Policy Violations".to_string(), violations_str]);

    if !agent.metadata.is_empty() {
        let meta = agent
            .metadata
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        table.add_row(vec!["Metadata".to_string(), meta]);
    }

    println!("{table}");

    // Active sessions section
    if !agent.active_sessions.is_empty() {
        println!("\nActive Sessions:");
        let mut sessions_table = Table::new();
        sessions_table.set_header(vec!["SESSION_ID", "STARTED_AT", "STATUS"]);
        for s in &agent.active_sessions {
            sessions_table.add_row(vec![
                Cell::new(&s.session_id),
                Cell::new(&s.started_at),
                Cell::new(&s.status),
            ]);
        }
        println!("{sessions_table}");
    }

    // Recent events section
    if !agent.recent_events.is_empty() {
        println!("\nRecent Events:");
        let mut events_table = Table::new();
        events_table.set_header(vec!["TYPE", "SUMMARY", "TIMESTAMP"]);
        for e in &agent.recent_events {
            events_table.add_row(vec![
                Cell::new(&e.event_type),
                Cell::new(&e.summary),
                Cell::new(&e.timestamp),
            ]);
        }
        println!("{events_table}");
    }

    // Recent traces section
    if !agent.recent_traces.is_empty() {
        println!("\nRecent Traces:");
        let mut traces_table = Table::new();
        traces_table.set_header(vec!["SESSION_ID", "TIMESTAMP"]);
        for t in &agent.recent_traces {
            traces_table.add_row(vec![
                Cell::new(&t.session_id),
                Cell::new(&t.timestamp),
            ]);
        }
        println!("{traces_table}");
        println!("Tip: run `aasm trace <session-id>` to visualize a trace");
    }
}

/// Run the `aasm agent inspect` command.
pub fn run(args: InspectArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    let path = format!("/api/v1/agents/{}", args.agent_id);
    let agent: AgentResponse = match rt.block_on(client::get_json(ctx, &path)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    match output {
        OutputFormat::Table => render_detail(&agent),
        OutputFormat::Json => match serde_json::to_string_pretty(&agent) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("error serializing JSON: {e}"),
        },
        OutputFormat::Yaml => match serde_yaml::to_string(&agent) {
            Ok(yaml) => print!("{yaml}"),
            Err(e) => eprintln!("error serializing YAML: {e}"),
        },
    }

    ExitCode::SUCCESS
}
