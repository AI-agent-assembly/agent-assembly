//! `aasm approvals get` — show details of a single pending approval.

use clap::Args;

use crate::output::OutputFormat;

/// Arguments for the `aasm approvals get` subcommand.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Approval request ID to look up.
    pub id: String,

    /// Output format override for this subcommand.
    #[arg(long, value_enum)]
    pub output: Option<OutputFormat>,
}
