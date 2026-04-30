//! `aasm status` — kubectl-style tabular overview of governance state.

use clap::Args;

/// Arguments for the `aasm status` subcommand.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Auto-refresh the status display every 5 seconds.
    #[arg(long)]
    pub watch: bool,
}
