//! Event types emitted by eBPF probes.
//!
//! Re-exports shared event types from [`aa_ebpf_common`] and defines
//! userspace-only event types for file I/O kprobes.

pub use aa_ebpf_common::{
    AlertLevel, ProcessNode, ProcessSpawnEvent, ShellInjectionAlert, MAX_ARGV_ENTRIES,
    MAX_ARGV_LEN, MAX_EXECUTABLE_LEN,
};

use crate::syscall::SyscallKind;

/// A file I/O event captured by a kprobe.
///
/// Each event represents a single syscall interception with the metadata
/// needed to evaluate governance policies (PID lineage, file path, flags).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIoEvent {
    /// Process ID of the intercepted syscall.
    pub pid: u32,
    /// Thread ID of the intercepted syscall.
    pub tid: u32,
    /// Kernel timestamp in nanoseconds (from `bpf_ktime_get_ns`).
    pub timestamp_ns: u64,
    /// Which syscall was intercepted.
    pub syscall: SyscallKind,
    /// File path associated with the syscall.
    pub path: String,
    /// Syscall-specific flags (e.g., `O_RDONLY` for `openat`).
    pub flags: u32,
    /// Syscall return code (`0` for success on entry probes).
    pub return_code: i64,
}
