//! `aasm cost summary` — display cost summary for the current period.

use std::process::ExitCode;

use clap::{Args, ValueEnum};

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Time period for cost aggregation.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Period {
    /// Today's spend only.
    #[default]
    Today,
    /// Current month's spend.
    Month,
}

/// Arguments for `aasm cost summary`.
#[derive(Args)]
pub struct SummaryArgs {
    /// Time period to report on.
    #[arg(long, value_enum, default_value_t = Period::Today)]
    pub period: Period,

    /// Group spend by dimension.
    #[arg(long, value_enum)]
    pub group_by: Option<GroupBy>,
}

/// Grouping dimension for cost summary.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GroupBy {
    /// Group by agent.
    Agent,
}

pub fn run(_args: SummaryArgs, _ctx: &ResolvedContext, _output: OutputFormat) -> ExitCode {
    ExitCode::SUCCESS
}
