//! Timeline renderer for session traces using ASCII bar charts.

use super::models::TraceEvent;

/// Find the maximum duration among a flat list of events.
pub fn compute_max_duration(events: &[TraceEvent]) -> u64 {
    events.iter().map(|e| e.duration_ms).max().unwrap_or(0)
}
