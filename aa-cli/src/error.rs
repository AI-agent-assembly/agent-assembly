//! Unified error type for the `aasm` CLI.

use std::path::PathBuf;

/// Errors that can occur during CLI execution.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// Failed to read or write the configuration file.
    #[error("config error at {path}: {source}")]
    Config {
        path: PathBuf,
        source: std::io::Error,
    },

    /// The configuration file contains invalid YAML.
    #[error("invalid config YAML: {0}")]
    ConfigParse(#[from] serde_yaml::Error),

    /// The requested named context does not exist.
    #[error("context not found: {0}")]
    ContextNotFound(String),

    /// An HTTP request to the gateway failed.
    #[error("API request failed: {0}")]
    Api(#[from] reqwest::Error),

    /// Generic I/O error.
    #[error("{0}")]
    Io(#[from] std::io::Error),
}
