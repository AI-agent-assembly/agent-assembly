//! BPF map types and constants for path filtering.

/// Maximum number of path patterns in the BPF hash map.
pub const MAX_PATH_PATTERNS: u32 = 256;

/// Maximum byte length of a single path pattern stored in a BPF map entry.
pub const MAX_PATH_LEN: usize = 256;

/// Whether matching a path pattern should allow or deny the operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PathVerdict {
    /// The path is allowed — no policy violation.
    Allow = 0,
    /// The path is blocked — triggers a policy violation event.
    Deny = 1,
}

/// A path pattern entry stored in a BPF hash map.
///
/// Userspace writes these entries to configure which file paths the kprobes
/// should flag. The map is updatable at runtime without reloading the eBPF
/// programs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPattern {
    /// The path prefix or exact path to match (e.g., `/etc/shadow`).
    pub pattern: String,
    /// Whether matching this pattern should allow or deny the operation.
    pub verdict: PathVerdict,
}
