//! Data composition — transform API responses into display models.

use super::models::{AgentResponse, AgentRow, HealthResponse, RuntimeHealth};

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

/// Convert API agent responses into display-ready rows.
pub fn build_agent_rows(agents: Vec<AgentResponse>) -> Vec<AgentRow> {
    agents
        .into_iter()
        .map(|a| AgentRow {
            id: a.id,
            name: a.name,
            framework: a.framework,
            status: a.status,
            // Per-agent violation count not yet available from the API.
            violations_today: 0,
        })
        .collect()
}
