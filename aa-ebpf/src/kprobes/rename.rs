//! Kprobe handler for `sys_rename`.
//!
//! Intercepts file rename/move operations to detect attempts to relocate
//! sensitive files or disguise unauthorized file modifications.
