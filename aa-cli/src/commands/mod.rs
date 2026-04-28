//! Top-level CLI subcommand definitions and dispatch.

use std::process::ExitCode;

use clap::Subcommand;

pub mod policy;

/// Top-level subcommands for the `aasm` CLI.
#[derive(Subcommand)]
pub enum Commands {
    /// Manage governance policies.
    Policy(policy::PolicyArgs),
}

/// Dispatch the parsed CLI command to the appropriate handler.
pub fn dispatch(cmd: Commands) -> ExitCode {
    match cmd {
        Commands::Policy(args) => policy::dispatch(args),
    }
}
