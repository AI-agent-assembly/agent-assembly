//! BPF ring-buffer consumer: reads events from kernel-space to userspace.
//!
//! All three eBPF sub-tasks (AAASM-37, 38, 39) emit events through a shared
//! BPF ring buffer map.  `RingBufReader` multiplexes the three event types
//! and dispatches them to registered callbacks.

use std::mem;

use aa_ebpf_common::{exec::ExecEvent, file::FileEvent, tls::TlsCaptureEvent};
use aya::Ebpf;

use crate::error::EbpfError;

/// Dispatched event variants read from the shared BPF ring buffer.
#[derive(Debug)]
pub enum EbpfEvent {
    /// TLS plaintext capture (AAASM-37).
    Tls(Box<TlsCaptureEvent>),
    /// File I/O operation (AAASM-38).
    File(Box<FileEvent>),
    /// Process exec (AAASM-39).
    Exec(Box<ExecEvent>),
}

/// Async consumer that reads [`EbpfEvent`]s from the BPF ring buffer.
///
/// Create via [`RingBufReader::new`], then poll with [`RingBufReader::next`]
/// inside a Tokio task.
#[allow(dead_code)]
pub struct RingBufReader {
    bpf: Ebpf,
}

impl RingBufReader {
    /// Construct a `RingBufReader` from a loaded [`Ebpf`] handle.
    ///
    /// Looks up the `EVENTS` ring buffer map in the loaded object.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::MapNotFound`] if the `EVENTS` map is absent.
    pub fn new(bpf: Ebpf) -> Result<Self, EbpfError> {
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

/// Discriminate a raw byte slice by size and copy it into an owned event.
///
/// Sizes (from `#[repr(C)]` layout):
/// - [`TlsCaptureEvent`]: 4 + 8 + 4 + 4 + 4 + 4 + 1 + 7 + 4096 = 4112 bytes
/// - [`FileEvent`]:  8 + 4 + 4 + 4 + 4 + 3 + 256 = 283... see struct for exact
/// - [`ExecEvent`]:  8 + 4 + 4 + 4 + 4 + 256 + 512 = 792 bytes
fn parse_event(bytes: &[u8]) -> Result<EbpfEvent, EbpfError> {
    match bytes.len() {
        n if n == mem::size_of::<TlsCaptureEvent>() => {
            Ok(EbpfEvent::Tls(Box::new(bytes_to::<TlsCaptureEvent>(bytes))))
        }
        n if n == mem::size_of::<FileEvent>() => {
            Ok(EbpfEvent::File(Box::new(bytes_to::<FileEvent>(bytes))))
        }
        n if n == mem::size_of::<ExecEvent>() => {
            Ok(EbpfEvent::Exec(Box::new(bytes_to::<ExecEvent>(bytes))))
        }
        got => Err(EbpfError::EventSize {
            expected: mem::size_of::<TlsCaptureEvent>(),
            got,
        }),
    }
}

/// Copy `bytes` into a new instance of `T` via a raw pointer copy.
///
/// # Safety
///
/// `T` must be `#[repr(C)]` and `Copy`.  The caller must guarantee that
/// `bytes.len() == size_of::<T>()` (enforced by [`parse_event`]).
fn bytes_to<T: Copy>(bytes: &[u8]) -> T {
    assert_eq!(bytes.len(), mem::size_of::<T>());
    // SAFETY: T is #[repr(C)] and Copy; size equality is checked above.
    let mut value = mem::MaybeUninit::<T>::uninit();
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            value.as_mut_ptr().cast::<u8>(),
            bytes.len(),
        );
        value.assume_init()
    }
}
