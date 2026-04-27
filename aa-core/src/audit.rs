//! Immutable, hash-chained audit entry for Agent Assembly governance events.
//!
//! Each [`AuditEntry`] commits to all tamper-meaningful fields via a SHA-256 hash
//! that includes the hash of the preceding entry, forming a tamper-evident chain.
//!
//! Gated on the `alloc` feature because [`AuditEntry::payload`] is an
//! [`alloc::string::String`].

use alloc::string::String;
use sha2::{Digest, Sha256};

use crate::{AgentId, SessionId};

// ---------------------------------------------------------------------------
// AuditEventType
// ---------------------------------------------------------------------------

/// Category of a governance event recorded in an [`AuditEntry`].
///
/// The `#[repr(u32)]` attribute makes `event_type as u32` the canonical
/// 4-byte discriminant used in the SHA-256 hash input.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AuditEventType {
    ToolCallIntercepted = 0,
    PolicyViolation = 1,
    CredentialLeakBlocked = 2,
    ApprovalRequested = 3,
    ApprovalGranted = 4,
    ApprovalDenied = 5,
    BudgetLimitApproached = 6,
    BudgetLimitExceeded = 7,
}

impl AuditEventType {
    /// Returns the string label used in [`Display`] output and log messages.
    ///
    /// [`Display`]: core::fmt::Display
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ToolCallIntercepted => "ToolCallIntercepted",
            Self::PolicyViolation => "PolicyViolation",
            Self::CredentialLeakBlocked => "CredentialLeakBlocked",
            Self::ApprovalRequested => "ApprovalRequested",
            Self::ApprovalGranted => "ApprovalGranted",
            Self::ApprovalDenied => "ApprovalDenied",
            Self::BudgetLimitApproached => "BudgetLimitApproached",
            Self::BudgetLimitExceeded => "BudgetLimitExceeded",
        }
    }
}

// ---------------------------------------------------------------------------
// AuditEntry
// ---------------------------------------------------------------------------

/// An immutable, hash-chained record of a single governance event.
///
/// ## Immutability
///
/// All fields are private. The only way to create an [`AuditEntry`] is via
/// [`AuditEntry::new`]. There are no mutation methods.
///
/// ## Hash chain
///
/// `entry_hash` is a SHA-256 digest computed over all tamper-meaningful fields
/// in a canonical byte order (see [`AuditEntry::new`] for the full sequence).
/// Each entry commits to `previous_hash`, linking entries into a tamper-evident
/// chain. The genesis entry uses `[0u8; 32]` as `previous_hash`.
///
/// ## Tamper detection
///
/// [`AuditEntry::verify_integrity`] re-computes the hash from the stored fields
/// and compares it to the stored `entry_hash`. Any field alteration — including
/// via `unsafe` code — will cause the re-computed hash to diverge.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AuditEntry {
    seq: u64,
    timestamp_ns: u64,
    event_type: AuditEventType,
    agent_id: AgentId,
    session_id: SessionId,
    payload: String,
    previous_hash: [u8; 32],
    entry_hash: [u8; 32],
}

impl AuditEntry {
    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /// Create a new [`AuditEntry`], computing `entry_hash` over all fields.
    ///
    /// ## Parameters
    ///
    /// - `seq` — monotonic counter within the session; genesis entry is `0`.
    /// - `timestamp_ns` — nanoseconds since the Unix epoch (caller-supplied;
    ///   use `Timestamp::from(SystemTime::now()).as_nanos()` in `std` environments).
    /// - `event_type` — category of the governance event.
    /// - `agent_id` — identifier of the agent that produced the event.
    /// - `session_id` — identifier of the specific agent run.
    /// - `payload` — pre-serialized UTF-8 string (JSON in practice).
    /// - `previous_hash` — `entry_hash` of the preceding entry;
    ///   `[0u8; 32]` for the genesis entry.
    ///
    /// ## Canonical hash input (84 fixed bytes + variable payload)
    ///
    /// ```text
    /// SHA-256(
    ///     seq.to_be_bytes()                  //  8 bytes
    ///     || timestamp_ns.to_be_bytes()      //  8 bytes
    ///     || (event_type as u32).to_be_bytes() // 4 bytes
    ///     || agent_id.as_bytes()             // 16 bytes
    ///     || session_id.as_bytes()           // 16 bytes
    ///     || previous_hash                   // 32 bytes
    ///     || payload.as_bytes()              // variable
    /// )
    /// ```
    pub fn new(
        seq: u64,
        timestamp_ns: u64,
        event_type: AuditEventType,
        agent_id: AgentId,
        session_id: SessionId,
        payload: String,
        previous_hash: [u8; 32],
    ) -> Self {
        let entry_hash = Self::compute_hash(
            seq,
            timestamp_ns,
            &event_type,
            &agent_id,
            &session_id,
            &previous_hash,
            &payload,
        );
        Self {
            seq,
            timestamp_ns,
            event_type,
            agent_id,
            session_id,
            payload,
            previous_hash,
            entry_hash,
        }
    }

    // -----------------------------------------------------------------------
    // Getters
    // -----------------------------------------------------------------------

    /// Monotonic sequence counter within the session.
    #[inline]
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// Nanoseconds since the Unix epoch at the time the entry was created.
    #[inline]
    pub fn timestamp_ns(&self) -> u64 {
        self.timestamp_ns
    }

    /// Category of the governance event.
    #[inline]
    pub fn event_type(&self) -> AuditEventType {
        self.event_type
    }

    /// Identifier of the agent that produced this entry.
    #[inline]
    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    /// Identifier of the specific agent run (session) that produced this entry.
    #[inline]
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Pre-serialized UTF-8 payload (JSON in practice).
    #[inline]
    pub fn payload(&self) -> &str {
        &self.payload
    }

    /// SHA-256 hash of the preceding entry; `[0u8; 32]` for the genesis entry.
    #[inline]
    pub fn previous_hash(&self) -> &[u8; 32] {
        &self.previous_hash
    }

    /// SHA-256 hash computed over all tamper-meaningful fields at construction.
    #[inline]
    pub fn entry_hash(&self) -> &[u8; 32] {
        &self.entry_hash
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Canonical SHA-256 computation over all tamper-meaningful fields.
    ///
    /// Field order and encoding are fixed — see [`AuditEntry::new`] for the
    /// documented byte sequence.
    fn compute_hash(
        seq: u64,
        timestamp_ns: u64,
        event_type: &AuditEventType,
        agent_id: &AgentId,
        session_id: &SessionId,
        previous_hash: &[u8; 32],
        payload: &str,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(seq.to_be_bytes());
        hasher.update(timestamp_ns.to_be_bytes());
        hasher.update((*event_type as u32).to_be_bytes());
        hasher.update(agent_id.as_bytes());
        hasher.update(session_id.as_bytes());
        hasher.update(previous_hash);
        hasher.update(payload.as_bytes());
        hasher.finalize().into()
    }
}
