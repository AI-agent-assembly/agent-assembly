//! Top-level CLI subcommand definitions and dispatch.

use clap::Subcommand;

pub mod policy;

/// Top-level subcommands for the `aasm` CLI.
#[derive(Subcommand)]
pub enum Commands {
    /// Manage governance policies.
    Policy(policy::PolicyArgs),
}

/// Dispatch the parsed CLI command to the appropriate handler.
pub fn dispatch(cmd: Commands) {
    match cmd {
        Commands::Policy(args) => policy::dispatch(args),
    }
}
