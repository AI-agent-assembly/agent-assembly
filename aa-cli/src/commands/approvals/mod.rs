//! `aasm approvals` — human-in-the-loop approval management subcommands.

use clap::{Args, Subcommand};

pub mod approve;
pub mod client;
pub mod get;
pub mod list;
pub mod models;
pub mod reject;
pub mod watch;

/// Subcommands for `aasm approvals`.
#[derive(Debug, Subcommand)]
pub enum ApprovalsSubcommand {
    /// List all pending approval requests.
    List(list::ListArgs),
    /// Show details of a single pending approval request.
    Get(get::GetArgs),
    /// Approve a pending action.
    Approve(approve::ApproveArgs),
    /// Reject a pending action (--reason required).
    Reject(reject::RejectArgs),
    /// Watch for new approval requests in real time.
    Watch(watch::WatchArgs),
}

/// Top-level arguments for the `aasm approvals` command group.
#[derive(Debug, Args)]
pub struct ApprovalsArgs {
    #[command(subcommand)]
    pub command: ApprovalsSubcommand,
}
