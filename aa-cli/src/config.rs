//! Configuration file management for the `aasm` CLI.
//!
//! Config is stored at `~/.aa/config.yaml` and contains named contexts,
//! each with an API URL and optional API key.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::CliError;

/// A named API context (e.g. "production", "staging").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Base URL of the Agent Assembly API (e.g. `http://localhost:8080`).
    pub api_url: String,
    /// Optional API key for authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Top-level CLI configuration file schema (`~/.aa/config.yaml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Name of the default context to use when `--context` is not specified.
    #[serde(default)]
    pub default_context: Option<String>,
    /// Named contexts mapping (e.g. `{ "production": { api_url: "..." } }`).
    #[serde(default)]
    pub contexts: BTreeMap<String, ContextConfig>,
}

/// Return the config directory path (`~/.aa/`).
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("cannot determine home directory")
        .join(".aa")
}

/// Return the config file path (`~/.aa/config.yaml`).
pub fn config_path() -> PathBuf {
    config_dir().join("config.yaml")
}

/// Load the CLI configuration from `~/.aa/config.yaml`.
///
/// Returns a default (empty) config if the file does not exist.
pub fn load() -> Result<CliConfig, CliError> {
    let path = config_path();
    if !path.exists() {
        return Ok(CliConfig {
            default_context: None,
            contexts: BTreeMap::new(),
        });
    }
    let contents = std::fs::read_to_string(&path).map_err(|e| CliError::Config {
        path: path.clone(),
        source: e,
    })?;
    let config: CliConfig = serde_yaml::from_str(&contents)?;
    Ok(config)
}

/// Save the CLI configuration to `~/.aa/config.yaml`.
///
/// Creates the `~/.aa/` directory if it does not exist.
pub fn save(config: &CliConfig) -> Result<(), CliError> {
    let dir = config_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| CliError::Config {
            path: dir.clone(),
            source: e,
        })?;
    }
    let path = config_path();
    let yaml = serde_yaml::to_string(config)?;
    std::fs::write(&path, yaml).map_err(|e| CliError::Config { path, source: e })?;
    Ok(())
}
