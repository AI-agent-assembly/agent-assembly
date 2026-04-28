//! Output types for policy simulation results.

use serde::Serialize;

/// The outcome of evaluating a single event against a policy in dry-run mode.
#[derive(Debug, Clone, Serialize)]
pub struct EventOutcome {
    /// Zero-based index of the event in the input sequence.
    pub event_index: usize,
    /// Human-readable description of the action that was evaluated.
    pub action: String,
    /// The policy decision: "allow", "deny", or "requires_approval".
    pub decision: String,
    /// Explanation of why this decision was reached.
    pub reason: String,
}
