//! Top-level CLI subcommand definitions and dispatch.

use std::process::ExitCode;

use clap::Subcommand;

use crate::config::ResolvedContext;

pub mod completion;
pub mod context;
pub mod policy;
pub mod version;

/// Top-level subcommands for the `aasm` CLI.
#[derive(Subcommand)]
pub enum Commands {
    /// Manage governance policies.
    Policy(policy::PolicyArgs),
    /// Manage named API contexts (connection profiles).
    Context(context::ContextArgs),
    /// Generate shell completion scripts.
    Completion(completion::CompletionArgs),
    /// Show CLI and gateway version information.
    Version,
}

/// Dispatch the parsed CLI command to the appropriate handler.
pub fn dispatch(cmd: Commands, ctx: &ResolvedContext) -> ExitCode {
    match cmd {
        Commands::Policy(args) => policy::dispatch(args),
        Commands::Context(args) => context::dispatch(args),
        Commands::Completion(args) => completion::run(args),
        Commands::Version => version::run(ctx),
    }
}
