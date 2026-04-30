//! `aasm approvals reject` — reject a pending action.

use clap::Args;

/// Arguments for the `aasm approvals reject` subcommand.
#[derive(Debug, Args)]
pub struct RejectArgs {
    /// Approval request ID to reject.
    pub id: String,

    /// Reason for rejection (required in non-interactive mode).
    #[arg(long)]
    pub reason: Option<String>,
}
