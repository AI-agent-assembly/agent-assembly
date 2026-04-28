//! Shared types between eBPF kernel-space probes and userspace loader.
//!
//! All types in this crate use fixed-size representations compatible with
//! eBPF maps. This crate is `no_std` so it can be compiled for both the
//! `bpfel-unknown-none` BPF target and standard userspace targets.

#![no_std]

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
