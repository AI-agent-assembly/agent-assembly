//! Timeline renderer for session traces using ASCII bar charts.

use super::models::{TraceEvent, TraceEventKind};
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
