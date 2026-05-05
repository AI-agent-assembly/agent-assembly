//! Intermediate policy decision type used by the cascade merge layer.
//!
//! `PolicyDecision` is richer than `aa_core::PolicyResult` — it carries
//! the `source_scope` on `Deny` for audit and debugging. The engine
//! converts to `PolicyResult` before returning `EvaluationResult`.

use std::sync::Arc;

use crate::policy::document::PolicyDocument;
use crate::policy::scope::PolicyScope;

/// The outcome produced by evaluating a single policy document against one action.
///
/// Used by [`merge_decisions`] to combine per-scope outcomes. Convert to
/// [`aa_core::PolicyResult`] via [`PolicyDecision::into_policy_result`]
/// before surfacing through `EvaluationResult`.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    /// Action is allowed by this policy.
    Allow,
    /// Action requires human approval. Carries the policy's configured timeout.
    RequireApproval { reason: String, timeout_secs: u32 },
    /// Action is denied. `source_scope` identifies which scope triggered the deny.
    Deny { reason: String, source_scope: PolicyScope },
}

impl PolicyDecision {
    /// Convert to `aa_core::PolicyResult`, dropping the `source_scope` audit
    /// field that is not part of the core protocol type.
    pub fn into_policy_result(self) -> aa_core::PolicyResult {
        match self {
            PolicyDecision::Allow => aa_core::PolicyResult::Allow,
            PolicyDecision::RequireApproval { timeout_secs, .. } => {
                aa_core::PolicyResult::RequiresApproval { timeout_secs }
            }
            PolicyDecision::Deny { reason, .. } => aa_core::PolicyResult::Deny { reason },
        }
    }
}

/// Evaluate a single `PolicyDocument` against `(ctx, action)` and return the
/// decision for stages 1–3 and 5 (schedule, network, tool-allow, approval-condition).
///
/// Stages 4 (rate-limit) and 7 (budget) are stateful at engine level and are
/// evaluated separately after merging. Stage 6 (credential scan) does not
/// produce a decision — it is handled at the engine level.
///
/// Returns `Deny` on the first matching deny rule, `RequireApproval` for approval
/// conditions, and `Allow` if no rule fires.
pub(crate) fn evaluate_single_doc(
    doc: &PolicyDocument,
    ctx: &aa_core::AgentContext,
    action: &aa_core::GovernanceAction,
) -> PolicyDecision {
    // Stage 1 — Schedule
    if let Some(schedule) = &doc.schedule {
        if let Some(ah) = &schedule.active_hours {
            use chrono::Timelike;
            let tz: chrono_tz::Tz = ah.timezone.parse().unwrap_or(chrono_tz::UTC);
            let now = chrono::Utc::now().with_timezone(&tz);
            let current_hhmm = format!("{:02}:{:02}", now.hour(), now.minute());
            if current_hhmm < ah.start || current_hhmm >= ah.end {
                return PolicyDecision::Deny {
                    reason: "outside active hours".into(),
                    source_scope: doc.scope.clone(),
                };
            }
        }
    }

    // Stage 2 — Network allowlist
    if let aa_core::GovernanceAction::NetworkRequest { url, .. } = action {
        if let Some(np) = &doc.network {
            if !np.allowlist.is_empty() {
                let host = url
                    .split_once("://")
                    .map(|x| x.1)
                    .unwrap_or("")
                    .split('/')
                    .next()
                    .unwrap_or("");
                if !np.allowlist.iter().any(|entry| entry == host) {
                    return PolicyDecision::Deny {
                        reason: "host not in network allowlist".into(),
                        source_scope: doc.scope.clone(),
                    };
                }
            }
        }
    }

    // Stage 3 — Tool allow/deny
    if let aa_core::GovernanceAction::ToolCall { name, .. } = action {
        if let Some(tp) = doc.tools.get(name) {
            if !tp.allow {
                return PolicyDecision::Deny {
                    reason: "tool denied by policy".into(),
                    source_scope: doc.scope.clone(),
                };
            }
        }
    }

    // Stage 5 — Approval condition
    if let aa_core::GovernanceAction::ToolCall { name, .. } = action {
        if let Some(tp) = doc.tools.get(name) {
            if let Some(expr) = &tp.requires_approval_if {
                if !expr.is_empty() && crate::policy::expr::evaluate(expr, action, Some(ctx.governance_level)) {
                    return PolicyDecision::RequireApproval {
                        reason: format!("approval required for tool '{name}'"),
                        timeout_secs: doc.approval_timeout_secs,
                    };
                }
            }
        }
    }

    PolicyDecision::Allow
}

/// Merge a cascade of policy documents into a single `PolicyDecision` using
/// most-restrictive-wins semantics: `Deny > RequireApproval > Allow`.
///
/// Rules:
/// - Any `Deny` from any scope short-circuits immediately and is returned.
/// - If no `Deny` and any `RequireApproval` exists, return the first
///   `RequireApproval` encountered (broadest-scope wins for approval reason).
/// - If all policies say `Allow`, return `Allow`.
/// - An empty cascade returns a fail-closed `Deny` — never silently allow.
///
/// Stages 4 (rate-limit) and 7 (budget) must be applied by the caller after
/// this function returns `Allow`.
pub fn merge_decisions(
    cascade: &[Arc<PolicyDocument>],
    ctx: &aa_core::AgentContext,
    action: &aa_core::GovernanceAction,
) -> PolicyDecision {
    if cascade.is_empty() {
        return PolicyDecision::Deny {
            reason: "no policy — fail-closed".into(),
            source_scope: PolicyScope::Global,
        };
    }

    let mut running = PolicyDecision::Allow;

    for doc in cascade {
        let verdict = evaluate_single_doc(doc, ctx, action);
        match verdict {
            // Short-circuit: Deny always wins.
            PolicyDecision::Deny { .. } => return verdict,
            // Upgrade from Allow to RequireApproval, but don't downgrade RequireApproval.
            PolicyDecision::RequireApproval { .. } => {
                if matches!(running, PolicyDecision::Allow) {
                    running = verdict;
                }
            }
            PolicyDecision::Allow => {}
        }
    }

    running
}
