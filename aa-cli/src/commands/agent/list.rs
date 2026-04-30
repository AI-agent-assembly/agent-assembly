//! `aasm agent list` — list all registered agents.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for `aasm agent list`.
#[derive(Args)]
pub struct ListArgs {
    /// Filter by agent status (e.g. active, suspended, deregistered).
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by agent framework (e.g. langgraph, crewai).
    #[arg(long)]
    pub framework: Option<String>,

    /// Auto-refresh the table every 2 seconds.
    #[arg(long)]
    pub watch: bool,
}

/// Run the `aasm agent list` command.
pub fn run(_args: ListArgs, _ctx: &ResolvedContext, _output: OutputFormat) -> ExitCode {
    ExitCode::SUCCESS
}
