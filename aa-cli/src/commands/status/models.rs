//! Data models for the `aasm status` command.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// API response from `GET /api/v1/health`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthResponse {
    /// Liveness status string, always `"ok"` when the service is running.
    pub status: String,
}

/// Computed runtime health for display.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeHealth {
    /// Whether the API gateway is reachable.
    pub reachable: bool,
    /// Status string from the health endpoint (e.g. `"ok"`).
    pub status: String,
}

/// API response item from `GET /api/v1/agents`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub version: String,
    pub status: String,
    pub tool_names: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Flattened agent row for tabular display.
#[derive(Debug, Clone, Serialize)]
pub struct AgentRow {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub status: String,
    pub violations_today: u64,
}
