//! Re-exports of shared eBPF event types for downstream consumers.
//!
//! All event structs live in [`aa_ebpf_common`] so they can be shared with
//! the kernel-space eBPF programs. This module re-exports them for
//! convenience so userspace code can import directly from `aa_ebpf`.

pub use aa_ebpf_common::exec::{
    AlertLevel, ProcessNode, ProcessSpawnEvent, ShellInjectionAlert, MAX_ARGV_ENTRIES,
    MAX_ARGV_LEN, MAX_EXECUTABLE_LEN,
};
