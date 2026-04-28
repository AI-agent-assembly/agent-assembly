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
