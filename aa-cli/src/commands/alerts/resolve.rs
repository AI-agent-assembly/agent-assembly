//! `aasm alerts resolve` — resolve an alert.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;

/// Arguments for `aasm alerts resolve`.
#[derive(Args)]
pub struct ResolveArgs {
    /// Alert ID to resolve.
    pub alert_id: String,

    /// Optional resolution note.
    #[arg(long)]
    pub reason: Option<String>,

    /// Skip the confirmation prompt.
    #[arg(long)]
    pub force: bool,
}

/// Run the `aasm alerts resolve` command.
pub fn run(_args: ResolveArgs, _ctx: &ResolvedContext) -> ExitCode {
    todo!()
}
