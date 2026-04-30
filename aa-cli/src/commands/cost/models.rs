//! Data models for the `aasm cost` command.

use serde::{Deserialize, Serialize};

/// Per-agent cost entry from the API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentCostEntry {
    pub agent_id: String,
    pub daily_spend_usd: String,
    pub monthly_spend_usd: Option<String>,
    pub date: String,
}

/// API response from `GET /api/v1/costs`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CostResponse {
    pub daily_spend_usd: String,
    pub monthly_spend_usd: Option<String>,
    pub date: String,
    #[serde(default)]
    pub daily_limit_usd: Option<String>,
    #[serde(default)]
    pub monthly_limit_usd: Option<String>,
    #[serde(default)]
    pub agents: Vec<AgentCostEntry>,
}

/// Display model for cost summary output.
#[derive(Debug, Clone, Serialize)]
pub struct CostSummaryDisplay {
    pub daily_spend_usd: String,
    pub monthly_spend_usd: Option<String>,
    pub date: String,
    pub daily_limit_usd: Option<String>,
    pub monthly_limit_usd: Option<String>,
    pub agents: Vec<AgentCostEntry>,
}

/// Display model for cost forecast output.
#[derive(Debug, Clone, Serialize)]
pub struct CostForecastDisplay {
    pub date: String,
    pub day_of_month: u32,
    pub days_in_month: u32,
    pub current_daily_spend: String,
    pub projected_monthly_spend: String,
    pub monthly_limit_usd: Option<String>,
    pub projected_utilization_pct: Option<String>,
}

impl From<CostResponse> for CostSummaryDisplay {
    fn from(resp: CostResponse) -> Self {
        Self {
            daily_spend_usd: resp.daily_spend_usd,
            monthly_spend_usd: resp.monthly_spend_usd,
            date: resp.date,
            daily_limit_usd: resp.daily_limit_usd,
            monthly_limit_usd: resp.monthly_limit_usd,
            agents: resp.agents,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_response_deserializes_minimal() {
        let json = r#"{"daily_spend_usd":"0.00","date":"2026-04-30"}"#;
        let resp: CostResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.daily_spend_usd, "0.00");
        assert!(resp.monthly_spend_usd.is_none());
        assert!(resp.daily_limit_usd.is_none());
        assert!(resp.agents.is_empty());
    }

    #[test]
    fn cost_response_deserializes_full() {
        let json = r#"{
            "daily_spend_usd": "8.10",
            "monthly_spend_usd": "142.50",
            "date": "2026-04-30",
            "daily_limit_usd": "50.00",
            "monthly_limit_usd": "500.00",
            "agents": [{
                "agent_id": "abc123",
                "daily_spend_usd": "4.00",
                "monthly_spend_usd": "80.00",
                "date": "2026-04-30"
            }]
        }"#;
        let resp: CostResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.daily_limit_usd.as_deref(), Some("50.00"));
        assert_eq!(resp.agents.len(), 1);
        assert_eq!(resp.agents[0].agent_id, "abc123");
    }
}
