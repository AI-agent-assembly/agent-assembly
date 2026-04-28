//! Validation error and warning types for policy YAML parsing.

/// An error produced during policy document validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// Dot-notation field path, e.g. `"budget.daily_limit_usd"`.
    pub field: String,
    /// Human-readable description of the violated constraint.
    pub message: String,
    /// Best-effort line number from the YAML source (`None` when not determinable).
    pub line: Option<u32>,
}

impl ValidationError {
    /// Create a new error with no line information.
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            line: None,
        }
    }

    /// Attach a best-effort line number.
    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_new_sets_field_and_message() {
        let e = ValidationError::new("budget.daily_limit_usd", "must be > 0");
        assert_eq!(e.field, "budget.daily_limit_usd");
        assert_eq!(e.message, "must be > 0");
        assert_eq!(e.line, None);
    }

    #[test]
    fn validation_error_with_line_sets_line() {
        let e = ValidationError::new("network.allowlist[0]", "must not be empty")
            .with_line(7);
        assert_eq!(e.line, Some(7));
    }
}
