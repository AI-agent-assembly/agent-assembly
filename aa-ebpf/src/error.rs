//! Error types for the aa-ebpf userspace loader.

use thiserror::Error;

/// Errors that can occur while loading, attaching, or reading eBPF programs.
#[derive(Debug, Error)]
pub enum EbpfError {
    /// Failed to load the eBPF program ELF object.
    #[error("failed to load eBPF object: {0}")]
    Load(#[from] aya::EbpfError),

    /// Failed to attach an uprobe or kprobe to a target symbol.
    #[error("failed to attach probe to `{symbol}`: {source}")]
    Attach {
        /// Name of the target symbol (e.g. `SSL_write`).
        symbol: String,
        /// Underlying aya error.
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
}
