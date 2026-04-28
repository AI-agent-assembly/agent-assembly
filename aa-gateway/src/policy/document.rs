//! Validated, strongly-typed policy document types for aa-gateway.

/// Validated network egress policy.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkPolicy {
    /// Domain glob patterns the agent may connect to.
    pub allowlist: Vec<String>,
}

/// Validated active-hours window.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveHours {
    /// Window start in `HH:MM` 24-hour format.
    pub start: String,
    /// Window end in `HH:MM` 24-hour format.
    pub end: String,
    /// IANA timezone name.
    pub timezone: String,
}

/// Validated schedule policy.
#[derive(Debug, Clone, PartialEq)]
pub struct SchedulePolicy {
    /// Optional time window during which the agent is permitted to run.
    pub active_hours: Option<ActiveHours>,
}

/// Validated spend budget policy.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetPolicy {
    /// Maximum USD spend per calendar day; `None` means no limit.
    pub daily_limit_usd: Option<f64>,
}

/// Validated data / PII policy.
#[derive(Debug, Clone, PartialEq)]
pub struct DataPolicy {
    /// Compiled regex patterns for PII / credential detection.
    pub sensitive_patterns: Vec<String>,
}

/// Validated per-tool policy entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolPolicy {
    /// Whether this tool is permitted.
    pub allow: bool,
    /// Max calls per hour; `None` means unlimited.
    pub limit_per_hour: Option<u32>,
    /// CEL expression that triggers human-in-the-loop approval.
    pub requires_approval_if: Option<String>,
}

/// Fully validated policy document produced by [`super::validator::PolicyValidator`].
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyDocument {
    /// Schema version string.
    pub version: Option<String>,
    /// Network egress policy.
    pub network: Option<NetworkPolicy>,
    /// Schedule / active-hours policy.
    pub schedule: Option<SchedulePolicy>,
    /// Spend budget policy.
    pub budget: Option<BudgetPolicy>,
    /// Data / PII policy.
    pub data: Option<DataPolicy>,
    /// Per-tool policies keyed by tool name.
    pub tools: std::collections::HashMap<String, ToolPolicy>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_document_default_tools_is_empty_map() {
        let doc = PolicyDocument {
            version: None,
            network: None,
            schedule: None,
            budget: None,
            data: None,
            tools: std::collections::HashMap::new(),
        };
        assert!(doc.tools.is_empty());
    }

    #[test]
    fn network_policy_stores_allowlist() {
        let np = NetworkPolicy {
            allowlist: vec!["api.openai.com".to_string()],
        };
        assert_eq!(np.allowlist.len(), 1);
    }

    #[test]
    fn tool_policy_allow_defaults() {
        let tp = ToolPolicy {
            allow: true,
            limit_per_hour: None,
            requires_approval_if: None,
        };
        assert!(tp.allow);
        assert!(tp.limit_per_hour.is_none());
    }
}
