//! Top-level CLI subcommand definitions and dispatch.

use std::process::ExitCode;

use clap::Subcommand;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

pub mod agent;
pub mod completion;
pub mod context;
pub mod logs;
pub mod policy;
pub mod trace;
pub mod version;

/// Top-level subcommands for the `aasm` CLI.
#[derive(Subcommand)]
pub enum Commands {
    /// Manage monitored agent processes.
    Agent(agent::AgentArgs),
    /// Manage governance policies.
    Policy(policy::PolicyArgs),
    /// Manage named API contexts (connection profiles).
    Context(context::ContextArgs),
    /// Query audit logs or stream live events.
    Logs(logs::LogsArgs),
    /// Generate shell completion scripts.
    Completion(completion::CompletionArgs),
    /// Show CLI and gateway version information.
    Version,
    /// Visualize a session trace (tree or timeline).
    Trace(trace::TraceArgs),
}

/// Dispatch the parsed CLI command to the appropriate handler.
pub fn dispatch(cmd: Commands, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    match cmd {
        Commands::Agent(args) => agent::dispatch(args, ctx, output),
        Commands::Policy(args) => policy::dispatch(args),
        Commands::Context(args) => context::dispatch(args),
        Commands::Logs(args) => logs::run(args, ctx, output),
        Commands::Completion(args) => completion::run(args),
        Commands::Version => version::run(ctx),
        Commands::Trace(args) => trace::dispatch(args, ctx, output),
    }
}
