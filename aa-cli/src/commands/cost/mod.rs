//! Cost management subcommands (`aasm cost ...`).

use std::process::ExitCode;

use clap::{Args, Subcommand};

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

pub mod client;
pub mod forecast;
pub mod models;
pub mod summary;

/// Arguments for the `aasm cost` subcommand group.
#[derive(Args)]
pub struct CostArgs {
    #[command(subcommand)]
    pub command: CostCommands,
}

/// Available cost subcommands.
#[derive(Subcommand)]
pub enum CostCommands {
    /// Show cost summary for the current period.
    Summary(summary::SummaryArgs),
    /// Forecast monthly spending based on current daily rate.
    Forecast(forecast::ForecastArgs),
}

/// Dispatch a cost subcommand.
pub fn dispatch(args: CostArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    match args.command {
        CostCommands::Summary(summary_args) => summary::run(summary_args, ctx, output),
        CostCommands::Forecast(forecast_args) => forecast::run(forecast_args, ctx, output),
    }
}
