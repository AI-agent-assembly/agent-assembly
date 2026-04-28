//! Human-approval request queue for Agent Assembly governance.
//!
//! When the policy engine returns [`aa_core::PolicyResult::RequiresApproval`],
//! the runtime submits an [`ApprovalRequest`] here. The request stays pending
//! until a human operator calls [`ApprovalQueue::decide`], or the per-request
//! timeout elapses and the queue auto-resolves it as [`ApprovalDecision::TimedOut`].

use uuid::Uuid;

// ---------------------------------------------------------------------------
// Public type aliases
// ---------------------------------------------------------------------------

/// Opaque identifier for a single pending approval request.
pub type ApprovalRequestId = Uuid;

/// A one-shot receiver that resolves to the [`ApprovalDecision`] once a human
/// (or the timeout task) settles the request.
pub type ApprovalFuture = tokio::sync::oneshot::Receiver<ApprovalDecision>;

// ---------------------------------------------------------------------------
// ApprovalRequest
// ---------------------------------------------------------------------------

/// All data needed to present a pending action to a human operator.
#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    /// Unique ID for this request (UUID v4).
    pub request_id: ApprovalRequestId,
    /// The agent that triggered the approval requirement.
    pub agent_id: String,
    /// Human-readable description of the action awaiting approval.
    pub action: String,
    /// Name or description of the policy condition that triggered this request.
    pub condition_triggered: String,
    /// Unix epoch timestamp (seconds) when the request was submitted.
    pub submitted_at: u64,
    /// Seconds before the queue auto-resolves the request as timed-out.
    pub timeout_secs: u64,
    /// Policy decision to apply if the request times out without a human decision.
    pub fallback: aa_core::PolicyResult,
}

// ---------------------------------------------------------------------------
// PendingApprovalRequest  (safe, outward-facing view — no channel or fallback)
// ---------------------------------------------------------------------------

/// A redacted, outward-facing snapshot of a pending request.
///
/// Returned by [`ApprovalQueue::list`] so callers cannot access the internal
/// one-shot sender or fallback policy.
#[derive(Debug, Clone)]
pub struct PendingApprovalRequest {
    /// Unique ID for this request.
    pub request_id: ApprovalRequestId,
    /// The agent that triggered the approval requirement.
    pub agent_id: String,
    /// Human-readable description of the action awaiting approval.
    pub action: String,
    /// Name or description of the policy condition that triggered this request.
    pub condition_triggered: String,
    /// Unix epoch timestamp (seconds) when the request was submitted.
    pub submitted_at: u64,
    /// Seconds before the request times out.
    pub timeout_secs: u64,
}

// ---------------------------------------------------------------------------
// ApprovalDecision  (placeholder — full definition added in next commit)
// ---------------------------------------------------------------------------

/// The outcome of a pending [`ApprovalRequest`].
#[derive(Debug, Clone)]
pub enum ApprovalDecision {
    /// A human operator approved the action.
    Approved {
        /// Identifier of the operator who approved.
        by: String,
        /// Optional free-text rationale.
        reason: Option<String>,
    },
    /// A human operator rejected the action.
    Rejected {
        /// Identifier of the operator who rejected.
        by: String,
        /// Mandatory explanation for the rejection.
        reason: String,
    },
    /// The timeout elapsed before a human decided; the fallback policy applies.
    TimedOut {
        /// The fallback [`aa_core::PolicyResult`] originally attached to the request.
        fallback: aa_core::PolicyResult,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- type aliases ---

    #[test]
    fn approval_request_id_is_uuid() {
        let id: ApprovalRequestId = Uuid::new_v4();
        assert!(!id.is_nil());
    }

    // --- ApprovalRequest fields ---

    #[test]
    fn approval_request_fields_are_accessible() {
        let req = ApprovalRequest {
            request_id: Uuid::new_v4(),
            agent_id: "agent-1".to_string(),
            action: "read_file /etc/passwd".to_string(),
            condition_triggered: "sensitive-file-access".to_string(),
            submitted_at: 1_700_000_000,
            timeout_secs: 30,
            fallback: aa_core::PolicyResult::Deny {
                reason: "timed out".to_string(),
            },
        };
        assert_eq!(req.agent_id, "agent-1");
        assert_eq!(req.timeout_secs, 30);
        assert!(!req.request_id.is_nil());
    }

    // --- PendingApprovalRequest ---

    #[test]
    fn pending_approval_request_fields_match_source() {
        let id = Uuid::new_v4();
        let pending = PendingApprovalRequest {
            request_id: id,
            agent_id: "agent-1".to_string(),
            action: "read_file /etc/passwd".to_string(),
            condition_triggered: "sensitive-file-access".to_string(),
            submitted_at: 1_700_000_000,
            timeout_secs: 60,
        };
        assert_eq!(pending.request_id, id);
        assert_eq!(pending.agent_id, "agent-1");
        assert_eq!(pending.timeout_secs, 60);
    }
}
