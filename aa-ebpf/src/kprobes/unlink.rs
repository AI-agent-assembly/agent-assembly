//! Kprobe handler for `sys_unlink`.
//!
//! Intercepts file deletion operations to detect evidence destruction
//! (e.g., deleting audit logs) or unauthorized file removal.
