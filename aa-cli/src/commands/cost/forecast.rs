//! `aasm cost forecast` — project monthly spending based on current daily rate.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for `aasm cost forecast`.
#[derive(Args)]
pub struct ForecastArgs {}

pub fn run(_args: ForecastArgs, _ctx: &ResolvedContext, _output: OutputFormat) -> ExitCode {
    ExitCode::SUCCESS
}
