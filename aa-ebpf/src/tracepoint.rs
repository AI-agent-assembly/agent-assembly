//! Tracepoint management for process exec monitoring (AAASM-39).
//!
//! Attaches the `sched_process_exec` tracepoint from `aa-ebpf-programs` to
//! the Linux `sched/sched_process_exec` kernel tracepoint.

use aya::Ebpf;

use crate::error::EbpfError;

/// Attaches and manages the `sched_process_exec` tracepoint program.
///
/// Create via [`TracepointManager::attach`]. The tracepoint stays active
/// until the `TracepointManager` is dropped.
#[allow(dead_code)]
pub struct TracepointManager;

impl TracepointManager {
    /// Attach the `sched/sched_process_exec` tracepoint program.
    ///
    /// This tracepoint fires for every `execve`/`execveat` syscall on the
    /// system, regardless of whether the process imported the aa SDK.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::Attach`] if the tracepoint category or name
    /// is not available on the running kernel.
    ///
    /// # Arguments
    ///
    /// * `bpf` — live [`Ebpf`] handle from [`crate::loader::EbpfLoader::load`].
    pub fn attach(bpf: &mut Ebpf) -> Result<Self, EbpfError> {
        // TODO(AAASM-39): attach sched_process_exec tracepoint.
        let _ = bpf;
        todo!("attach sched_process_exec tracepoint")
    }
}
