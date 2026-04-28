//! Error types for the aa-ebpf userspace loader.

use thiserror::Error;

/// Errors that can occur while loading, attaching, or reading eBPF programs.
#[derive(Debug, Error)]
pub enum EbpfError {
    /// Failed to load the eBPF program ELF object.
    #[cfg(target_os = "linux")]
    #[error("failed to load eBPF object: {0}")]
    Load(#[from] aya::EbpfError),

    /// An eBPF map operation failed (e.g. writing to a PID filter map).
    #[cfg(target_os = "linux")]
    #[error("eBPF map operation failed: {0}")]
    Map(#[from] aya::maps::MapError),

    /// An eBPF program operation failed (e.g. load or attach).
    #[cfg(target_os = "linux")]
    #[error("eBPF program operation failed: {0}")]
    Program(#[from] aya::programs::ProgramError),

    /// Failed to attach an uprobe or kprobe to a target symbol.
    #[error("failed to attach probe to `{symbol}`: {source}")]
    Attach {
        /// Name of the target symbol (e.g. `SSL_write`).
        symbol: String,
        /// Underlying error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// The ring buffer returned an unexpected event size.
    #[error("ring buffer event size mismatch: expected {expected}, got {got}")]
    EventSize {
        /// Expected byte length.
        expected: usize,
        /// Actual byte length received.
        got: usize,
    },

    /// A required eBPF map was not found in the loaded object.
    #[error("eBPF map `{name}` not found in object")]
    MapNotFound {
        /// Name of the missing map.
        name: String,
    },

    /// OpenSSL shared library could not be located for the target process.
    #[error("could not find OpenSSL library for pid {pid:?}")]
    OpenSslNotFound {
        /// Target PID, or `None` for system-wide search.
        pid: Option<i32>,
    },

    /// An I/O error occurred during async ring-buffer polling or /proc parsing.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
