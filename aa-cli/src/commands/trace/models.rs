//! Data models for trace session visualization.

use serde::{Deserialize, Serialize};

/// The kind of event recorded in a trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceEventKind {
    /// An LLM inference call.
    Llm,
    /// A tool invocation by the agent.
    ToolCall,
    /// The result returned by a tool.
    ToolResult,
    /// A policy evaluation that allowed the action.
    PolicyAllow,
    /// A policy evaluation that denied the action.
    PolicyDeny,
}

/// A single event within a trace session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// What kind of event this is.
    pub kind: TraceEventKind,
    /// Human-readable label (e.g. tool name, model name).
    pub label: String,
    /// How long this event took in milliseconds.
    pub duration_ms: u64,
    /// Nested child events (e.g. tool calls within an LLM step).
    #[serde(default)]
    pub children: Vec<TraceEvent>,
    /// If the event was a policy denial, the reason why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violation_reason: Option<String>,
}

/// A complete trace for one agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTrace {
    /// Unique identifier for the session.
    pub session_id: String,
    /// Top-level events in the session (in chronological order).
    pub events: Vec<TraceEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_event_kind_serializes_to_snake_case() {
        assert_eq!(serde_json::to_string(&TraceEventKind::Llm).unwrap(), "\"llm\"");
        assert_eq!(serde_json::to_string(&TraceEventKind::ToolCall).unwrap(), "\"tool_call\"");
        assert_eq!(serde_json::to_string(&TraceEventKind::ToolResult).unwrap(), "\"tool_result\"");
        assert_eq!(serde_json::to_string(&TraceEventKind::PolicyAllow).unwrap(), "\"policy_allow\"");
        assert_eq!(serde_json::to_string(&TraceEventKind::PolicyDeny).unwrap(), "\"policy_deny\"");
    }

    #[test]
    fn trace_event_kind_deserializes_from_snake_case() {
        let kind: TraceEventKind = serde_json::from_str("\"tool_call\"").unwrap();
        assert_eq!(kind, TraceEventKind::ToolCall);
    }

    #[test]
    fn trace_event_round_trip() {
        let event = TraceEvent {
            kind: TraceEventKind::Llm,
            label: "GPT-4o".to_string(),
            duration_ms: 834,
            children: vec![],
            violation_reason: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: TraceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind, TraceEventKind::Llm);
        assert_eq!(parsed.label, "GPT-4o");
        assert_eq!(parsed.duration_ms, 834);
        assert!(parsed.children.is_empty());
        assert!(parsed.violation_reason.is_none());
    }

    #[test]
    fn trace_event_violation_reason_included_when_present() {
        let event = TraceEvent {
            kind: TraceEventKind::PolicyDeny,
            label: "process_refund".to_string(),
            duration_ms: 1,
            children: vec![],
            violation_reason: Some("amount exceeds limit".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("violation_reason"));
        assert!(json.contains("amount exceeds limit"));
    }

    #[test]
    fn trace_event_violation_reason_omitted_when_none() {
        let event = TraceEvent {
            kind: TraceEventKind::ToolCall,
            label: "query_db".to_string(),
            duration_ms: 12,
            children: vec![],
            violation_reason: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("violation_reason"));
    }
}
