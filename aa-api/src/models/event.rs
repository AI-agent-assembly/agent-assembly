//! Unified governance event model for WebSocket streaming.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::event_type::EventType;

/// Unique identifier for a governance event in the replay buffer.
pub type EventId = u64;

/// A governance event delivered to WebSocket subscribers.
///
/// This is the unified JSON representation sent over the wire.
/// It wraps events from all three domain channels (pipeline,
/// approval, budget) into a single schema that clients can
/// filter by [`EventType`].
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GovernanceEvent {
    /// Monotonically increasing event identifier.
    #[schema(value_type = u64)]
    pub id: EventId,
    /// Classification of the event for client-side filtering.
    pub event_type: EventType,
    /// Agent that produced or is associated with the event.
    pub agent_id: String,
    /// Event-specific payload serialised as a JSON value.
    pub payload: serde_json::Value,
    /// Timestamp when the event was received by the API layer (ISO 8601).
    #[schema(value_type = String)]
    pub timestamp: DateTime<Utc>,
}
