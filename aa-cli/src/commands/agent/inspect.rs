//! `aasm agent inspect` — show detailed agent information.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for `aasm agent inspect`.
#[derive(Args)]
pub struct InspectArgs {
    /// Hex-encoded agent UUID to inspect.
    pub agent_id: String,
}

/// Run the `aasm agent inspect` command.
pub fn run(_args: InspectArgs, _ctx: &ResolvedContext, _output: OutputFormat) -> ExitCode {
    ExitCode::SUCCESS
}
