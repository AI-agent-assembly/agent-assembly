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
    run_with_config(args, HistoryConfig::default_config())
}

/// Inner implementation that accepts a [`HistoryConfig`] for testability.
fn run_with_config(args: GetArgs, config: HistoryConfig) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let store = FsHistoryStore::new(config);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(dir: &std::path::Path) -> HistoryConfig {
        HistoryConfig {
            history_dir: dir.join("policy-history"),
            max_versions: 100,
        }
    }

    #[test]
    fn get_no_versions_exits_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let config = test_config(tmp.path());
        let args = GetArgs { version: None };
        assert_eq!(run_with_config(args, config), ExitCode::FAILURE);
    }

    #[test]
    fn get_latest_after_apply_exits_success() {
        let tmp = tempfile::tempdir().unwrap();
        let config = test_config(tmp.path());

        // Apply a policy first
        let rt = tokio::runtime::Runtime::new().unwrap();
        let store = FsHistoryStore::new(config.clone());
        let yaml = "tier: low\nrules:\n  - id: r1\n    description: test\n    match:\n      actions: [\"*\"]\n    effect: allow\n    audit: true\n";
        let meta = rt.block_on(store.save(yaml, Some("test"))).unwrap();

        // Now get latest
        let args = GetArgs { version: None };
        assert_eq!(run_with_config(args, config.clone()), ExitCode::SUCCESS);

        // Get by specific version
        let args = GetArgs {
            version: Some(meta.sha256[..12].to_string()),
        };
        assert_eq!(run_with_config(args, config), ExitCode::SUCCESS);
    }

    #[test]
    fn get_unknown_version_exits_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let config = test_config(tmp.path());
        let args = GetArgs {
            version: Some("nonexistent123".to_string()),
        };
        assert_eq!(run_with_config(args, config), ExitCode::FAILURE);
    }
}
