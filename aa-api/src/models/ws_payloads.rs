//! OpenAPI schemas for WebSocket event payloads.
//!
//! These types document the JSON structure of `GovernanceEvent.payload`
//! for each [`EventType`] variant.  They mirror the internal runtime
//! and gateway types (`PipelineEvent`, `ApprovalRequest`, `BudgetAlert`)
//! without pulling `utoipa` into those crates.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Payload for `event_type: "violation"` events.
///
/// Represents a governance audit event from the pipeline — either an
/// action that violated policy or an interception layer degradation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ViolationPayload {
    /// A governance audit event enriched with runtime metadata.
    Audit {
        /// Source that delivered the event: `"sdk"`, `"ebpf"`, or `"proxy"`.
        source: String,
        /// Unix milliseconds when the pipeline received the event.
        received_at_ms: i64,
        /// Monotonic sequence number assigned by the pipeline.
        sequence_number: u64,
    },
    /// An interception layer became unavailable.
    LayerDegradation {
        /// Name of the degraded layer (e.g. `"ebpf"`, `"proxy"`).
        layer: String,
        /// Human-readable reason for the degradation.
        reason: String,
        /// Remaining active layers after degradation.
        remaining_layers: Vec<String>,
    },
}

/// Payload for `event_type: "approval"` events.
///
/// Represents a human-in-the-loop approval request submitted by the
/// policy engine when an action requires explicit authorisation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApprovalPayload {
    /// Unique ID for the approval request (UUID v4).
    pub request_id: String,
    /// Human-readable description of the action awaiting approval.
    pub action: String,
    /// Policy condition that triggered this request.
    pub condition_triggered: String,
    /// Unix epoch timestamp (seconds) when the request was submitted.
    pub submitted_at: u64,
    /// Seconds before the request times out.
    pub timeout_secs: u64,
}

/// Payload for `event_type: "budget"` events.
///
/// Emitted when an agent's spend crosses a configured daily threshold
/// (80 % or 95 % of the daily limit).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BudgetAlertPayload {
    /// Threshold percentage that was crossed (e.g. `80` or `95`).
    pub threshold_pct: u8,
    /// Current total spend in USD at the time of the alert.
    pub spent_usd: f64,
    /// Configured daily limit in USD.
    pub limit_usd: f64,
}
