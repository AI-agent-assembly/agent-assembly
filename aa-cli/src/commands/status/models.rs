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

/// API response item from `GET /api/v1/approvals`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApprovalResponse {
    pub id: String,
    pub agent_id: String,
    pub action: String,
    pub reason: String,
    pub status: String,
    pub created_at: String,
}

/// Computed approvals summary for display.
#[derive(Debug, Clone, Serialize)]
pub struct ApprovalsSummary {
    /// Number of approvals currently in `"pending"` status.
    pub pending_count: usize,
    /// Human-readable age of the oldest pending approval (e.g. `"2h 15m"`).
    pub oldest_pending_age: Option<String>,
}

/// API response from `GET /api/v1/costs`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CostResponse {
    pub daily_spend_usd: String,
    pub monthly_spend_usd: Option<String>,
    pub date: String,
}

/// Per-agent budget row for display.
#[derive(Debug, Clone, Serialize)]
pub struct BudgetRow {
    /// Total daily spend in USD (aggregated, since per-agent is not yet available).
    pub daily_spend_usd: String,
    /// Monthly spend if available.
    pub monthly_spend_usd: Option<String>,
    /// Reporting date.
    pub date: String,
}

/// Paginated API response wrapper.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

/// Complete status snapshot combining all sections.
#[derive(Debug, Clone, Serialize)]
pub struct StatusSnapshot {
    pub runtime: RuntimeHealth,
    pub agents: Vec<AgentRow>,
    pub approvals: ApprovalsSummary,
    pub budget: BudgetRow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_deserializes() {
        let json = r#"{"status":"ok"}"#;
        let resp: HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "ok");
    }

    #[test]
    fn agent_response_deserializes() {
        let json = r#"{
            "id": "abc123",
            "name": "support-agent",
            "framework": "langgraph",
            "version": "1.0.0",
            "status": "Running",
            "tool_names": ["query_db", "send_slack"],
            "metadata": {"team": "support"}
        }"#;
        let resp: AgentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "abc123");
        assert_eq!(resp.name, "support-agent");
        assert_eq!(resp.framework, "langgraph");
        assert_eq!(resp.tool_names.len(), 2);
        assert_eq!(resp.metadata.get("team").unwrap(), "support");
    }

    #[test]
    fn approval_response_deserializes() {
        let json = r#"{
            "id": "ap-001",
            "agent_id": "abc123",
            "action": "process_refund",
            "reason": "amount exceeds $100",
            "status": "pending",
            "created_at": "2026-04-30T10:00:00Z"
        }"#;
        let resp: ApprovalResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "ap-001");
        assert_eq!(resp.status, "pending");
        assert_eq!(resp.created_at, "2026-04-30T10:00:00Z");
    }

    #[test]
    fn cost_response_deserializes() {
        let json = r#"{
            "daily_spend_usd": "8.10",
            "monthly_spend_usd": "142.50",
            "date": "2026-04-30"
        }"#;
        let resp: CostResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.daily_spend_usd, "8.10");
        assert_eq!(resp.monthly_spend_usd.as_deref(), Some("142.50"));
        assert_eq!(resp.date, "2026-04-30");
    }

    #[test]
    fn cost_response_deserializes_without_monthly() {
        let json = r#"{"daily_spend_usd": "0.00", "date": "2026-04-30"}"#;
        let resp: CostResponse = serde_json::from_str(json).unwrap();
        assert!(resp.monthly_spend_usd.is_none());
    }
}
