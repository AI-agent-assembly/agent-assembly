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
