//! Correlation outcome types produced by the engine.

use uuid::Uuid;

/// A positive causal correlation between an intent event and an action event.
///
/// Produced when the engine matches an LLM response intent to a kernel-level
/// action within the configured time window and PID lineage.
#[derive(Debug, Clone)]
pub struct CausalCorrelation {
    /// ID of the intent event (LLM response) that initiated the action.
    pub intent_event_id: Uuid,
    /// ID of the action event (kernel syscall) that was correlated.
    pub action_event_id: Uuid,
    /// Strength of the correlation (0.0–1.0).
    ///
    /// Determined by the matching algorithm based on keyword overlap,
    /// PID proximity, and time delta.
    pub correlation_strength: f64,
    /// Time elapsed (in milliseconds) between the intent and the action.
    pub time_delta_ms: u64,
}

/// The result of a correlation check — one of three possible outcomes.
#[derive(Debug, Clone)]
pub enum CorrelationOutcome {
    /// An intent was matched to a kernel action within the time window.
    Matched(CausalCorrelation),
    /// A kernel action was observed with no preceding LLM intent in the window.
    ///
    /// This indicates the agent performed an action that was not instructed —
    /// potential unauthorized escalation.
    UnexpectedAction {
        /// ID of the unmatched action event.
        action_event_id: Uuid,
    },
    /// An LLM intent was observed but no corresponding kernel action followed
    /// within the time window.
    ///
    /// This may indicate the agent bypassed the normal execution path.
    IntentWithoutAction {
        /// ID of the unmatched intent event.
        intent_event_id: Uuid,
    },
}
