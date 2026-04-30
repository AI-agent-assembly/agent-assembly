//! `aasm agent kill` — deregister and terminate an agent.

use std::process::ExitCode;

use clap::Args;

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

/// Run the `aasm agent kill` command.
pub fn run(_args: KillArgs, _ctx: &ResolvedContext) -> ExitCode {
    ExitCode::SUCCESS
}
