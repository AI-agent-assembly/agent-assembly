//! `aasm status` — kubectl-style tabular overview of governance state.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;

/// Arguments for the `aasm status` subcommand.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Auto-refresh the status display every 5 seconds.
    #[arg(long)]
    pub watch: bool,
}

/// Entry point for `aasm status`.
pub fn dispatch(_args: StatusArgs, _ctx: &ResolvedContext) -> ExitCode {
    eprintln!("status: not yet implemented");
    ExitCode::FAILURE
}
