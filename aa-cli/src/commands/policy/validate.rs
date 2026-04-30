//! `aasm policy validate` — local-only policy YAML validation.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;

/// Arguments for `aasm policy validate`.
#[derive(Args)]
pub struct ValidateArgs {
    /// Path to the policy YAML file to validate.
    pub file: PathBuf,
}

/// Execute the `aasm policy validate` command.
///
/// Validates the policy YAML file locally using [`PolicyValidator::from_yaml`].
/// Exits 0 if valid, 1 if invalid with error details printed to stderr.
pub fn run(args: ValidateArgs) -> ExitCode {
    let yaml = match std::fs::read_to_string(&args.file) {
        Ok(y) => y,
        Err(e) => {
            eprintln!("error: failed to read {}: {e}", args.file.display());
            return ExitCode::FAILURE;
        }
    };

    match aa_gateway::policy::PolicyValidator::from_yaml(&yaml) {
        Ok(output) => {
            for w in &output.warnings {
                eprintln!("warning: {w:?}");
            }
            println!("Policy is valid: {}", args.file.display());
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {e:?}");
            }
            ExitCode::FAILURE
        }
    }
}
