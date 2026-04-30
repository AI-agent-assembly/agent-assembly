//! `aasm approvals reject` — reject a pending action.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;

use super::client;

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

/// Execute the `aasm approvals reject` subcommand.
pub fn run_reject(args: RejectArgs, ctx: &ResolvedContext) -> ExitCode {
    let reason = match validate_reject_reason(&args.reason) {
        Ok(r) => r.to_string(),
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::FAILURE;
        }
    };

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let result = rt.block_on(client::reject_action(ctx, &args.id, &reason));

    match result {
        Ok(resp) => {
            println!("Rejected: {} (status: {})", resp.id, resp.status);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
