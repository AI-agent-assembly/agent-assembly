//! `aasm alerts get` — show full alert detail.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for `aasm alerts get`.
#[derive(Args)]
pub struct GetArgs {
    /// Alert ID to inspect.
    pub alert_id: String,
}

/// Run the `aasm alerts get` command.
pub fn run(_args: GetArgs, _ctx: &ResolvedContext, _output: OutputFormat) -> ExitCode {
    todo!()
}
