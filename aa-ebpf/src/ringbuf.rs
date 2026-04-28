//! BPF ring-buffer consumer: reads events from kernel-space to userspace.
//!
//! All three eBPF sub-tasks (AAASM-37, 38, 39) emit events through a shared
//! BPF ring buffer map.  `RingBufReader` multiplexes the three event types
//! and dispatches them to registered callbacks.

use aya::Bpf;
use aa_ebpf_common::{exec::ExecEvent, file::FileEvent, tls::TlsCaptureEvent};

use crate::error::EbpfError;

/// Dispatched event variants read from the shared BPF ring buffer.
#[derive(Debug)]
pub enum EbpfEvent {
    /// TLS plaintext capture (AAASM-37).
    Tls(TlsCaptureEvent),
    /// File I/O operation (AAASM-38).
    File(FileEvent),
    /// Process exec (AAASM-39).
    Exec(ExecEvent),
}

/// Async consumer that reads [`EbpfEvent`]s from the BPF ring buffer.
///
/// Create via [`RingBufReader::new`], then poll with [`RingBufReader::next`]
/// inside a Tokio task.
#[allow(dead_code)]
pub struct RingBufReader {
    bpf: Bpf,
}

impl RingBufReader {
    /// Construct a `RingBufReader` from a loaded [`Bpf`] handle.
    ///
    /// Looks up the `EVENTS` ring buffer map in the loaded object.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::MapNotFound`] if the `EVENTS` map is absent.
    pub fn new(bpf: Bpf) -> Result<Self, EbpfError> {
        // TODO(AAASM-37/38/39): obtain AsyncRingBuf handle from the EVENTS map.
        Ok(Self { bpf })
    }

    /// Read the next event from the ring buffer (async).
    ///
    /// Returns `None` when the ring buffer has been closed (loader shut down).
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::EventSize`] if the raw bytes cannot be
    /// interpreted as a known event type.
    pub async fn next(&mut self) -> Result<Option<EbpfEvent>, EbpfError> {
        // TODO(AAASM-37/38/39): await next raw bytes, discriminate by size,
        // cast to the correct event type, and return.
        todo!("read next event from BPF ring buffer")
    }
}
