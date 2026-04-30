//! `aasm approvals approve` — approve a pending action.

use clap::Args;

/// Arguments for the `aasm approvals approve` subcommand.
#[derive(Debug, Args)]
pub struct ApproveArgs {
    /// Approval request ID to approve.
    pub id: String,

    /// Optional reason for the approval.
    #[arg(long)]
    pub reason: Option<String>,
}
