//! Data models for the `aasm status` command.

use serde::{Deserialize, Serialize};

/// API response from `GET /api/v1/health`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthResponse {
    /// Liveness status string, always `"ok"` when the service is running.
    pub status: String,
}
