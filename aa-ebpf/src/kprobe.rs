//! Kprobe management for file I/O interception (AAASM-38).
//!
//! Attaches `openat_kprobe`, `write_kprobe`, and `unlink_kprobe` from
//! `aa-ebpf-programs` to the corresponding kernel functions, filtered by
//! the target PID stored in a BPF map.

use aya::Bpf;

use crate::error::EbpfError;

/// Attaches and manages file I/O kprobe programs.
///
/// Create via [`KprobeManager::attach`]. The probes stay active until the
/// `KprobeManager` is dropped.
#[allow(dead_code)]
pub struct KprobeManager {
    /// Target PID to filter inside the eBPF program.
    target_pid: Option<i32>,
}

impl KprobeManager {
    /// Attach file I/O kprobes (`openat`, `write`, `unlink`) for the target PID.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::Attach`] if a kernel symbol cannot be found
    /// (e.g., the running kernel uses a different internal function name).
    ///
    /// # Arguments
    ///
    /// * `bpf` — live [`Bpf`] handle from [`crate::loader::EbpfLoader::load`].
    /// * `target_pid` — PID to filter, or `None` for system-wide monitoring.
    pub fn attach(bpf: &mut Bpf, target_pid: Option<i32>) -> Result<Self, EbpfError> {
        // TODO(AAASM-38): attach openat_kprobe, write_kprobe, unlink_kprobe
        // and write target_pid into the PID filter BPF map.
        let _ = (bpf, target_pid);
        todo!("attach file I/O kprobes")
    }
}
