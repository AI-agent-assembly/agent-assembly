//! `aasm approvals list` — list pending approval requests.

use clap::Args;

use crate::output::OutputFormat;

/// Arguments for the `aasm approvals list` subcommand.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Output format override for this subcommand.
    #[arg(long, value_enum)]
    pub output: Option<OutputFormat>,
}
