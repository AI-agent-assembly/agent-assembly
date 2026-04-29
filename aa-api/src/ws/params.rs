//! Query parameters for the WebSocket events endpoint.

use serde::Deserialize;

use crate::models::{EventId, EventType};

/// Query parameters accepted by `GET /api/v1/ws/events`.
///
/// All parameters are optional:
/// - `types`: comma-separated event type filter (e.g. `violation,budget`).
///   All types are included when omitted.
/// - `agent_id`: restrict events to a single agent.
/// - `since`: replay buffered events whose id is greater than this value.
#[derive(Debug, Deserialize)]
pub struct WsQueryParams {
    /// Comma-separated event type filter.
    pub types: Option<String>,
    /// Filter events by agent identifier.
    pub agent_id: Option<String>,
    /// Replay events after this event id on reconnect.
    pub since: Option<EventId>,
}

impl WsQueryParams {
    /// Resolve the event type filter to a concrete list.
    pub fn event_types(&self) -> Vec<EventType> {
        EventType::parse_filter(self.types.as_deref())
    }
}
