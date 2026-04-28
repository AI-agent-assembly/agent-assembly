//! Human-approval request queue for Agent Assembly governance.
//!
//! When the policy engine returns [`aa_core::PolicyResult::RequiresApproval`],
//! the runtime submits an [`ApprovalRequest`] here. The request stays pending
//! until a human operator calls [`ApprovalQueue::decide`], or the per-request
//! timeout elapses and the queue auto-resolves it as [`ApprovalDecision::TimedOut`].

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::oneshot;
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
// ApprovalError
// ---------------------------------------------------------------------------

/// Errors returned by [`ApprovalQueue::decide`].
#[derive(Debug, PartialEq, Eq)]
pub enum ApprovalError {
    /// No pending request exists for the given ID (already resolved or never submitted).
    NotFound,
}

impl std::fmt::Display for ApprovalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "approval request not found"),
        }
    }
}

impl std::error::Error for ApprovalError {}

// ---------------------------------------------------------------------------
// ApprovalQueue
// ---------------------------------------------------------------------------

/// Concurrent, in-memory store of pending approval requests.
///
/// Constructed via [`ApprovalQueue::new`], which returns an [`Arc`] so the
/// queue can be cloned cheaply across tasks (e.g., the timeout spawner holds
/// a back-reference).
pub struct ApprovalQueue {
    pending: DashMap<ApprovalRequestId, (ApprovalRequest, oneshot::Sender<ApprovalDecision>)>,
}

impl ApprovalQueue {
    /// Creates a new, empty queue wrapped in an [`Arc`].
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            pending: DashMap::new(),
        })
    }

    /// Returns a snapshot of all currently pending requests.
    ///
    /// The snapshot is consistent at the moment of the call; entries submitted
    /// or resolved concurrently may not appear.
    pub fn list(&self) -> Vec<PendingApprovalRequest> {
        self.pending
            .iter()
            .map(|entry| {
                let req = &entry.value().0;
                PendingApprovalRequest {
                    request_id: req.request_id,
                    agent_id: req.agent_id.clone(),
                    action: req.action.clone(),
                    condition_triggered: req.condition_triggered.clone(),
                    submitted_at: req.submitted_at,
                    timeout_secs: req.timeout_secs,
                }
            })
            .collect()
    }

    /// Apply an [`ApprovalDecision`] to the request identified by `id`.
    ///
    /// Returns `Err(ApprovalError::NotFound)` if no pending request exists for
    /// `id` (already resolved, timed out, or never submitted).
    pub fn decide(
        &self,
        id: ApprovalRequestId,
        decision: ApprovalDecision,
    ) -> Result<(), ApprovalError> {
        if self.resolve(id, decision) {
            Ok(())
        } else {
            Err(ApprovalError::NotFound)
        }
    }

    /// Remove and settle the request identified by `id`.
    ///
    /// Returns `true` if the entry existed and the sender was consumed, `false`
    /// if the entry was already gone (idempotent — a second call for the same
    /// `id` is a safe no-op).
    fn resolve(&self, id: ApprovalRequestId, decision: ApprovalDecision) -> bool {
        if let Some((_key, (_req, tx))) = self.pending.remove(&id) {
            // Ignore send errors: the receiver may have been dropped (caller
            // gave up waiting), which is not a failure on our side.
            let _ = tx.send(decision);
            true
        } else {
            false
        }
    }

    /// Submit a new approval request and start its timeout task.
    ///
    /// Returns the request's [`ApprovalRequestId`] and an [`ApprovalFuture`]
    /// that resolves when the request is settled (approved, rejected, or timed
    /// out).
    ///
    /// # Timeout behaviour
    ///
    /// A `tokio::spawn`ed task sleeps for `request.timeout_secs` seconds, then
    /// calls `resolve(TimedOut)`. Because [`resolve`] is idempotent, a human
    /// decision that arrives before the timeout simply wins the race; the
    /// timeout task's subsequent `resolve` call becomes a no-op.
    pub fn submit(self: &Arc<Self>, request: ApprovalRequest) -> (ApprovalRequestId, ApprovalFuture) {
        let id = request.request_id;
        let timeout_secs = request.timeout_secs;
        let fallback = request.fallback.clone();

        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, (request, tx));

        // Spawn the timeout enforcer.  The Arc clone keeps the queue alive
        // for the duration of the sleep even if all other holders drop.
        let queue = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)).await;
            queue.resolve(id, ApprovalDecision::TimedOut { fallback });
        });

        (id, rx)
    }
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

    // --- ApprovalDecision ---

    #[test]
    fn approval_decision_approved_fields() {
        let d = ApprovalDecision::Approved {
            by: "alice".to_string(),
            reason: Some("looks safe".to_string()),
        };
        if let ApprovalDecision::Approved { by, reason } = d {
            assert_eq!(by, "alice");
            assert_eq!(reason, Some("looks safe".to_string()));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn approval_decision_rejected_fields() {
        let d = ApprovalDecision::Rejected {
            by: "bob".to_string(),
            reason: "policy violation".to_string(),
        };
        if let ApprovalDecision::Rejected { by, reason } = d {
            assert_eq!(by, "bob");
            assert_eq!(reason, "policy violation");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn approval_decision_timed_out_carries_fallback() {
        let fallback = aa_core::PolicyResult::Deny {
            reason: "expired".to_string(),
        };
        let d = ApprovalDecision::TimedOut { fallback: fallback.clone() };
        if let ApprovalDecision::TimedOut { fallback: f } = d {
            assert_eq!(f, fallback);
        } else {
            panic!("wrong variant");
        }
    }

    // --- ApprovalError ---

    #[test]
    fn approval_error_not_found_display() {
        let e = ApprovalError::NotFound;
        assert_eq!(e.to_string(), "approval request not found");
    }

    #[test]
    fn approval_error_not_found_eq() {
        assert_eq!(ApprovalError::NotFound, ApprovalError::NotFound);
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

    // --- ApprovalQueue::new and list ---

    #[test]
    fn new_queue_list_is_empty() {
        let q = ApprovalQueue::new();
        assert!(q.list().is_empty());
    }

    // --- ApprovalQueue::decide (no pending entry) ---

    #[test]
    fn decide_unknown_id_returns_not_found() {
        let q = ApprovalQueue::new();
        let result = q.decide(
            Uuid::new_v4(),
            ApprovalDecision::Approved {
                by: "alice".to_string(),
                reason: None,
            },
        );
        assert_eq!(result, Err(ApprovalError::NotFound));
    }

    fn make_request(timeout_secs: u64) -> ApprovalRequest {
        ApprovalRequest {
            request_id: Uuid::new_v4(),
            agent_id: "agent-1".to_string(),
            action: "read_file /etc/passwd".to_string(),
            condition_triggered: "sensitive-file-access".to_string(),
            submitted_at: 1_700_000_000,
            timeout_secs,
            fallback: aa_core::PolicyResult::Deny {
                reason: "timed out".to_string(),
            },
        }
    }

    // --- ApprovalQueue::submit ---

    #[tokio::test]
    async fn submit_then_approve_resolves_future() {
        let q = ApprovalQueue::new();
        let req = make_request(60);
        let id = req.request_id;
        let (_rid, fut) = q.submit(req);

        q.decide(
            id,
            ApprovalDecision::Approved { by: "alice".to_string(), reason: None },
        )
        .expect("decide should succeed");

        let decision = fut.await.expect("future should resolve");
        assert!(matches!(decision, ApprovalDecision::Approved { .. }));
    }

    #[tokio::test]
    async fn submit_then_reject_resolves_future() {
        let q = ApprovalQueue::new();
        let req = make_request(60);
        let id = req.request_id;
        let (_rid, fut) = q.submit(req);

        q.decide(
            id,
            ApprovalDecision::Rejected {
                by: "bob".to_string(),
                reason: "not allowed".to_string(),
            },
        )
        .expect("decide should succeed");

        let decision = fut.await.expect("future should resolve");
        assert!(matches!(decision, ApprovalDecision::Rejected { .. }));
    }

    #[tokio::test]
    async fn decide_after_resolve_returns_not_found() {
        let q = ApprovalQueue::new();
        let req = make_request(60);
        let id = req.request_id;
        let (_rid, _fut) = q.submit(req);

        q.decide(
            id,
            ApprovalDecision::Approved { by: "alice".to_string(), reason: None },
        )
        .expect("first decide should succeed");

        let result = q.decide(
            id,
            ApprovalDecision::Rejected { by: "eve".to_string(), reason: "too late".to_string() },
        );
        assert_eq!(result, Err(ApprovalError::NotFound));
    }

    #[tokio::test(start_paused = true)]
    async fn submit_times_out_after_timeout_secs() {
        let q = ApprovalQueue::new();
        let req = make_request(5);
        let (_rid, fut) = q.submit(req);

        tokio::time::advance(std::time::Duration::from_secs(6)).await;

        let decision = fut.await.expect("future should resolve after timeout");
        assert!(matches!(decision, ApprovalDecision::TimedOut { .. }));
    }

    #[tokio::test]
    async fn list_reflects_pending_and_clears_after_decide() {
        let q = ApprovalQueue::new();
        let req = make_request(60);
        let id = req.request_id;
        let (_rid, _fut) = q.submit(req);

        let pending = q.list();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].request_id, id);

        q.decide(
            id,
            ApprovalDecision::Approved { by: "alice".to_string(), reason: None },
        )
        .expect("decide should succeed");

        assert!(q.list().is_empty());
    }

    #[tokio::test]
    async fn submit_100_concurrent_requests_all_resolve() {
        use std::collections::HashMap;

        let q = ApprovalQueue::new();
        let n = 100_usize;

        let mut futures_map = HashMap::new();
        for _ in 0..n {
            let req = make_request(60);
            let id = req.request_id;
            let (_rid, fut) = q.submit(req);
            futures_map.insert(id, fut);
        }

        assert_eq!(q.list().len(), n);

        let ids: Vec<_> = futures_map.keys().copied().collect();
        for id in &ids {
            q.decide(
                *id,
                ApprovalDecision::Approved { by: "operator".to_string(), reason: None },
            )
            .expect("decide should succeed for each request");
        }

        for (_id, fut) in futures_map {
            let decision = fut.await.expect("future should resolve");
            assert!(matches!(decision, ApprovalDecision::Approved { .. }));
        }

        assert!(q.list().is_empty());
    }
}
