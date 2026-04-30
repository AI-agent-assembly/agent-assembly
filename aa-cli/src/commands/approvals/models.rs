//! Data models for the `aasm approvals` subcommand.

use serde::{de::DeserializeOwned, Deserialize, Serialize};

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

/// Generic paginated response wrapper matching the aa-api JSON envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: DeserializeOwned> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Current page number (1-based).
    pub page: u64,
    /// Items per page.
    pub per_page: u64,
    /// Total items across all pages.
    pub total: u64,
}
