//! Anomaly response action executor.
//!
//! Receives [`AnomalyEvent`](super::types::AnomalyEvent) values from the
//! detector and executes the corresponding enforcement action:
//!
//! - **Pause**: suspend the agent via the registry; it can be resumed after review.
//! - **Block**: immediately deny the current action and further actions.
//! - **Alert**: emit a notification without interrupting the agent.
//! - **Quarantine**: isolate the agent and flag for security review.
//!
//! Alert delivery uses `tracing::warn!` for now. When the event bus
//! (AAASM-141) is implemented, alerts will be published as
//! [`AlertTriggered`](proto::event::AlertTriggered) messages on the broadcast
//! channel.

// TODO(AAASM-137): Implement AnomalyResponder struct with:
//   - respond(&self, event: AnomalyEvent) -> Result<(), ResponderError>
//   - tracing-based alert emission (upgrade to event bus in AAASM-141)
//   - registry integration for Pause/Block/Quarantine actions
