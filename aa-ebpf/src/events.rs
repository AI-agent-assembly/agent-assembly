//! Event types emitted by eBPF probes.
//!
//! Re-exports shared event types from [`aa_ebpf_common`] and defines
//! userspace-only event types for file I/O kprobes.

pub use aa_ebpf_common::{
    AlertLevel, ProcessNode, ProcessSpawnEvent, ShellInjectionAlert, MAX_ARGV_ENTRIES,
    MAX_ARGV_LEN, MAX_EXECUTABLE_LEN,
};

use crate::error::EbpfError;
use crate::syscall::SyscallKind;
use aa_ebpf_common::{FileIoEventRaw, SyscallType};

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
    /// Whether this event matched a blocklisted path (flags bit 0).
    pub is_sensitive: bool,
}

impl FileIoEvent {
    /// Parse a [`FileIoEventRaw`] received from the BPF perf event array
    /// into a userspace-friendly [`FileIoEvent`].
    pub fn from_raw(raw: &FileIoEventRaw) -> Result<Self, EbpfError> {
        let syscall = match raw.syscall {
            SyscallType::Openat => SyscallKind::Openat,
            SyscallType::Read => SyscallKind::Read,
            SyscallType::Write => SyscallKind::Write,
            SyscallType::Unlink => SyscallKind::Unlink,
            SyscallType::Rename => SyscallKind::Rename,
        };

        // Extract the null-terminated path from the fixed-size buffer.
        let nul_pos = raw.path.iter().position(|&b| b == 0).unwrap_or(raw.path.len());
        let path = core::str::from_utf8(&raw.path[..nul_pos])
            .map_err(|e| EbpfError::EventParse(format!("invalid UTF-8 in path: {e}")))?
            .to_string();

        Ok(Self {
            pid: raw.pid,
            tid: raw.tid,
            timestamp_ns: raw.timestamp_ns,
            syscall,
            path,
            flags: raw.flags,
            return_code: raw.return_code,
            is_sensitive: raw.flags & 1 != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_io_event_construction() {
        let event = FileIoEvent {
            pid: 1234,
            tid: 1234,
            timestamp_ns: 999_999,
            syscall: SyscallKind::Openat,
            path: "/etc/shadow".into(),
            flags: 0,
            return_code: 0,
        };
        assert_eq!(event.pid, 1234);
        assert_eq!(event.syscall, SyscallKind::Openat);
        assert_eq!(event.path, "/etc/shadow");
    }
}
