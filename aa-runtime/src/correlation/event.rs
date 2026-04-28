//! Event types for the causal correlation engine.

use uuid::Uuid;

/// An LLM response intent captured via the SDK hook or proxy.
///
/// Represents what the LLM instructed the agent to do — e.g., "delete file X",
/// "make HTTP request to Y". The correlation engine matches these intents
/// against observed kernel-level actions.
#[derive(Debug, Clone)]
pub struct IntentEvent {
    /// Unique identifier for this intent event.
    pub event_id: Uuid,
    /// Unix timestamp in milliseconds when the intent was captured.
    pub timestamp_ms: u64,
    /// PID of the process that received the LLM response.
    pub pid: u32,
    /// The raw text or structured description of the intended action
    /// extracted from the LLM response.
    pub intent_text: String,
    /// The action type keyword derived from the intent text
    /// (e.g., "file_delete", "network_connect", "process_exec").
    pub action_keyword: String,
}

/// A kernel-level action captured via eBPF probes.
///
/// Represents an observed syscall — e.g., `unlink("/tmp/foo")`,
/// `connect(1.2.3.4:443)`, `execve("/bin/sh")`. The correlation engine
/// matches these actions against preceding LLM intents.
#[derive(Debug, Clone)]
pub struct ActionEvent {
    /// Unique identifier for this action event.
    pub event_id: Uuid,
    /// Unix timestamp in milliseconds when the syscall was observed.
    pub timestamp_ms: u64,
    /// PID of the process that performed the syscall.
    pub pid: u32,
    /// The syscall name (e.g., "unlink", "connect", "execve", "openat").
    pub syscall: String,
    /// Human-readable summary of the syscall arguments
    /// (e.g., the file path for unlink, the address for connect).
    pub details: String,
}

/// A correlation event — either an intent from the LLM or an action from the kernel.
///
/// This is the unified input type ingested by the [`super::SlidingWindow`].
#[derive(Debug, Clone)]
pub enum CorrelationEvent {
    /// An LLM response intent.
    Intent(IntentEvent),
    /// A kernel-level syscall action.
    Action(ActionEvent),
}

impl CorrelationEvent {
    /// Returns the timestamp (in milliseconds) of the underlying event.
    pub fn timestamp_ms(&self) -> u64 {
        match self {
            Self::Intent(e) => e.timestamp_ms,
            Self::Action(e) => e.timestamp_ms,
        }
    }

    /// Returns the PID of the process that produced the event.
    pub fn pid(&self) -> u32 {
        match self {
            Self::Intent(e) => e.pid,
            Self::Action(e) => e.pid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_intent(ts: u64, pid: u32) -> IntentEvent {
        IntentEvent {
            event_id: Uuid::new_v4(),
            timestamp_ms: ts,
            pid,
            intent_text: "delete file".to_string(),
            action_keyword: "file_delete".to_string(),
        }
    }

    fn make_action(ts: u64, pid: u32) -> ActionEvent {
        ActionEvent {
            event_id: Uuid::new_v4(),
            timestamp_ms: ts,
            pid,
            syscall: "unlink".to_string(),
            details: "/tmp/foo".to_string(),
        }
    }

    #[test]
    fn intent_event_fields_accessible() {
        let intent = make_intent(1000, 42);
        assert_eq!(intent.timestamp_ms, 1000);
        assert_eq!(intent.pid, 42);
    }

    #[test]
    fn action_event_fields_accessible() {
        let action = make_action(2000, 99);
        assert_eq!(action.timestamp_ms, 2000);
        assert_eq!(action.pid, 99);
    }

    #[test]
    fn correlation_event_timestamp_delegates_to_inner() {
        let intent = CorrelationEvent::Intent(make_intent(1000, 1));
        let action = CorrelationEvent::Action(make_action(2000, 2));
        assert_eq!(intent.timestamp_ms(), 1000);
        assert_eq!(action.timestamp_ms(), 2000);
    }

    #[test]
    fn correlation_event_pid_delegates_to_inner() {
        let intent = CorrelationEvent::Intent(make_intent(1000, 10));
        let action = CorrelationEvent::Action(make_action(2000, 20));
        assert_eq!(intent.pid(), 10);
        assert_eq!(action.pid(), 20);
    }
}
