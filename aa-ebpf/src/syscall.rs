//! Syscall kind classification for file I/O kprobes.

use core::fmt;

/// Identifies which file-related syscall was intercepted by a kprobe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SyscallKind {
    /// `sys_openat` — open or create a file.
    Openat = 0,
    /// `sys_read` — read from a file descriptor.
    Read = 1,
    /// `sys_write` — write to a file descriptor.
    Write = 2,
    /// `sys_unlink` — delete a file.
    Unlink = 3,
    /// `sys_rename` — rename or move a file.
    Rename = 4,
}

impl fmt::Display for SyscallKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Openat => write!(f, "openat"),
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
            Self::Unlink => write!(f, "unlink"),
            Self::Rename => write!(f, "rename"),
        }
    }
}
