//! Tree renderer for session traces using box-drawing characters.

/// Format a duration in milliseconds into a human-readable string.
///
/// Examples: `0ms`, `142ms`, `1200ms`, `60000ms`.
pub fn format_duration(duration_ms: u64) -> String {
    format!("{duration_ms}ms")
}
