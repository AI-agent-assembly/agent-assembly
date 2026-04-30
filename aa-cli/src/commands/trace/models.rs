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
