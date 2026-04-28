//! `aasm policy` subcommands — apply, history, rollback, diff.

use aa_gateway::policy::history::{FsHistoryStore, HistoryConfig, PolicyHistoryStore};
use clap::Subcommand;
use std::path::PathBuf;

/// Policy management subcommands.
#[derive(Subcommand)]
pub enum PolicyCommand {
    /// Apply a policy YAML file and save it to version history.
    Apply {
        /// Path to the policy YAML file.
        file: PathBuf,
        /// Identity of the person or system applying the policy.
        #[arg(long)]
        applied_by: Option<String>,
    },
    /// List recent policy versions.
    History {
        /// Maximum number of versions to show.
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,
    },
    /// Roll back to a previous policy version.
    Rollback {
        /// Version identifier (SHA-256 prefix) to roll back to.
        version: String,
    },
    /// Show the diff between two policy versions.
    Diff {
        /// First version identifier (SHA-256 prefix).
        version_a: String,
        /// Second version identifier (SHA-256 prefix).
        version_b: String,
    },
}

/// Execute a policy subcommand.
pub async fn run(cmd: PolicyCommand) -> Result<(), Box<dyn std::error::Error>> {
    let store = FsHistoryStore::new(HistoryConfig::default_config());

    match cmd {
        PolicyCommand::Apply { file, applied_by } => {
            let yaml = std::fs::read_to_string(&file)?;
            // Validate before saving
            aa_gateway::policy::PolicyValidator::from_yaml(&yaml)
                .map_err(|errs| format!("Policy validation failed: {:?}", errs))?;
            let meta = store.save(&yaml, applied_by.as_deref()).await?;
            println!("Policy applied successfully.");
            println!("  Version:    {}", &meta.sha256[..12]);
            println!("  Timestamp:  {}", meta.timestamp);
            println!("  SHA-256:    {}", meta.sha256);
            if let Some(by) = &meta.applied_by {
                println!("  Applied by: {}", by);
            }
            Ok(())
        }
        PolicyCommand::History { limit } => {
            let versions = store.list(limit).await?;
            if versions.is_empty() {
                println!("No policy versions found.");
                return Ok(());
            }
            println!(
                "{:<14} {:<26} {:<12} {:<10}",
                "VERSION", "TIMESTAMP", "APPLIED BY", "ROLLBACK"
            );
            println!("{}", "-".repeat(64));
            for meta in versions {
                let version_short = &meta.sha256[..meta.sha256.len().min(12)];
                let applied_by = meta.applied_by.as_deref().unwrap_or("-");
                let rollback = if meta.is_rollback { "yes" } else { "-" };
                println!(
                    "{:<14} {:<26} {:<12} {:<10}",
                    version_short, meta.timestamp, applied_by, rollback
                );
            }
            Ok(())
        }
        PolicyCommand::Rollback { version } => {
            let meta = store.rollback(&version).await?;
            println!("Rolled back successfully.");
            println!("  New version:    {}", &meta.sha256[..12]);
            println!("  Timestamp:      {}", meta.timestamp);
            println!(
                "  Rolled back to: {}",
                meta.rollback_target.as_deref().unwrap_or("unknown")
            );
            Ok(())
        }
        PolicyCommand::Diff { version_a, version_b } => {
            let diff = store.diff(&version_a, &version_b).await?;
            if diff.lines().count() <= 2 {
                println!("No differences between the two versions.");
            } else {
                print!("{}", diff);
            }
            Ok(())
        }
    }
}
