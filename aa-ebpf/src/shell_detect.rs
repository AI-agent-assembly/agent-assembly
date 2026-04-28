//! Shell injection detection for agent process families (AAASM-39).
//!
//! Inspects process exec events against a set of known shell and download
//! utility patterns. When a monitored agent (or one of its descendants)
//! spawns a suspicious process, the detector classifies it with an
//! [`AlertLevel`] and produces a [`ShellInjectionAlert`].

use aa_ebpf_common::exec::{AlertLevel, ShellInjectionAlert, MAX_EXECUTABLE_LEN};

/// Detects shell injection patterns in process exec events.
///
/// Maintains a list of executable basenames grouped by severity.
/// Call [`ShellDetector::check`] with a filename to determine whether
/// it matches a known pattern.
pub struct ShellDetector {
    /// Critical-severity patterns (raw shells, download utilities).
    critical: Vec<&'static str>,
    /// Warning-severity patterns (scripting runtimes).
    warning: Vec<&'static str>,
}

impl ShellDetector {
    /// Create a detector with default shell and utility patterns.
    pub fn new() -> Self {
        Self {
            critical: vec!["sh", "bash", "dash", "zsh", "csh", "curl", "wget", "nc", "ncat"],
            warning: vec!["python", "python3", "node", "perl", "ruby", "php", "lua"],
        }
    }

    /// Check whether the given executable filename matches a known pattern.
    ///
    /// Extracts the basename from the path and compares it against the
    /// critical and warning pattern lists.
    ///
    /// Returns `None` if the filename is not suspicious.
    pub fn check(&self, filename: &str) -> Option<AlertLevel> {
        let basename = filename.rsplit('/').next().unwrap_or(filename);

        if self.critical.iter().any(|&pat| basename == pat) {
            return Some(AlertLevel::Critical);
        }
        if self.warning.iter().any(|&pat| basename == pat) {
            return Some(AlertLevel::Warning);
        }

        None
    }

    /// Build a [`ShellInjectionAlert`] from an exec event that matched.
    ///
    /// `parent_pid` is the monitored agent PID (or its nearest tracked
    /// ancestor). `child_pid` is the PID of the suspicious process.
    pub fn build_alert(
        &self,
        parent_pid: u32,
        child_pid: u32,
        filename: &str,
        timestamp_ns: u64,
    ) -> Option<ShellInjectionAlert> {
        let level = self.check(filename)?;

        let mut executable = [0u8; MAX_EXECUTABLE_LEN];
        let bytes = filename.as_bytes();
        let len = bytes.len().min(MAX_EXECUTABLE_LEN);
        executable[..len].copy_from_slice(&bytes[..len]);

        Some(ShellInjectionAlert {
            parent_pid,
            child_pid,
            executable,
            alert_level: level,
            timestamp_ns,
        })
    }
}

impl Default for ShellDetector {
    fn default() -> Self {
        Self::new()
    }
}
