//! Error types for eBPF program lifecycle operations.

use core::fmt;

/// Errors that can occur during eBPF program loading, attachment, and operation.
#[derive(Debug)]
pub enum EbpfError {
    /// Failed to load the compiled eBPF bytecode into the kernel.
    ProgramLoad(String),
    /// Failed to attach a kprobe to the target syscall.
    ProbeAttach(String),
    /// Failed to update a BPF map from userspace.
    MapUpdate(String),
    /// Failed to parse an event received from the BPF ring buffer.
    EventParse(String),
}

impl fmt::Display for EbpfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProgramLoad(msg) => write!(f, "eBPF program load failed: {msg}"),
            Self::ProbeAttach(msg) => write!(f, "kprobe attach failed: {msg}"),
            Self::MapUpdate(msg) => write!(f, "BPF map update failed: {msg}"),
            Self::EventParse(msg) => write!(f, "event parse failed: {msg}"),
        }
    }
}

impl std::error::Error for EbpfError {}
