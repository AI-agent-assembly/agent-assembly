//! `aasm status` — kubectl-style tabular overview of governance state.

pub mod client;
pub mod fetch;
pub mod models;
pub mod render;
pub mod watch;

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for the `aasm status` subcommand.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Auto-refresh the status display every 5 seconds.
    #[arg(long)]
    pub watch: bool,
}

use models::StatusSnapshot;

/// Compute the process exit code from a status snapshot.
///
/// - `0` — all healthy
/// - `1` — at least one agent has violations
/// - `2` — runtime API is unreachable
pub fn compute_exit_code(snapshot: &StatusSnapshot) -> ExitCode {
    if !snapshot.runtime.reachable {
        return ExitCode::from(2);
    }
    let has_violations = snapshot.agents.iter().any(|a| a.violations_today > 0);
    if has_violations {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// Entry point for `aasm status`.
pub fn dispatch(args: StatusArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let api_client = client::StatusClient::new(&ctx.api_url);

        if args.watch {
            watch::run_watch_loop(&api_client, output).await;
            ExitCode::SUCCESS
        } else {
            let snapshot = fetch::fetch_all(&api_client).await;
            render::render_all(&snapshot, output);
            compute_exit_code(&snapshot)
        }
    })
}
