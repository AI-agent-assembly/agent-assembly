//! Timeline renderer for session traces using ASCII bar charts.

use super::models::TraceEvent;

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
