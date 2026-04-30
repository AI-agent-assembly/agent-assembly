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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_zero() {
        assert_eq!(format_duration(0), "0ms");
    }

    #[test]
    fn format_duration_typical() {
        assert_eq!(format_duration(142), "142ms");
    }

    #[test]
    fn format_duration_large() {
        assert_eq!(format_duration(60000), "60000ms");
    }

    fn make_event(kind: TraceEventKind, label: &str, duration_ms: u64) -> TraceEvent {
        TraceEvent {
            kind,
            label: label.to_string(),
            duration_ms,
            children: vec![],
            violation_reason: None,
        }
    }

    #[test]
    fn render_event_line_llm() {
        let event = make_event(TraceEventKind::Llm, "GPT-4o", 834);
        let line = render_event_line(&event);
        assert!(line.contains("LLM"));
        assert!(line.contains("GPT-4o"));
        assert!(line.contains("834ms"));
    }

    #[test]
    fn render_event_line_tool_call() {
        let event = make_event(TraceEventKind::ToolCall, "query_db", 12);
        let line = render_event_line(&event);
        assert!(line.contains("TOOL"));
        assert!(line.contains("query_db"));
        assert!(line.contains("12ms"));
    }
}
