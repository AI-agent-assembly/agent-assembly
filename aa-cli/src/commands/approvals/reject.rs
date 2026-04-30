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

/// Validate that a rejection reason was provided.
///
/// Returns the reason string if present, or an error message explaining
/// that `--reason` is required for non-interactive rejection.
pub fn validate_reject_reason(reason: &Option<String>) -> Result<&str, &'static str> {
    match reason.as_deref() {
        Some(r) if !r.trim().is_empty() => Ok(r),
        _ => Err("error: --reason is required for aasm approvals reject"),
    }
}
