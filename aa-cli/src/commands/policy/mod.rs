//! Policy management subcommands (`aasm policy ...`).

use clap::{Args, Subcommand};

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
    /// Simulate a policy against historical events or live traffic (dry-run).
    Simulate(simulate::SimulateArgs),
}

/// Dispatch a policy subcommand.
pub fn dispatch(args: PolicyArgs) {
    match args.command {
        PolicyCommands::Simulate(sim_args) => simulate::run(sim_args),
    }
}
