//! `aasm status` — kubectl-style tabular overview of governance state.

pub mod client;
pub mod fetch;
pub mod models;
pub mod render;

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

use models::StatusSnapshot;

/// Compute the process exit code from a status snapshot.
///
/// - `0` — all healthy
/// - `1` — at least one agent has violations
/// - `2` — runtime API is unreachable
pub fn compute_exit_code(snapshot: &StatusSnapshot) -> ExitCode {
    if !snapshot.runtime.reachable {
        return ExitCode::from(2);
    }
    let has_violations = snapshot.agents.iter().any(|a| a.violations_today > 0);
    if has_violations {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// Entry point for `aasm status`.
pub fn dispatch(_args: StatusArgs, _ctx: &ResolvedContext) -> ExitCode {
    eprintln!("status: not yet implemented");
    ExitCode::FAILURE
}
