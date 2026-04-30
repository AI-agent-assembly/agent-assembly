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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_entry_deserializes() {
        let json = r#"{
            "seq": 42,
            "timestamp": "2026-04-30T10:00:00Z",
            "agent_id": "aa001",
            "session_id": "sess001",
            "event_type": "PolicyViolation",
            "payload": "{\"tool\":\"bash\",\"result\":\"deny\"}"
        }"#;
        let entry: AuditEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.seq, 42);
        assert_eq!(entry.timestamp, "2026-04-30T10:00:00Z");
        assert_eq!(entry.agent_id, "aa001");
        assert_eq!(entry.session_id, "sess001");
        assert_eq!(entry.event_type, "PolicyViolation");
        assert!(entry.payload.contains("deny"));
    }

    #[test]
    fn paginated_audit_response_deserializes() {
        let json = r#"{
            "items": [{
                "seq": 0,
                "timestamp": "2026-04-30T10:00:00Z",
                "agent_id": "aa001",
                "session_id": "sess001",
                "event_type": "ToolCallIntercepted",
                "payload": "{}"
            }],
            "page": 1,
            "per_page": 50,
            "total": 1
        }"#;
        let resp: PaginatedAuditResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.page, 1);
        assert_eq!(resp.per_page, 50);
        assert_eq!(resp.total, 1);
    }

    #[test]
    fn audit_result_as_filter_str() {
        assert_eq!(AuditResult::Allow.as_filter_str(), "allow");
        assert_eq!(AuditResult::Deny.as_filter_str(), "deny");
        assert_eq!(AuditResult::Pending.as_filter_str(), "pending");
    }

    #[test]
    fn audit_result_value_variants_contains_all() {
        let variants = AuditResult::value_variants();
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn export_format_display() {
        assert_eq!(ExportFormat::Csv.to_string(), "csv");
        assert_eq!(ExportFormat::Json.to_string(), "json");
    }

    #[test]
    fn export_format_value_variants_contains_all() {
        let variants = ExportFormat::value_variants();
        assert_eq!(variants.len(), 2);
    }

    #[test]
    fn compliance_format_display() {
        assert_eq!(ComplianceFormat::EuAiAct.to_string(), "eu-ai-act");
        assert_eq!(ComplianceFormat::Soc2.to_string(), "soc2");
    }

    #[test]
    fn compliance_format_value_variants_contains_all() {
        let variants = ComplianceFormat::value_variants();
        assert_eq!(variants.len(), 2);
    }

    #[test]
    fn audit_entry_round_trip_serialization() {
        let entry = AuditEntry {
            seq: 1,
            timestamp: "2026-04-30T10:00:00Z".to_string(),
            agent_id: "aa001".to_string(),
            session_id: "sess001".to_string(),
            event_type: "ToolCallIntercepted".to_string(),
            payload: "{}".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.seq, entry.seq);
        assert_eq!(parsed.agent_id, entry.agent_id);
    }
}
