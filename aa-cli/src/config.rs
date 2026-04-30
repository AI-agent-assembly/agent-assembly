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
