//! Kprobe handler for `sys_openat`.
//!
//! Intercepts file open operations to detect access to sensitive paths
//! (e.g., `/etc/shadow`, `~/.ssh/`) and capture the open flags
//! (`O_RDONLY`, `O_WRONLY`, `O_RDWR`, `O_CREAT`).
