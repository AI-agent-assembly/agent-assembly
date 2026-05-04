//! Scope-keyed index of loaded policies (`PolicyId` ↔ `PolicyScope`).
//!
//! Built for AAASM-951 (F92 Phase B). Stores policy documents alongside a
//! map from each [`crate::policy::PolicyScope`] to the list of policy ids
//! loaded under that scope, in insertion order, so the cascading evaluator
//! added by AAASM-220 (F93) can resolve applicable policies in O(1).
