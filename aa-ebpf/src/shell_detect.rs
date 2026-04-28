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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_critical_bash() {
        let d = ShellDetector::new();
        assert_eq!(d.check("/bin/bash"), Some(AlertLevel::Critical));
    }

    #[test]
    fn detects_critical_sh() {
        let d = ShellDetector::new();
        assert_eq!(d.check("/bin/sh"), Some(AlertLevel::Critical));
    }

    #[test]
    fn detects_critical_curl() {
        let d = ShellDetector::new();
        assert_eq!(d.check("/usr/bin/curl"), Some(AlertLevel::Critical));
    }

    #[test]
    fn detects_critical_wget() {
        let d = ShellDetector::new();
        assert_eq!(d.check("/usr/bin/wget"), Some(AlertLevel::Critical));
    }

    #[test]
    fn detects_warning_python() {
        let d = ShellDetector::new();
        assert_eq!(d.check("/usr/bin/python3"), Some(AlertLevel::Warning));
    }

    #[test]
    fn detects_warning_node() {
        let d = ShellDetector::new();
        assert_eq!(d.check("/usr/bin/node"), Some(AlertLevel::Warning));
    }

    #[test]
    fn allows_safe_binary() {
        let d = ShellDetector::new();
        assert_eq!(d.check("/usr/bin/ls"), None);
    }

    #[test]
    fn allows_agent_binary() {
        let d = ShellDetector::new();
        assert_eq!(d.check("/opt/agent/run"), None);
    }

    #[test]
    fn basename_extraction_no_slash() {
        let d = ShellDetector::new();
        assert_eq!(d.check("bash"), Some(AlertLevel::Critical));
    }

    #[test]
    fn build_alert_returns_some_for_match() {
        let d = ShellDetector::new();
        let alert = d.build_alert(100, 200, "/bin/bash", 5000).unwrap();

        assert_eq!(alert.parent_pid, 100);
        assert_eq!(alert.child_pid, 200);
        assert_eq!(alert.alert_level, AlertLevel::Critical);
        assert_eq!(alert.timestamp_ns, 5000);
        // Check executable contains the path.
        let nul = alert.executable.iter().position(|&b| b == 0).unwrap_or(alert.executable.len());
        let exe_str = core::str::from_utf8(&alert.executable[..nul]).unwrap();
        assert_eq!(exe_str, "/bin/bash");
    }

    #[test]
    fn build_alert_returns_none_for_safe() {
        let d = ShellDetector::new();
        assert!(d.build_alert(100, 200, "/usr/bin/ls", 5000).is_none());
    }
}
