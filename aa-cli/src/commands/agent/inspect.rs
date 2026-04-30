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
    table.add_row(vec![
        Cell::new("Status"),
        Cell::new(&agent.status).fg(status_color),
    ]);

    let tools = if agent.tool_names.is_empty() {
        "(none)".to_string()
    } else {
        agent.tool_names.join(", ")
    };
    table.add_row(vec!["Tools".to_string(), tools]);

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
