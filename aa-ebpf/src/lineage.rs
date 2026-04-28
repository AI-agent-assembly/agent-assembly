//! PID ancestry tracking for agent process families.
//!
//! Maintains a userspace mirror of the in-kernel PID lineage map, allowing
//! the runtime to query parent-child relationships up to 5 levels deep.
//! Populated by [`ProcessSpawnEvent`](crate::events::ProcessSpawnEvent)s
//! received from the eBPF ring buffer.

/// Tracks the process lineage tree for monitored agent families.
///
/// Receives [`ProcessSpawnEvent`](crate::events::ProcessSpawnEvent)s from
/// the eBPF ring buffer and maintains an in-memory parent-child map.
pub struct ProcessLineageTracker {
    _private: (),
}
