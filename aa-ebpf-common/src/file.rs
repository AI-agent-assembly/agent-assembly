//! Shared types for file I/O kprobe events (AAASM-38).
//!
//! Emitted by `openat`, `write`, and `unlink` kprobes in `aa-ebpf-programs`
//! and consumed by the userspace ring-buffer reader in `aa-ebpf`.

/// Maximum file path bytes captured per event.
pub const MAX_PATH_LEN: usize = 256;

/// File operation kind intercepted at the kernel level.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileOp {
    /// `openat` — file open attempt.
    Open = 0,
    /// `write` — data written to a file descriptor.
    Write = 1,
    /// `unlink` / `unlinkat` — file deletion attempt.
    Unlink = 2,
}

/// A single file I/O kprobe event emitted from kernel-space.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FileEvent {
    /// Monotonic kernel timestamp (nanoseconds).
    pub timestamp_ns: u64,
    /// Process ID of the monitored agent.
    pub pid: u32,
    /// Thread ID within the process.
    pub tid: u32,
    /// File descriptor involved (−1 if not applicable).
    pub fd: i32,
    /// Operation that triggered this event.
    pub op: FileOp,
    /// Padding for alignment.
    pub _pad: [u8; 3],
    /// Null-terminated file path (up to [`MAX_PATH_LEN`] bytes).
    pub path: [u8; MAX_PATH_LEN],
}
