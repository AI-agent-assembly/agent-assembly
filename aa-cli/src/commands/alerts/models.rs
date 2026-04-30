//! Data models for the `aasm alerts` subcommands.

use std::fmt;

use comfy_table::Color;
use serde::{Deserialize, Serialize};

/// JSON representation of a governance alert returned by the gateway API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertResponse {
    /// Unique alert identifier.
    pub id: String,
    /// Agent ID that triggered the alert (if applicable).
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Alert severity level.
    pub severity: String,
    /// Alert category (e.g. "budget", "policy_violation", "anomaly").
    #[serde(default)]
    pub category: String,
    /// Human-readable alert message.
    pub message: String,
    /// Alert status (unresolved, acknowledged, resolved).
    #[serde(default = "default_status")]
    pub status: String,
    /// ISO 8601 timestamp when the alert was created.
    #[serde(alias = "timestamp")]
    pub created_at: String,
    /// ISO 8601 timestamp when the alert was last updated.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Additional context payload.
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

fn default_status() -> String {
    "unresolved".to_string()
}

/// Request body for `POST /api/v1/alerts/:id/resolve`.
#[derive(Debug, Serialize)]
pub struct ResolveAlertRequest {
    /// Optional resolution note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Known severity levels with associated terminal colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
    Unknown,
}

impl AlertSeverity {
    /// Parse a severity string (case-insensitive).
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" => Self::Critical,
            "warning" => Self::Warning,
            "info" => Self::Info,
            _ => Self::Unknown,
        }
    }

    /// Terminal color for this severity level.
    pub fn color(self) -> Color {
        match self {
            Self::Critical => Color::Red,
            Self::Warning => Color::Yellow,
            Self::Info => Color::White,
            Self::Unknown => Color::Reset,
        }
    }
}

impl fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Known alert status values with associated terminal colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertStatusKind {
    Unresolved,
    Acknowledged,
    Resolved,
    Unknown,
}

impl AlertStatusKind {
    /// Parse a status string (case-insensitive).
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "unresolved" => Self::Unresolved,
            "acknowledged" => Self::Acknowledged,
            "resolved" => Self::Resolved,
            _ => Self::Unknown,
        }
    }

    /// Terminal color for this status.
    pub fn color(self) -> Color {
        match self {
            Self::Unresolved => Color::Red,
            Self::Acknowledged => Color::Yellow,
            Self::Resolved => Color::Green,
            Self::Unknown => Color::Reset,
        }
    }
}

impl fmt::Display for AlertStatusKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unresolved => write!(f, "unresolved"),
            Self::Acknowledged => write!(f, "acknowledged"),
            Self::Resolved => write!(f, "resolved"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_from_str_case_insensitive() {
        assert_eq!(AlertSeverity::from_str("Critical"), AlertSeverity::Critical);
        assert_eq!(AlertSeverity::from_str("WARNING"), AlertSeverity::Warning);
        assert_eq!(AlertSeverity::from_str("info"), AlertSeverity::Info);
        assert_eq!(AlertSeverity::from_str("other"), AlertSeverity::Unknown);
    }

    #[test]
    fn severity_colors() {
        assert_eq!(AlertSeverity::Critical.color(), Color::Red);
        assert_eq!(AlertSeverity::Warning.color(), Color::Yellow);
        assert_eq!(AlertSeverity::Info.color(), Color::White);
        assert_eq!(AlertSeverity::Unknown.color(), Color::Reset);
    }

    #[test]
    fn status_from_str_case_insensitive() {
        assert_eq!(AlertStatusKind::from_str("unresolved"), AlertStatusKind::Unresolved);
        assert_eq!(AlertStatusKind::from_str("ACKNOWLEDGED"), AlertStatusKind::Acknowledged);
        assert_eq!(AlertStatusKind::from_str("Resolved"), AlertStatusKind::Resolved);
        assert_eq!(AlertStatusKind::from_str("other"), AlertStatusKind::Unknown);
    }

    #[test]
    fn status_colors() {
        assert_eq!(AlertStatusKind::Unresolved.color(), Color::Red);
        assert_eq!(AlertStatusKind::Acknowledged.color(), Color::Yellow);
        assert_eq!(AlertStatusKind::Resolved.color(), Color::Green);
        assert_eq!(AlertStatusKind::Unknown.color(), Color::Reset);
    }

    #[test]
    fn alert_response_deserializes_with_defaults() {
        let json = r#"{
            "id": "alert-001",
            "severity": "warning",
            "message": "Budget threshold exceeded",
            "timestamp": "2026-04-30T10:00:00Z"
        }"#;
        let alert: AlertResponse = serde_json::from_str(json).unwrap();
        assert_eq!(alert.id, "alert-001");
        assert_eq!(alert.status, "unresolved");
        assert_eq!(alert.created_at, "2026-04-30T10:00:00Z");
        assert!(alert.agent_id.is_none());
        assert!(alert.context.is_none());
    }

    #[test]
    fn alert_response_round_trip() {
        let alert = AlertResponse {
            id: "alert-002".to_string(),
            agent_id: Some("agent-abc".to_string()),
            severity: "critical".to_string(),
            category: "policy_violation".to_string(),
            message: "Blocked tool call".to_string(),
            status: "resolved".to_string(),
            created_at: "2026-04-30T10:00:00Z".to_string(),
            updated_at: Some("2026-04-30T11:00:00Z".to_string()),
            context: Some(serde_json::json!({"tool": "shell_exec"})),
        };
        let json = serde_json::to_string(&alert).unwrap();
        let parsed: AlertResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "alert-002");
        assert_eq!(parsed.status, "resolved");
        assert_eq!(parsed.context.unwrap()["tool"], "shell_exec");
    }

    #[test]
    fn resolve_request_skips_none_reason() {
        let req = ResolveAlertRequest { reason: None };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn resolve_request_includes_reason() {
        let req = ResolveAlertRequest {
            reason: Some("False positive".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("False positive"));
    }

    #[test]
    fn severity_display() {
        assert_eq!(format!("{}", AlertSeverity::Critical), "critical");
        assert_eq!(format!("{}", AlertSeverity::Warning), "warning");
        assert_eq!(format!("{}", AlertSeverity::Info), "info");
    }

    #[test]
    fn status_display() {
        assert_eq!(format!("{}", AlertStatusKind::Unresolved), "unresolved");
        assert_eq!(format!("{}", AlertStatusKind::Acknowledged), "acknowledged");
        assert_eq!(format!("{}", AlertStatusKind::Resolved), "resolved");
    }
}
