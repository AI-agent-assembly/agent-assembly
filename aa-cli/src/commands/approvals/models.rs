//! Data models for the `aasm approvals` subcommand.

use serde::{Deserialize, Serialize};

/// JSON representation of a pending approval request returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponse {
    /// Unique approval request identifier.
    pub id: String,
    /// Agent that triggered the approval.
    pub agent_id: String,
    /// The governance action requiring approval.
    pub action: String,
    /// Human-readable reason for the approval request.
    pub reason: String,
    /// Current status: "pending", "approved", or "rejected".
    pub status: String,
    /// ISO 8601 timestamp when the request was created.
    pub created_at: String,
}

/// Color category for approval countdown timer display.
///
/// Indicates urgency: `Red` means the approval is about to time out,
/// `Yellow` means moderate urgency, `Green` means plenty of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutColor {
    /// Less than 60 seconds remaining.
    Red,
    /// Between 60 and 180 seconds remaining.
    Yellow,
    /// More than 180 seconds remaining.
    Green,
}

/// Determine the [`TimeoutColor`] for the given remaining seconds.
///
/// - `remaining <= 0` or `remaining < 60` → [`TimeoutColor::Red`]
/// - `60 <= remaining <= 180` → [`TimeoutColor::Yellow`]
/// - `remaining > 180` → [`TimeoutColor::Green`]
pub fn compute_timeout_color(remaining_secs: i64) -> TimeoutColor {
    if remaining_secs < 60 {
        TimeoutColor::Red
    } else if remaining_secs <= 180 {
        TimeoutColor::Yellow
    } else {
        TimeoutColor::Green
    }
}

/// Format remaining seconds as a human-readable countdown string.
///
/// Returns strings like `"2m 30s"`, `"45s"`, or `"expired"` for
/// non-positive values.
pub fn format_countdown(remaining_secs: i64) -> String {
    if remaining_secs <= 0 {
        return "expired".to_string();
    }
    let mins = remaining_secs / 60;
    let secs = remaining_secs % 60;
    if mins > 0 {
        format!("{mins}m {secs:02}s")
    } else {
        format!("{secs}s")
    }
}

/// Generic paginated response wrapper matching the aa-api JSON envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + for<'a> Deserialize<'a>")]
pub struct PaginatedResponse<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Current page number (1-based).
    pub page: u64,
    /// Items per page.
    pub per_page: u64,
    /// Total items across all pages.
    pub total: u64,
}
