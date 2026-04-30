//! Data composition — transform API responses into display models.

use super::models::{HealthResponse, RuntimeHealth};

/// Convert a health API response into a display-ready `RuntimeHealth`.
pub fn build_runtime_health(resp: Option<HealthResponse>) -> RuntimeHealth {
    match resp {
        Some(h) => RuntimeHealth {
            reachable: true,
            status: h.status,
        },
        None => RuntimeHealth {
            reachable: false,
            status: "unreachable".to_string(),
        },
    }
}
