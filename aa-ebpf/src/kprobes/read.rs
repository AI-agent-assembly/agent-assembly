//! Kprobe handler for `sys_read`.
//!
//! Intercepts file read operations to detect data exfiltration from
//! sensitive files after they have been opened.
