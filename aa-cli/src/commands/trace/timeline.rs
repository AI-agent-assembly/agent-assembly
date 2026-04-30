//! Timeline renderer for session traces using ASCII bar charts.

use super::models::{SessionTrace, TraceEvent, TraceEventKind};
use super::tree::format_duration;

/// Find the maximum duration among a flat list of events.
pub fn compute_max_duration(events: &[TraceEvent]) -> u64 {
    events.iter().map(|e| e.duration_ms).max().unwrap_or(0)
}

/// Render an ASCII bar whose width is proportional to `duration_ms` relative to `max_duration`.
///
/// Returns a string of `█` characters up to `max_width` wide.
pub fn render_bar(duration_ms: u64, max_duration: u64, max_width: usize) -> String {
    if max_duration == 0 {
        return String::new();
    }
    let width = ((duration_ms as f64 / max_duration as f64) * max_width as f64).round() as usize;
    let width = width.max(if duration_ms > 0 { 1 } else { 0 });
    "█".repeat(width)
}

/// Label prefix for timeline rows.
fn timeline_label(event: &TraceEvent) -> String {
    let kind_tag = match event.kind {
        TraceEventKind::Llm => "LLM",
        TraceEventKind::ToolCall => "TOOL",
        TraceEventKind::ToolResult => "RESULT",
        TraceEventKind::PolicyAllow => "ALLOW",
        TraceEventKind::PolicyDeny => "DENY",
    };
    format!("{kind_tag:<6} {:<20}", event.label)
}

/// Render one row of the timeline: label | bar | duration.
pub fn render_timeline_row(
    event: &TraceEvent,
    max_duration: u64,
    bar_width: usize,
) -> String {
    let label = timeline_label(event);
    let bar = render_bar(event.duration_ms, max_duration, bar_width);
    format!("{label} {bar:<bar_width$}  {}", format_duration(event.duration_ms))
}

/// Flatten a trace tree into a depth-first list of events (ignoring nesting).
fn flatten_events(events: &[TraceEvent]) -> Vec<&TraceEvent> {
    let mut flat = Vec::new();
    for event in events {
        flat.push(event);
        flat.extend(flatten_events(&event.children));
    }
    flat
}

/// Render a full session trace as a horizontal ASCII timeline.
///
/// `max_width` controls the total line width (default 80).
pub fn render_timeline(trace: &SessionTrace, max_width: usize) -> String {
    let mut output = format!("Timeline: {}\n", trace.session_id);

    let flat = flatten_events(&trace.events);
    if flat.is_empty() {
        output.push_str("(no events)\n");
        return output;
    }

    let max_duration = compute_max_duration(
        &flat.iter().map(|e| (*e).clone()).collect::<Vec<_>>(),
    );
    // Reserve ~30 chars for label, ~10 for duration suffix
    let bar_width = max_width.saturating_sub(40);

    for event in &flat {
        output.push_str(&render_timeline_row(event, max_duration, bar_width));
        output.push('\n');
    }
    output
}
