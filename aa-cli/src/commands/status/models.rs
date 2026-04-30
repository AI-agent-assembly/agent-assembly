//! Data models for the `aasm status` command.

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
