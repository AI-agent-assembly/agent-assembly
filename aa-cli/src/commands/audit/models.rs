//! Data models for the `aasm audit` command.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// A single audit log entry as returned by `GET /api/v1/logs`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditEntry {
    /// Monotonic sequence number within the session.
    pub seq: u64,
    /// ISO 8601 timestamp of the event.
    pub timestamp: String,
    /// Hex-encoded agent ID.
    pub agent_id: String,
    /// Hex-encoded session ID.
    pub session_id: String,
    /// Audit event type (e.g. `ToolCallIntercepted`, `PolicyViolation`).
    pub event_type: String,
    /// Pre-serialized JSON payload.
    pub payload: String,
}

/// Paginated response envelope from `GET /api/v1/logs`.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedAuditResponse {
    pub items: Vec<AuditEntry>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

/// Export file format for `aasm audit export`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportFormat {
    /// Comma-separated values.
    Csv,
    /// JSON array.
    Json,
}

/// Compliance report format for `aasm audit export --compliance`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ComplianceFormat {
    /// EU AI Act compliance metadata.
    #[value(name = "eu-ai-act")]
    EuAiAct,
    /// SOC 2 compliance metadata.
    #[value(name = "soc2")]
    Soc2,
}

/// Policy decision result for the `--result` filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AuditResult {
    /// Action was allowed by policy.
    Allow,
    /// Action was denied by policy.
    Deny,
    /// Action is pending human approval.
    Pending,
}

impl AuditResult {
    /// Return the string representation used for matching against event types.
    pub fn as_filter_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Pending => "pending",
        }
    }
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Csv => f.write_str("csv"),
            Self::Json => f.write_str("json"),
        }
    }
}

impl std::fmt::Display for ComplianceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EuAiAct => f.write_str("eu-ai-act"),
            Self::Soc2 => f.write_str("soc2"),
        }
    }
}
