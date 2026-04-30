//! `aasm alerts` — manage governance alerts.

use std::process::ExitCode;

use clap::{Args, Subcommand};

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

pub mod get;
pub mod list;
pub mod models;
pub mod resolve;

/// Arguments for the `aasm alerts` subcommand group.
#[derive(Args)]
pub struct AlertsArgs {
    #[command(subcommand)]
    pub command: AlertsCommands,
}

/// Available alerts subcommands.
#[derive(Subcommand)]
pub enum AlertsCommands {
    /// List governance alerts.
    List(list::ListArgs),
    /// Show full detail for a specific alert.
    Get(get::GetArgs),
    /// Resolve an alert.
    Resolve(resolve::ResolveArgs),
}

/// Dispatch an alerts subcommand.
pub fn dispatch(args: AlertsArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    match args.command {
        AlertsCommands::List(list_args) => list::run(list_args, ctx, output),
        AlertsCommands::Get(get_args) => get::run(get_args, ctx, output),
        AlertsCommands::Resolve(resolve_args) => resolve::run(resolve_args, ctx),
    }
}
