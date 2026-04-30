//! `aasm agent kill` — deregister and terminate an agent.

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Args;

use crate::client;
use crate::config::ResolvedContext;

/// Arguments for `aasm agent kill`.
#[derive(Args)]
pub struct KillArgs {
    /// Hex-encoded agent UUID to kill.
    pub agent_id: String,

    /// Skip the confirmation prompt.
    #[arg(long)]
    pub force: bool,
}

/// Prompt the user for confirmation. Returns true if confirmed.
fn confirm_kill(agent_id: &str) -> bool {
    eprint!("Are you sure you want to kill agent {agent_id}? [y/N] ");
    io::stderr().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Run the `aasm agent kill` command.
pub fn run(args: KillArgs, ctx: &ResolvedContext) -> ExitCode {
    if !args.force && !confirm_kill(&args.agent_id) {
        eprintln!("Aborted.");
        return ExitCode::FAILURE;
    }

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    let path = format!("/api/v1/agents/{}", args.agent_id);
    match rt.block_on(client::delete(ctx, &path)) {
        Ok(()) => {
            println!("Agent {} has been killed.", args.agent_id);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
