//! PID lineage tracker for causal group membership.
//!
//! Maintains a mapping of child PID → parent PID so the correlation engine
//! can determine whether two processes belong to the same causal group
//! (i.e., the SDK process and all of its descendants).

use std::collections::{HashMap, HashSet};

/// Tracks PID-to-parent relationships for causal group membership.
///
/// The SDK process and all its descendant PIDs form a single causal group.
/// When the correlation engine sees an intent from PID A and an action from
/// PID B, it uses this tracker to determine whether B is a descendant of A
/// (or vice versa).
#[derive(Debug, Default)]
pub struct PidLineage {
    /// Maps child PID → parent PID.
    parent_map: HashMap<u32, u32>,
}

impl PidLineage {
    /// Create a new empty lineage tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a parent-child PID relationship.
    pub fn register(&mut self, child_pid: u32, parent_pid: u32) {
        self.parent_map.insert(child_pid, parent_pid);
    }

    /// Returns `true` if `pid_a` and `pid_b` belong to the same causal group
    /// (i.e., one is an ancestor of the other, or they share a common ancestor).
    ///
    /// Walks the ancestor chain of `pid_a` up to `max_depth` steps, collecting
    /// all ancestors into a set, then walks `pid_b`'s chain checking for overlap.
    /// A cycle guard (`max_depth`) prevents infinite loops in malformed data.
    pub fn is_same_family(&self, pid_a: u32, pid_b: u32) -> bool {
        if pid_a == pid_b {
            return true;
        }

        const MAX_DEPTH: usize = 64;

        // Collect all ancestors of pid_a (including pid_a itself).
        let mut ancestors_a = HashSet::new();
        ancestors_a.insert(pid_a);
        let mut current = pid_a;
        for _ in 0..MAX_DEPTH {
            match self.parent_map.get(&current) {
                Some(&parent) if !ancestors_a.contains(&parent) => {
                    ancestors_a.insert(parent);
                    current = parent;
                }
                _ => break,
            }
        }

        // Walk pid_b's ancestor chain, checking for overlap with ancestors_a.
        if ancestors_a.contains(&pid_b) {
            return true;
        }
        current = pid_b;
        for _ in 0..MAX_DEPTH {
            match self.parent_map.get(&current) {
                Some(&parent) => {
                    if ancestors_a.contains(&parent) {
                        return true;
                    }
                    current = parent;
                }
                None => break,
            }
        }

        false
    }

    /// Remove a PID from the lineage tracker (e.g., after process exit).
    pub fn remove(&mut self, pid: u32) {
        self.parent_map.remove(&pid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_lineage_is_empty() {
        let lineage = PidLineage::new();
        assert!(lineage.parent_map.is_empty());
    }

    #[test]
    fn register_adds_entry() {
        let mut lineage = PidLineage::new();
        lineage.register(100, 1);
        assert_eq!(lineage.parent_map.get(&100), Some(&1));
    }

    #[test]
    fn remove_deletes_entry() {
        let mut lineage = PidLineage::new();
        lineage.register(100, 1);
        lineage.remove(100);
        assert!(!lineage.parent_map.contains_key(&100));
    }

    #[test]
    fn same_pid_is_same_family() {
        let lineage = PidLineage::new();
        assert!(lineage.is_same_family(42, 42));
    }

    #[test]
    fn parent_child_is_same_family() {
        let mut lineage = PidLineage::new();
        // 100 is child of 1
        lineage.register(100, 1);
        assert!(lineage.is_same_family(1, 100));
        assert!(lineage.is_same_family(100, 1));
    }

    #[test]
    fn grandparent_grandchild_is_same_family() {
        let mut lineage = PidLineage::new();
        // 1 → 100 → 200
        lineage.register(100, 1);
        lineage.register(200, 100);
        assert!(lineage.is_same_family(1, 200));
        assert!(lineage.is_same_family(200, 1));
    }

    #[test]
    fn shared_ancestor_is_same_family() {
        let mut lineage = PidLineage::new();
        // 1 → 100, 1 → 200 (siblings)
        lineage.register(100, 1);
        lineage.register(200, 1);
        assert!(lineage.is_same_family(100, 200));
        assert!(lineage.is_same_family(200, 100));
    }

    #[test]
    fn unrelated_pids_are_not_same_family() {
        let mut lineage = PidLineage::new();
        // Two separate trees: 1→100 and 2→200
        lineage.register(100, 1);
        lineage.register(200, 2);
        assert!(!lineage.is_same_family(100, 200));
    }

    #[test]
    fn unknown_pids_are_not_same_family() {
        let lineage = PidLineage::new();
        assert!(!lineage.is_same_family(1, 2));
    }
}
