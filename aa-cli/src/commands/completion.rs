//! `aasm completion` — generate shell completion scripts.

use std::process::ExitCode;

use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};

/// Arguments for `aasm completion`.
#[derive(Args)]
pub struct CompletionArgs {
    /// Shell to generate completions for.
    pub shell: Shell,
}

/// Generate shell completion script and write to stdout.
pub fn run(args: CompletionArgs) -> ExitCode {
    let mut cmd = crate::Cli::command();
    generate(args.shell, &mut cmd, "aasm", &mut std::io::stdout());
    ExitCode::SUCCESS
}
