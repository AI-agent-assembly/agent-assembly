//! Event type enum for the `aasm logs` command filter.

use std::fmt;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Event types accepted by the `--type` filter.
///
/// These map 1:1 to the `EventType` variants exposed by the
/// `aa-api` WebSocket and REST endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEventType {
    /// Policy violation events.
    Violation,
    /// Human-in-the-loop approval requests.
    Approval,
    /// Budget threshold alerts.
    Budget,
}

impl LogEventType {
    /// Return the API wire representation used in query parameters.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Violation => "violation",
            Self::Approval => "approval",
            Self::Budget => "budget",
        }
    }
}

impl fmt::Display for LogEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_api_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_api_str_matches_serde_name() {
        assert_eq!(LogEventType::Violation.as_api_str(), "violation");
        assert_eq!(LogEventType::Approval.as_api_str(), "approval");
        assert_eq!(LogEventType::Budget.as_api_str(), "budget");
    }

    #[test]
    fn display_matches_api_str() {
        assert_eq!(LogEventType::Violation.to_string(), "violation");
        assert_eq!(LogEventType::Approval.to_string(), "approval");
        assert_eq!(LogEventType::Budget.to_string(), "budget");
    }

    #[test]
    fn value_variants_contains_all() {
        let variants = LogEventType::value_variants();
        assert_eq!(variants.len(), 3);
    }
}
