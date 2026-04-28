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
