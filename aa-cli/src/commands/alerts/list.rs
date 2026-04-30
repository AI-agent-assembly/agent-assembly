//! `aasm alerts list` — list governance alerts.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for `aasm alerts list`.
#[derive(Args)]
pub struct ListArgs {
    /// Filter by agent ID.
    #[arg(long)]
    pub agent: Option<String>,

    /// Filter by severity (critical, warning, info).
    #[arg(long)]
    pub severity: Option<String>,

    /// Filter by status (unresolved, acknowledged, resolved). Default: unresolved.
    #[arg(long, default_value = "unresolved")]
    pub status: Option<String>,
}

/// Run the `aasm alerts list` command.
pub fn run(_args: ListArgs, _ctx: &ResolvedContext, _output: OutputFormat) -> ExitCode {
    todo!()
}
