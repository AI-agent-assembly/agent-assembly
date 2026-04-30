//! `aasm` — the Agent Assembly command-line tool.
//!
//! Provides commands for managing agents, policies, and the governance
//! gateway from the terminal.

use std::process::ExitCode;

use aa_cli::{commands, config, Cli};
use clap::Parser;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error loading config: {e}");
            return ExitCode::FAILURE;
        }
    };

    let resolved = match config::resolve_context(
        &cfg,
        cli.context.as_deref(),
        cli.api_url.as_deref(),
        cli.api_key.as_deref(),
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    commands::dispatch(cli.command, &resolved, cli.output)
}
