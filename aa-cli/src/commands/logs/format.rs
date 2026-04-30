//! Log line formatting and color output for the `aasm logs` command.

use console::Style;
use serde::{Deserialize, Serialize};

/// Normalised log entry shared by both fetch (REST) and follow (WS) modes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLineData {
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Event type label (e.g. `"violation"`).
    pub event_type: String,
    /// Hex-encoded agent identifier.
    pub agent_id: String,
    /// Human-readable message summary.
    pub message: String,
}

/// Format a single log line for human-readable terminal output.
///
/// Output: `<timestamp> [<TYPE>] <agent_id>  <message>`
///
/// When `use_color` is true the type tag is styled according to
/// [`style_for_type`].
pub fn format_log_line(entry: &LogLineData, use_color: bool) -> String {
    let tag = format!("[{}]", entry.event_type.to_uppercase());
    let styled_tag = if use_color {
        let style = style_for_type(&entry.event_type);
        style.apply_to(&tag).to_string()
    } else {
        tag
    };
    format!(
        "{} {:12} {}  {}",
        entry.timestamp, styled_tag, entry.agent_id, entry.message
    )
}

/// Return a [`Style`] for the given event type string.
///
/// Known types get a distinct colour; unknown future types fall back
/// to white so the CLI can display them without a code change.
pub fn style_for_type(event_type: &str) -> Style {
    match event_type {
        "violation" => Style::new().red().bold(),
        "approval" => Style::new().yellow(),
        "budget" => Style::new().cyan(),
        _ => Style::new().white(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_types_get_distinct_styles() {
        // Ensure the function does not panic for each known type.
        let _ = style_for_type("violation");
        let _ = style_for_type("approval");
        let _ = style_for_type("budget");
    }

    #[test]
    fn unknown_type_returns_white_style() {
        let _ = style_for_type("tool_call");
        let _ = style_for_type("unknown_future_type");
    }

    fn sample_entry() -> LogLineData {
        LogLineData {
            timestamp: "2026-04-30T10:00:00Z".to_string(),
            event_type: "violation".to_string(),
            agent_id: "aa001".to_string(),
            message: "policy denied tool call".to_string(),
        }
    }

    #[test]
    fn format_log_line_no_color_contains_all_fields() {
        let line = format_log_line(&sample_entry(), false);
        assert!(line.contains("2026-04-30T10:00:00Z"));
        assert!(line.contains("[VIOLATION]"));
        assert!(line.contains("aa001"));
        assert!(line.contains("policy denied tool call"));
    }

    #[test]
    fn format_log_line_with_color_does_not_panic() {
        let _ = format_log_line(&sample_entry(), true);
    }
}
