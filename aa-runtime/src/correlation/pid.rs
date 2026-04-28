//! PID lineage tracker for causal group membership.
//!
//! Maintains a mapping of child PID → parent PID so the correlation engine
//! can determine whether two processes belong to the same causal group
//! (i.e., the SDK process and all of its descendants).

use std::collections::HashMap;

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
    pub fn is_same_family(&self, _pid_a: u32, _pid_b: u32) -> bool {
        todo!("implement ancestor walk using parent_map")
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
        assert!(lineage.parent_map.get(&100).is_none());
    }
}
