//! `aasm` — the Agent Assembly command-line tool.
//!
//! Provides commands for managing agents, policies, and the governance
//! gateway from the terminal.

use std::process::ExitCode;

use clap::Parser;

mod client;
mod commands;
mod config;
mod error;
mod output;

/// Agent Assembly CLI — governance gateway management tool.
#[derive(Parser)]
#[command(name = "aasm", version, about)]
struct Cli {
    /// Named context from ~/.aa/config.yaml to use.
    #[arg(long, global = true)]
    context: Option<String>,

    /// Output format for list/get commands.
    #[arg(long, global = true, value_enum, default_value_t = output::OutputFormat::Table)]
    output: output::OutputFormat,

    /// Override the API URL (takes precedence over context config).
    #[arg(long, global = true)]
    api_url: Option<String>,

    /// Override the API key (takes precedence over context config).
    #[arg(long, global = true)]
    api_key: Option<String>,

    #[command(subcommand)]
    command: commands::Commands,
}

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
