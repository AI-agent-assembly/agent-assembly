//! `aasm logs` — query audit logs and stream live events.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for the `aasm logs` subcommand.
#[derive(Args)]
pub struct LogsArgs {
    /// Stream live events via WebSocket (like `tail -f`).
    #[arg(long, short)]
    pub follow: bool,

    /// Filter by agent ID.
    #[arg(long)]
    pub agent_id: Option<String>,

    /// Filter by event type (e.g. `violation`, `approval`, `budget`).
    #[arg(long)]
    pub event_type: Option<String>,

    /// Page number for paginated queries (default: 1).
    #[arg(long, default_value_t = 1)]
    pub page: u32,

    /// Items per page (default: 50, max: 100).
    #[arg(long, default_value_t = 50)]
    pub per_page: u32,
}

/// Run the `aasm logs` command.
pub fn run(args: LogsArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    if args.follow {
        run_follow(args, ctx)
    } else {
        run_query(args, ctx, output)
    }
}

/// Query audit logs via REST API.
fn run_query(
    _args: LogsArgs,
    _ctx: &ResolvedContext,
    _output: OutputFormat,
) -> ExitCode {
    // Implemented in the next commit.
    eprintln!("error: REST log query not yet implemented");
    ExitCode::FAILURE
}

/// Stream live events via WebSocket.
fn run_follow(_args: LogsArgs, _ctx: &ResolvedContext) -> ExitCode {
    // Implemented in a later commit.
    eprintln!("error: WebSocket follow not yet implemented");
    ExitCode::FAILURE
}
