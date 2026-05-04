//! Scope-keyed index of loaded policies (`PolicyId` ↔ `PolicyScope`).
//!
//! Built for AAASM-951 (F92 Phase B). Stores policy documents alongside a
//! map from each [`crate::policy::PolicyScope`] to the list of policy ids
//! loaded under that scope, in insertion order, so the cascading evaluator
//! added by AAASM-220 (F93) can resolve applicable policies in O(1).

use std::collections::HashMap;
use std::sync::Arc;

use crate::policy::{PolicyDocument, PolicyScope};

/// Opaque identifier returned by [`ScopeIndex::insert`] (and by
/// [`crate::engine::PolicyEngine::load_policy`] in turn).
///
/// Monotonically increasing within a single `ScopeIndex` instance, but
/// callers must treat the inner value as opaque — it is not stable
/// across processes and not suitable as a database key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PolicyId(u64);

impl PolicyId {
    /// Construct a `PolicyId` from a raw counter value. Intended for tests
    /// and for `ScopeIndex` itself; production callers should obtain ids
    /// from `ScopeIndex::insert`.
    #[inline]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Return the raw counter value of this id.
    #[inline]
    pub const fn as_raw(&self) -> u64 {
        self.0
    }
}

/// Owns loaded policy documents and a secondary index from
/// [`PolicyScope`] to the list of [`PolicyId`]s registered under that
/// scope, preserving insertion order within each bucket.
///
/// Phase B (this Sub-task) only populates the index; the cascading
/// evaluator that *consumes* it lands in F93 (AAASM-220).
#[derive(Debug, Default)]
pub struct ScopeIndex {
    /// Owned policy documents keyed by their assigned id.
    policies: HashMap<PolicyId, Arc<PolicyDocument>>,
    /// Per-scope insertion-ordered list of policy ids.
    by_scope: HashMap<PolicyScope, Vec<PolicyId>>,
    /// Monotonic counter feeding new [`PolicyId`] values.
    next_id: u64,
}
