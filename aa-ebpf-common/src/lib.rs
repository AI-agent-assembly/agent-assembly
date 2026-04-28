//! Shared types between eBPF kernel-space probes and userspace loader.
//!
//! All types in this crate use fixed-size representations compatible with
//! eBPF maps. This crate is `no_std` so it can be compiled for both the
//! `bpfel-unknown-none` BPF target and standard userspace targets.

#![no_std]

// ── AAASM-39: Process lineage types ──────────────────────────────────

/// In-kernel node for the PID lineage map (`BpfHashMap<u32, ProcessNode>`).
///
/// Each entry maps a child PID to its parent and the command that spawned it.
/// Fixed-size layout is required for eBPF map compatibility.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessNode {
    /// Process ID of the child.
    pub pid: u32,
    /// Parent process ID.
    pub ppid: u32,
    /// Command name (`comm`) of the process, null-padded.
    pub comm: [u8; 16],
    /// Kernel timestamp in nanoseconds when the process was spawned.
    pub spawn_time_ns: u64,
}

/// Maximum number of argv entries captured per execve event.
pub const MAX_ARGV_ENTRIES: usize = 5;

/// Maximum byte length of a single argv entry.
pub const MAX_ARGV_LEN: usize = 128;

/// Event emitted from the eBPF tracepoint to userspace on each `execve` call.
///
/// Sent via ring buffer or perf event array. Fixed-size layout ensures
/// predictable memory use in the eBPF program.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessSpawnEvent {
    /// Process ID of the newly spawned process.
    pub pid: u32,
    /// Parent process ID.
    pub ppid: u32,
    /// Command name (`comm`) of the new process, null-padded.
    pub comm: [u8; 16],
    /// First [`MAX_ARGV_ENTRIES`] argv entries, each null-padded to [`MAX_ARGV_LEN`] bytes.
    pub argv: [[u8; MAX_ARGV_LEN]; MAX_ARGV_ENTRIES],
    /// Number of environment variables passed to execve.
    pub env_count: u32,
    /// Kernel timestamp in nanoseconds.
    pub timestamp_ns: u64,
}

/// Maximum byte length of the executable path in a [`ShellInjectionAlert`].
pub const MAX_EXECUTABLE_LEN: usize = 256;

/// Severity level for a shell injection alert.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    /// Informational — known-benign shell spawns (e.g. build scripts).
    Info = 0,
    /// Warning — potentially suspicious spawn (e.g. `python`, `node`).
    Warning = 1,
    /// Critical — high-risk spawn (e.g. `curl`, `wget`, raw `sh`/`bash`).
    Critical = 2,
}

/// Alert emitted when an agent process spawns a suspicious child process.
///
/// Generated in the eBPF program when the spawned executable matches a
/// known shell or download utility pattern (e.g. `bash`, `curl`, `wget`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ShellInjectionAlert {
    /// PID of the monitored agent (or its ancestor in the lineage tree).
    pub parent_pid: u32,
    /// PID of the suspicious child process.
    pub child_pid: u32,
    /// Full executable path of the child, null-padded.
    pub executable: [u8; MAX_EXECUTABLE_LEN],
    /// Severity of the alert.
    pub alert_level: AlertLevel,
    /// Kernel timestamp in nanoseconds.
    pub timestamp_ns: u64,
}

// ── AAASM-38: File I/O types ─────────────────────────────────────────

/// Identifies which file-related syscall was intercepted.
///
/// Uses `#[repr(u32)]` for BPF compatibility (BPF maps require 4-byte alignment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SyscallType {
    /// `sys_openat` — open or create a file.
    Openat = 0,
    /// `sys_read` — read from a file descriptor.
    Read = 1,
    /// `sys_write` — write to a file descriptor.
    Write = 2,
    /// `sys_unlink` — delete a file.
    Unlink = 3,
    /// `sys_rename` — rename or move a file.
    Rename = 4,
}
