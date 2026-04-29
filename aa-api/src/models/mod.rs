//! Data models for the REST and WebSocket API layer.

pub mod event;
pub mod event_type;

pub use event::{EventId, GovernanceEvent};
pub use event_type::EventType;
