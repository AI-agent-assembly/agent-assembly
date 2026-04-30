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
