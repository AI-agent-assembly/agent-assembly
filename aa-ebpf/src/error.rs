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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_program_load() {
        let err = EbpfError::ProgramLoad("missing privileges".into());
        assert_eq!(err.to_string(), "eBPF program load failed: missing privileges");
    }

    #[test]
    fn display_probe_attach() {
        let err = EbpfError::ProbeAttach("sys_openat not found".into());
        assert_eq!(err.to_string(), "kprobe attach failed: sys_openat not found");
    }

    #[test]
    fn display_map_update() {
        let err = EbpfError::MapUpdate("map full".into());
        assert_eq!(err.to_string(), "BPF map update failed: map full");
    }

    #[test]
    fn display_event_parse() {
        let err = EbpfError::EventParse("truncated buffer".into());
        assert_eq!(err.to_string(), "event parse failed: truncated buffer");
    }

    #[test]
    fn implements_std_error() {
        let err = EbpfError::ProgramLoad("test".into());
        let _: &dyn std::error::Error = &err;
    }
}
