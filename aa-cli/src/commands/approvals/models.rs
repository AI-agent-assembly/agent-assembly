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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_response_deserializes_from_json() {
        let json = r#"{
            "id": "abc-123",
            "agent_id": "support-agent",
            "action": "process_refund",
            "reason": "amount > $100",
            "status": "pending",
            "created_at": "2026-04-30T10:00:00Z"
        }"#;
        let resp: ApprovalResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "abc-123");
        assert_eq!(resp.agent_id, "support-agent");
        assert_eq!(resp.action, "process_refund");
        assert_eq!(resp.reason, "amount > $100");
        assert_eq!(resp.status, "pending");
        assert_eq!(resp.created_at, "2026-04-30T10:00:00Z");
    }

    #[test]
    fn paginated_response_deserializes_from_json() {
        let json = r#"{
            "items": [{
                "id": "abc-123",
                "agent_id": "support-agent",
                "action": "process_refund",
                "reason": "amount > $100",
                "status": "pending",
                "created_at": "2026-04-30T10:00:00Z"
            }],
            "page": 1,
            "per_page": 20,
            "total": 1
        }"#;
        let resp: PaginatedResponse<ApprovalResponse> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.page, 1);
        assert_eq!(resp.per_page, 20);
        assert_eq!(resp.total, 1);
        assert_eq!(resp.items[0].id, "abc-123");
    }

    #[test]
    fn compute_timeout_color_red_below_60() {
        assert_eq!(compute_timeout_color(0), TimeoutColor::Red);
        assert_eq!(compute_timeout_color(59), TimeoutColor::Red);
        assert_eq!(compute_timeout_color(-10), TimeoutColor::Red);
    }

    #[test]
    fn compute_timeout_color_yellow_60_to_180() {
        assert_eq!(compute_timeout_color(60), TimeoutColor::Yellow);
        assert_eq!(compute_timeout_color(120), TimeoutColor::Yellow);
        assert_eq!(compute_timeout_color(180), TimeoutColor::Yellow);
    }

    #[test]
    fn compute_timeout_color_green_above_180() {
        assert_eq!(compute_timeout_color(181), TimeoutColor::Green);
        assert_eq!(compute_timeout_color(300), TimeoutColor::Green);
    }
}
