//! `aasm audit` — audit log query and compliance export.

use std::process::ExitCode;

use clap::{Args, Subcommand};

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

pub mod export;
pub mod list;
pub mod models;

/// Arguments for the `aasm audit` subcommand group.
#[derive(Debug, Args)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub command: AuditCommands,
}

/// Available audit subcommands.
#[derive(Debug, Subcommand)]
pub enum AuditCommands {
    /// Query audit log entries with optional filters.
    List(list::ListArgs),
    /// Export audit data in CSV or JSON format.
    Export(export::ExportArgs),
}

/// Dispatch an audit subcommand.
pub fn dispatch(args: AuditArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    match args.command {
        AuditCommands::List(list_args) => list::run(list_args, ctx, output),
        AuditCommands::Export(export_args) => export::run(export_args, ctx),
    }
}
