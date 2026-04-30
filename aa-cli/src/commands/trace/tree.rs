//! Tree renderer for session traces using box-drawing characters.

use super::models::{TraceEvent, TraceEventKind};

/// Format a duration in milliseconds into a human-readable string.
///
/// Examples: `0ms`, `142ms`, `1200ms`, `60000ms`.
pub fn format_duration(duration_ms: u64) -> String {
    format!("{duration_ms}ms")
}

/// Return the icon for a given event kind.
fn event_icon(kind: &TraceEventKind) -> &'static str {
    match kind {
        TraceEventKind::Llm => "●  LLM",
        TraceEventKind::ToolCall => "●  TOOL",
        TraceEventKind::ToolResult => "←  RESULT",
        TraceEventKind::PolicyAllow => "✅ ALLOW",
        TraceEventKind::PolicyDeny => "❌ DENY",
    }
}

/// Render a single event as a one-line string (without tree prefix).
pub fn render_event_line(event: &TraceEvent) -> String {
    format!(
        "{} {}  {}",
        event_icon(&event.kind),
        event.label,
        format_duration(event.duration_ms),
    )
}
