//! Dry-run policy evaluation engine.

use crate::policy::document::PolicyDocument;

/// A simulation engine that evaluates events against a policy without enforcing decisions.
///
/// Created by cloning a `PolicyDocument` with `dry_run: true` semantics —
/// no audit log writes, no alert triggers, no approval queue entries.
pub struct SimulationEngine {
    /// The policy document to evaluate against.
    policy: PolicyDocument,
}

impl SimulationEngine {
    /// Create a new simulation engine for the given policy.
    pub fn new(policy: PolicyDocument) -> Self {
        Self { policy }
    }

    /// Returns a reference to the loaded policy document.
    pub fn policy(&self) -> &PolicyDocument {
        &self.policy
    }
}
