//! Shared types for process exec tracepoint events (AAASM-39).
//!
//! Emitted by the `sched_process_exec` tracepoint in `aa-ebpf-programs`
//! and consumed by the userspace ring-buffer reader in `aa-ebpf`.

/// Maximum bytes captured for the executable path.
pub const MAX_FILENAME_LEN: usize = 256;

/// Maximum bytes captured for the command-line argument string.
pub const MAX_ARGS_LEN: usize = 512;

/// A single process-exec tracepoint event emitted from kernel-space.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExecEvent {
    /// Monotonic kernel timestamp (nanoseconds).
    pub timestamp_ns: u64,
    /// Process ID of the new process.
    pub pid: u32,
    /// Parent process ID.
    pub ppid: u32,
    /// User ID that spawned the process.
    pub uid: u32,
    /// Padding for alignment.
    pub _pad: u32,
    /// Null-terminated executable path (up to [`MAX_FILENAME_LEN`] bytes).
    pub filename: [u8; MAX_FILENAME_LEN],
    /// Space-separated argv string (up to [`MAX_ARGS_LEN`] bytes).
    pub args: [u8; MAX_ARGS_LEN],
}
