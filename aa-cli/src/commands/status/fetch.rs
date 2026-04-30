//! Data composition — transform API responses into display models.

use chrono::Utc;

use super::models::{
    AgentResponse, AgentRow, ApprovalResponse, ApprovalsSummary, BudgetRow, CostResponse,
    HealthResponse, RuntimeHealth,
};

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

/// Compute approvals summary from the raw approval list.
pub fn build_approvals_summary(approvals: &[ApprovalResponse]) -> ApprovalsSummary {
    let pending: Vec<&ApprovalResponse> = approvals.iter().filter(|a| a.status == "pending").collect();
    let pending_count = pending.len();

    let oldest_pending_age = pending
        .iter()
        .filter_map(|a| chrono::DateTime::parse_from_rfc3339(&a.created_at).ok())
        .min()
        .map(|oldest| {
            let age = Utc::now().signed_duration_since(oldest);
            format_duration(age)
        });

    ApprovalsSummary {
        pending_count,
        oldest_pending_age,
    }
}

/// Convert cost API response into a display-ready `BudgetRow`.
pub fn build_budget_row(cost: CostResponse) -> BudgetRow {
    BudgetRow {
        daily_spend_usd: cost.daily_spend_usd,
        monthly_spend_usd: cost.monthly_spend_usd,
        date: cost.date,
    }
}

/// Format a chrono duration into a human-readable string (e.g. `"2h 15m"`).
fn format_duration(dur: chrono::Duration) -> String {
    let total_secs = dur.num_seconds().max(0);
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;

    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}
