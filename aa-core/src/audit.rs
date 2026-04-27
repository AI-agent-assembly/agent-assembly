//! Immutable, hash-chained audit entry for Agent Assembly governance events.
//!
//! Each [`AuditEntry`] commits to all tamper-meaningful fields via a SHA-256 hash
//! that includes the hash of the preceding entry, forming a tamper-evident chain.
//!
//! Gated on the `alloc` feature because [`AuditEntry::payload`] is an
//! [`alloc::string::String`].

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
