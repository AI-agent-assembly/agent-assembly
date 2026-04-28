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
        Self {
            nodes: HashMap::new(),
        }
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
