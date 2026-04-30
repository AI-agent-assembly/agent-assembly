//! Data composition — transform API responses into display models.

use chrono::Utc;

use super::client::StatusClient;
use super::models::{
    AgentResponse, AgentRow, ApprovalResponse, ApprovalsSummary, BudgetRow, CostResponse, HealthResponse,
    RuntimeHealth, StatusSnapshot,
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

/// Fetch all status data from the gateway in parallel and compose a `StatusSnapshot`.
pub async fn fetch_all(client: &StatusClient) -> StatusSnapshot {
    let (health_result, agents_result, approvals_result, costs_result) = tokio::join!(
        client.check_health(),
        client.list_agents(),
        client.list_approvals(),
        client.get_costs(),
    );

    let runtime = build_runtime_health(health_result.ok());
    let agents = build_agent_rows(agents_result.unwrap_or_default());
    let approvals = build_approvals_summary(&approvals_result.unwrap_or_default());
    let budget = match costs_result {
        Ok(c) => build_budget_row(c),
        Err(_) => BudgetRow {
            daily_spend_usd: "--".to_string(),
            monthly_spend_usd: None,
            date: "--".to_string(),
        },
    };

    StatusSnapshot {
        runtime,
        agents,
        approvals,
        budget,
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn build_runtime_health_reachable() {
        let resp = Some(HealthResponse {
            status: "ok".to_string(),
        });
        let health = build_runtime_health(resp);
        assert!(health.reachable);
        assert_eq!(health.status, "ok");
    }

    #[test]
    fn build_runtime_health_unreachable() {
        let health = build_runtime_health(None);
        assert!(!health.reachable);
        assert_eq!(health.status, "unreachable");
    }

    #[test]
    fn build_agent_rows_maps_fields() {
        let agents = vec![AgentResponse {
            id: "abc".to_string(),
            name: "test-agent".to_string(),
            framework: "langgraph".to_string(),
            version: "1.0.0".to_string(),
            status: "Running".to_string(),
            tool_names: vec!["tool_a".to_string()],
            metadata: BTreeMap::new(),
        }];
        let rows = build_agent_rows(agents);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "abc");
        assert_eq!(rows[0].name, "test-agent");
        assert_eq!(rows[0].framework, "langgraph");
        assert_eq!(rows[0].status, "Running");
        assert_eq!(rows[0].violations_today, 0);
    }

    #[test]
    fn build_approvals_summary_with_pending() {
        let approvals = vec![
            ApprovalResponse {
                id: "ap-1".to_string(),
                agent_id: "a1".to_string(),
                action: "refund".to_string(),
                reason: "amount".to_string(),
                status: "pending".to_string(),
                created_at: "2026-04-30T08:00:00Z".to_string(),
            },
            ApprovalResponse {
                id: "ap-2".to_string(),
                agent_id: "a2".to_string(),
                action: "delete".to_string(),
                reason: "test".to_string(),
                status: "approved".to_string(),
                created_at: "2026-04-30T07:00:00Z".to_string(),
            },
        ];
        let summary = build_approvals_summary(&approvals);
        assert_eq!(summary.pending_count, 1);
        assert!(summary.oldest_pending_age.is_some());
    }

    #[test]
    fn build_approvals_summary_no_pending() {
        let approvals = vec![ApprovalResponse {
            id: "ap-1".to_string(),
            agent_id: "a1".to_string(),
            action: "refund".to_string(),
            reason: "done".to_string(),
            status: "approved".to_string(),
            created_at: "2026-04-30T08:00:00Z".to_string(),
        }];
        let summary = build_approvals_summary(&approvals);
        assert_eq!(summary.pending_count, 0);
        assert!(summary.oldest_pending_age.is_none());
    }

    #[test]
    fn format_duration_minutes_only() {
        let dur = chrono::Duration::minutes(5);
        assert_eq!(format_duration(dur), "5m");
    }

    #[test]
    fn format_duration_hours_and_minutes() {
        let dur = chrono::Duration::hours(2) + chrono::Duration::minutes(15);
        assert_eq!(format_duration(dur), "2h 15m");
    }

    #[test]
    fn format_duration_days() {
        let dur = chrono::Duration::days(1) + chrono::Duration::hours(3);
        assert_eq!(format_duration(dur), "1d 3h");
    }
}
