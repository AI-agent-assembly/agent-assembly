//! `aasm` — the Agent Assembly command-line tool.
//!
//! Provides commands for managing agents, policies, and the governance
//! gateway from the terminal.

use std::process::ExitCode;

use clap::Parser;

mod commands;

/// Agent Assembly CLI — governance gateway management tool.
#[derive(Parser)]
#[command(name = "aasm", version, about)]
struct Cli {
    #[command(subcommand)]
    command: commands::Commands,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    commands::dispatch(cli.command)
}
