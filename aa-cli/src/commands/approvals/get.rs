//! `aasm approvals get` — show details of a single pending approval.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

use super::client;

/// Arguments for the `aasm approvals get` subcommand.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Approval request ID to look up.
    pub id: String,

    /// Output format override for this subcommand.
    #[arg(long, value_enum)]
    pub output: Option<OutputFormat>,
}

/// Execute the `aasm approvals get` subcommand.
pub fn run_get(args: GetArgs, ctx: &ResolvedContext, global_output: OutputFormat) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let result = rt.block_on(client::get_approval(ctx, &args.id));

    match result {
        Ok(approval) => {
            let format = args.output.unwrap_or(global_output);
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&approval).unwrap_or_default());
                }
                OutputFormat::Yaml => {
                    println!("{}", serde_yaml::to_string(&approval).unwrap_or_default());
                }
                OutputFormat::Table => {
                    println!("ID:         {}", approval.id);
                    println!("Agent:      {}", approval.agent_id);
                    println!("Action:     {}", approval.action);
                    println!("Condition:  {}", approval.reason);
                    println!("Status:     {}", approval.status);
                    println!("Created at: {}", approval.created_at);
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
