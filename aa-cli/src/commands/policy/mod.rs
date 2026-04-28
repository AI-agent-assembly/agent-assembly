//! Policy management subcommands (`aasm policy ...`).

use std::process::ExitCode;

use clap::{Args, Subcommand};

pub mod history;
pub mod simulate;

/// Arguments for the `aasm policy` subcommand group.
#[derive(Args)]
pub struct PolicyArgs {
    #[command(subcommand)]
    pub command: PolicyCommands,
}

/// Available policy subcommands.
#[derive(Subcommand)]
pub enum PolicyCommands {
    /// Apply a policy YAML file and save it to version history.
    Apply(history::ApplyArgs),
    /// List recent policy versions.
    History(history::HistoryArgs),
    /// Roll back to a previous policy version.
    Rollback(history::RollbackArgs),
    /// Show the diff between two policy versions.
    Diff(history::DiffArgs),
    /// Simulate a policy against historical events or live traffic (dry-run).
    Simulate(simulate::SimulateArgs),
}

/// Dispatch a policy subcommand.
pub fn dispatch(args: PolicyArgs) -> ExitCode {
    match args.command {
        PolicyCommands::Apply(apply_args) => history::run_apply(apply_args),
        PolicyCommands::History(history_args) => history::run_history(history_args),
        PolicyCommands::Rollback(rollback_args) => history::run_rollback(rollback_args),
        PolicyCommands::Diff(diff_args) => history::run_diff(diff_args),
        PolicyCommands::Simulate(sim_args) => simulate::run(sim_args),
    }
}
