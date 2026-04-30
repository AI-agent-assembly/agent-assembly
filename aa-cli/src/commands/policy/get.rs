//! `aasm policy get` — display the currently active (or a specific) policy.

use std::process::ExitCode;

use aa_gateway::policy::history::{FsHistoryStore, HistoryConfig, PolicyHistoryStore};
use clap::Args;

/// Arguments for `aasm policy get`.
#[derive(Args)]
pub struct GetArgs {
    /// Version identifier (SHA-256 prefix) to retrieve.
    /// Shows the latest active policy when omitted.
    #[arg(long)]
    pub version: Option<String>,
}

/// Execute the `aasm policy get` command.
///
/// When `--version` is provided, retrieves that specific policy version.
/// Otherwise, retrieves the most recently applied (active) policy.
pub fn run(args: GetArgs) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let store = FsHistoryStore::new(HistoryConfig::default_config());

        if let Some(ref version) = args.version {
            match store.get(version).await {
                Ok(snapshot) => {
                    print!("{}", snapshot.yaml_content);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            }
        } else {
            match store.list(1).await {
                Ok(versions) if versions.is_empty() => {
                    eprintln!("No policy versions found.");
                    ExitCode::FAILURE
                }
                Ok(versions) => {
                    let latest = &versions[0];
                    let version_id = &latest.sha256;
                    match store.get(version_id).await {
                        Ok(snapshot) => {
                            print!("{}", snapshot.yaml_content);
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("error: {e}");
                            ExitCode::FAILURE
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    })
}
