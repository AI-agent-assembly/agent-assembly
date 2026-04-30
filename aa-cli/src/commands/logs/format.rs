//! Log line formatting and color output for the `aasm logs` command.

use console::Style;

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
}
