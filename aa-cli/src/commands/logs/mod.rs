//! `aasm logs` — paginated audit log viewer and real-time log tail.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

pub mod fetch;
pub mod follow;
pub mod format;
pub mod types;

use types::LogEventType;

/// Arguments for the `aasm logs` subcommand.
#[derive(Debug, Args)]
pub struct LogsArgs {
    /// Stream events in real-time (like `tail -f`). Connects via WebSocket.
    #[arg(long, short = 'f')]
    pub follow: bool,

    /// Filter by agent identifier.
    #[arg(long)]
    pub agent: Option<String>,

    /// Filter by event type (comma-separated). Accepted: violation, approval, budget.
    #[arg(long, value_delimiter = ',')]
    pub r#type: Option<Vec<LogEventType>>,

    /// Show events after this duration or ISO 8601 timestamp (e.g. `30m`, `2h`, `2026-04-30T10:00:00Z`).
    #[arg(long)]
    pub since: Option<String>,

    /// Show events before this ISO 8601 timestamp.
    #[arg(long)]
    pub until: Option<String>,

    /// Maximum number of entries to return in non-follow mode.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,

    /// Disable colour output.
    #[arg(long)]
    pub no_color: bool,

    /// Override the global output format for this command.
    #[arg(long, value_enum)]
    pub output: Option<OutputFormat>,
}

/// Dispatch the `aasm logs` command to fetch or follow mode.
pub fn dispatch(args: LogsArgs, ctx: &ResolvedContext) -> ExitCode {
    if args.follow {
        follow::run(args, ctx)
    } else {
        fetch::run(args, ctx)
    }
}
