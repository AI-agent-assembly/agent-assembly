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
