//! Kprobe management for file I/O interception (AAASM-38).
//!
//! Attaches `openat_kprobe`, `write_kprobe`, and `unlink_kprobe` from
//! `aa-ebpf-programs` to the corresponding kernel functions, filtered by
//! the target PID stored in a BPF map.

#[cfg(target_os = "linux")]
use aya::Ebpf;

use crate::error::EbpfError;

/// Attaches and manages file I/O kprobe programs.
///
/// Create via [`KprobeManager::attach`]. The probes stay active until the
/// `KprobeManager` is dropped.
pub struct KprobeManager {
    /// Target PID to filter inside the eBPF program.
    target_pid: Option<i32>,
    /// Live kprobe link handles. Dropping them detaches the probes from the
    /// kernel. Stored as type-erased `Box<dyn Any>` to avoid coupling to
    /// aya's internal link-id type (matches `UprobeManager` convention).
    #[cfg(target_os = "linux")]
    _links: Vec<Box<dyn std::any::Any>>,
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
    /// * `bpf` — live [`Ebpf`] handle from [`crate::loader::EbpfLoader::load`].
    /// * `target_pid` — PID to filter, or `None` for system-wide monitoring.
    #[cfg(target_os = "linux")]
    pub fn attach(bpf: &mut Ebpf, target_pid: Option<i32>) -> Result<Self, EbpfError> {
        // TODO(AAASM-38): attach openat_kprobe, write_kprobe, unlink_kprobe
        // and write target_pid into the PID filter BPF map.
        let _ = (bpf, target_pid);
        todo!("attach file I/O kprobes")
    }
}
