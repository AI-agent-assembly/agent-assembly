//! Tree renderer for session traces using box-drawing characters.

use colored::Colorize;

use super::models::{SessionTrace, TraceEvent, TraceEventKind};

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
///
/// Policy denials are highlighted in red with the violation reason appended.
pub fn render_event_line(event: &TraceEvent) -> String {
    let line = format!(
        "{} {}  {}",
        event_icon(&event.kind),
        event.label,
        format_duration(event.duration_ms),
    );

    if event.kind == TraceEventKind::PolicyDeny {
        let reason = event
            .violation_reason
            .as_deref()
            .unwrap_or("no reason provided");
        format!("{}", format!("{line}  ({reason})").red())
    } else {
        line
    }
}

/// Recursively render a list of events as a tree with box-drawing prefixes.
///
/// `prefix` is the indentation string inherited from the parent level.
fn render_tree_recursive(events: &[TraceEvent], prefix: &str, output: &mut String) {
    let count = events.len();
    for (i, event) in events.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└─ " } else { "├─ " };
        let child_prefix = if is_last {
            format!("{prefix}   ")
        } else {
            format!("{prefix}│  ")
        };

        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&render_event_line(event));
        output.push('\n');

        if !event.children.is_empty() {
            render_tree_recursive(&event.children, &child_prefix, output);
        }
    }
}

/// Render a full session trace as an indented tree with box-drawing characters.
pub fn render_tree(trace: &SessionTrace) -> String {
    let mut output = format!("Trace: {}\n", trace.session_id);
    render_tree_recursive(&trace.events, "", &mut output);
    output
}
