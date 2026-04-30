//! `aasm trace` — session trace visualization.

use std::process::ExitCode;

use clap::{Args, ValueEnum};

use crate::config::ResolvedContext;

pub mod models;

/// Visualization format for trace output.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum TraceFormat {
    /// Indented tree with box-drawing characters (default).
    #[default]
    Tree,
    /// Horizontal ASCII timeline with duration bars.
    Timeline,
}

/// Arguments for the `aasm trace` subcommand.
#[derive(Debug, Args)]
pub struct TraceArgs {
    /// Session ID to retrieve the trace for.
    pub session_id: String,

    /// Visualization format.
    #[arg(long, value_enum, default_value_t = TraceFormat::Tree)]
    pub format: TraceFormat,
}

/// Execute the `aasm trace` subcommand.
pub fn dispatch(_args: TraceArgs, _ctx: &ResolvedContext) -> ExitCode {
    ExitCode::SUCCESS
}
