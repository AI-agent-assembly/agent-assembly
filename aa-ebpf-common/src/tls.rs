//! Shared types for TLS plaintext capture events (AAASM-37).
//!
//! Emitted by the `SSL_write` uprobe and `SSL_read` uretprobe in
//! `aa-ebpf-programs` and consumed by the userspace ring-buffer reader in
//! `aa-ebpf`.

/// Maximum payload bytes captured per event.
///
/// Larger payloads are split across multiple events identified by the same
/// `seq` counter.
pub const MAX_PAYLOAD_LEN: usize = 4096;

/// A single TLS plaintext capture event emitted from kernel-space.
///
/// The struct is `#[repr(C)]` so that the same memory layout is shared between
/// the eBPF program (compiled for the bpf target) and the userspace consumer
/// (compiled for the host target).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TlsCaptureEvent {
    /// Monotonic kernel timestamp (nanoseconds).
    pub timestamp_ns: u64,
    /// Process ID of the monitored agent.
    pub pid: u32,
    /// Thread ID within the process.
    pub tid: u32,
    /// Total plaintext length before splitting (bytes).
    pub data_len: u32,
    /// Sequence number for reassembling split payloads (0-indexed).
    pub seq: u32,
    /// Direction: `0` = write (outbound), `1` = read (inbound).
    pub direction: u8,
    /// Padding for alignment.
    pub _pad: [u8; 7],
    /// Raw plaintext bytes (up to [`MAX_PAYLOAD_LEN`]).
    pub payload: [u8; MAX_PAYLOAD_LEN],
}
