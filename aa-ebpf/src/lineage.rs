//! PID ancestry tracking for agent process families.
//!
//! Maintains a userspace mirror of process lineage, allowing the runtime
//! to query parent-child relationships up to [`MAX_LINEAGE_DEPTH`] levels.
//! Populated by [`ExecEvent`](aa_ebpf_common::exec::ExecEvent)s received
//! from the eBPF ring buffer.

use std::collections::HashMap;

/// Maximum depth of ancestry walks.
pub const MAX_LINEAGE_DEPTH: usize = 5;

/// A node in the process lineage tree.
#[derive(Debug, Clone)]
pub struct LineageNode {
    /// Process ID.
    pub pid: u32,
    /// Parent process ID.
    pub ppid: u32,
    /// Executable filename (null-terminated bytes decoded to String).
    pub filename: String,
    /// Kernel timestamp (nanoseconds) when the exec event was observed.
    pub timestamp_ns: u64,
}

/// Tracks the process lineage tree for monitored agent families.
///
/// Receives exec events from the eBPF ring buffer and maintains an
/// in-memory parent-child map. Supports ancestry walks up to
/// [`MAX_LINEAGE_DEPTH`] levels and process exit cleanup.
pub struct ProcessLineageTracker {
    /// Maps PID → lineage node.
    nodes: HashMap<u32, LineageNode>,
}

impl ProcessLineageTracker {
    /// Create an empty lineage tracker.
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
    }

    /// Record a new process exec event.
    ///
    /// Inserts or updates the lineage entry for the given PID.
    pub fn insert(&mut self, pid: u32, ppid: u32, filename: String, timestamp_ns: u64) {
        self.nodes.insert(
            pid,
            LineageNode {
                pid,
                ppid,
                filename,
                timestamp_ns,
            },
        );
    }

    /// Remove a PID from the lineage map (called on process exit).
    ///
    /// Returns the removed node, if it existed.
    pub fn remove(&mut self, pid: u32) -> Option<LineageNode> {
        self.nodes.remove(&pid)
    }

    /// Look up a single node by PID.
    pub fn get(&self, pid: u32) -> Option<&LineageNode> {
        self.nodes.get(&pid)
    }

    /// Walk the ancestry chain from `pid` up to [`MAX_LINEAGE_DEPTH`] levels.
    ///
    /// Returns a vec of `(pid, ppid, filename)` starting from the given PID
    /// and walking upward through parents. Stops when:
    /// - The parent is not in the map (unknown ancestor).
    /// - The depth limit is reached.
    /// - A cycle is detected (pid == ppid).
    pub fn ancestry(&self, pid: u32) -> Vec<&LineageNode> {
        let mut result = Vec::with_capacity(MAX_LINEAGE_DEPTH);
        let mut current = pid;

        for _ in 0..MAX_LINEAGE_DEPTH {
            match self.nodes.get(&current) {
                Some(node) => {
                    result.push(node);
                    if node.ppid == current || node.ppid == 0 {
                        break;
                    }
                    current = node.ppid;
                }
                None => break,
            }
        }

        result
    }

    /// Check whether `descendant_pid` is a descendant of `ancestor_pid`
    /// within [`MAX_LINEAGE_DEPTH`] levels.
    pub fn is_descendant_of(&self, descendant_pid: u32, ancestor_pid: u32) -> bool {
        let mut current = descendant_pid;

        for _ in 0..MAX_LINEAGE_DEPTH {
            if current == ancestor_pid {
                return true;
            }
            match self.nodes.get(&current) {
                Some(node) => {
                    if node.ppid == current || node.ppid == 0 {
                        return false;
                    }
                    current = node.ppid;
                }
                None => return false,
            }
        }

        false
    }

    /// Return the number of tracked processes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Return whether the tracker is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for ProcessLineageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracker_with_chain() -> ProcessLineageTracker {
        // Build a 4-level chain: init(1) → agent(100) → python(200) → curl(300)
        let mut t = ProcessLineageTracker::new();
        t.insert(1, 0, "/sbin/init".into(), 1000);
        t.insert(100, 1, "/usr/bin/agent".into(), 2000);
        t.insert(200, 100, "/usr/bin/python".into(), 3000);
        t.insert(300, 200, "/usr/bin/curl".into(), 4000);
        t
    }

    #[test]
    fn insert_and_get() {
        let mut t = ProcessLineageTracker::new();
        t.insert(42, 1, "/bin/bash".into(), 1000);

        let node = t.get(42).unwrap();
        assert_eq!(node.pid, 42);
        assert_eq!(node.ppid, 1);
        assert_eq!(node.filename, "/bin/bash");
        assert_eq!(node.timestamp_ns, 1000);
    }

    #[test]
    fn get_missing_returns_none() {
        let t = ProcessLineageTracker::new();
        assert!(t.get(999).is_none());
    }

    #[test]
    fn remove_returns_node() {
        let mut t = ProcessLineageTracker::new();
        t.insert(42, 1, "/bin/bash".into(), 1000);

        let removed = t.remove(42).unwrap();
        assert_eq!(removed.pid, 42);
        assert!(t.get(42).is_none());
    }

    #[test]
    fn remove_missing_returns_none() {
        let mut t = ProcessLineageTracker::new();
        assert!(t.remove(999).is_none());
    }

    #[test]
    fn ancestry_walks_full_chain() {
        let t = tracker_with_chain();
        let chain = t.ancestry(300);

        assert_eq!(chain.len(), 4);
        assert_eq!(chain[0].pid, 300);
        assert_eq!(chain[1].pid, 200);
        assert_eq!(chain[2].pid, 100);
        assert_eq!(chain[3].pid, 1);
    }

    #[test]
    fn ancestry_stops_at_unknown_parent() {
        let mut t = ProcessLineageTracker::new();
        t.insert(500, 999, "/bin/orphan".into(), 5000);

        let chain = t.ancestry(500);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].pid, 500);
    }

    #[test]
    fn ancestry_stops_at_depth_limit() {
        // Build a chain longer than MAX_LINEAGE_DEPTH (5).
        let mut t = ProcessLineageTracker::new();
        for i in 1..=8 {
            t.insert(i, i.saturating_sub(1), format!("/proc/{i}"), i as u64 * 1000);
        }

        let chain = t.ancestry(8);
        assert_eq!(chain.len(), MAX_LINEAGE_DEPTH);
    }

    #[test]
    fn ancestry_handles_self_parent_cycle() {
        let mut t = ProcessLineageTracker::new();
        t.insert(1, 1, "/sbin/init".into(), 1000);

        let chain = t.ancestry(1);
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn is_descendant_of_direct_parent() {
        let t = tracker_with_chain();
        assert!(t.is_descendant_of(300, 200));
    }

    #[test]
    fn is_descendant_of_grandparent() {
        let t = tracker_with_chain();
        assert!(t.is_descendant_of(300, 100));
    }

    #[test]
    fn is_descendant_of_self() {
        let t = tracker_with_chain();
        assert!(t.is_descendant_of(300, 300));
    }

    #[test]
    fn is_not_descendant_of_unrelated() {
        let mut t = tracker_with_chain();
        t.insert(999, 0, "/unrelated".into(), 9000);
        assert!(!t.is_descendant_of(300, 999));
    }

    #[test]
    fn len_and_is_empty() {
        let mut t = ProcessLineageTracker::new();
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);

        t.insert(1, 0, "/init".into(), 1000);
        assert!(!t.is_empty());
        assert_eq!(t.len(), 1);
    }
}
