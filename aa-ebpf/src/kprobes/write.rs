//! Kprobe handler for `sys_write`.
//!
//! Intercepts file write operations to detect unauthorized modifications
//! to configuration files or audit logs.
