//! Userspace eBPF program loader and lifecycle manager.

use crate::error::EbpfError;
use crate::maps::PathPattern;

/// Manages the lifecycle of eBPF programs: loading bytecode, attaching
/// kprobes, and updating BPF maps at runtime.
///
/// The loader is the primary entry point for userspace interaction with
/// the eBPF subsystem. It is only functional on Linux; on other platforms
/// it returns [`EbpfError::ProgramLoad`] immediately.
pub struct EbpfLoader {
    /// Target PID to monitor (and its descendants).
    target_pid: u32,
}

impl EbpfLoader {
    /// Create a new loader targeting the given PID and its descendants.
    pub fn new(target_pid: u32) -> Self {
        Self { target_pid }
    }

    /// Load the compiled eBPF bytecode into the kernel.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::ProgramLoad`] if the bytecode cannot be loaded
    /// (e.g., missing privileges, unsupported kernel, or non-Linux platform).
    pub fn load(&mut self) -> Result<(), EbpfError> {
        #[cfg(not(target_os = "linux"))]
        {
            return Err(EbpfError::ProgramLoad("eBPF is only supported on Linux".into()));
        }

        #[cfg(target_os = "linux")]
        {
            tracing::info!(pid = self.target_pid, "loading eBPF programs");
            // TODO: load bytecode via aya::Ebpf::load() (AAASM-132)
            Ok(())
        }
    }

    /// Attach all file I/O kprobes to the running kernel.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::ProbeAttach`] if any kprobe fails to attach.
    pub fn attach_kprobes(&mut self) -> Result<(), EbpfError> {
        #[cfg(not(target_os = "linux"))]
        {
            return Err(EbpfError::ProbeAttach("eBPF is only supported on Linux".into()));
        }

        #[cfg(target_os = "linux")]
        {
            tracing::info!("attaching file I/O kprobes");
            // TODO: attach kprobes for openat, read, write, unlink, rename
            Ok(())
        }
    }

    /// Update the path filter BPF map with new patterns.
    ///
    /// This can be called at runtime without reloading the eBPF programs.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::MapUpdate`] if the map update fails.
    pub fn update_path_filter(&self, _patterns: &[PathPattern]) -> Result<(), EbpfError> {
        #[cfg(not(target_os = "linux"))]
        {
            return Err(EbpfError::MapUpdate("eBPF is only supported on Linux".into()));
        }

        #[cfg(target_os = "linux")]
        {
            tracing::info!(count = _patterns.len(), "updating path filter map");
            // TODO: write patterns to BPF hash map
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::PathVerdict;

    #[test]
    fn new_stores_target_pid() {
        let loader = EbpfLoader::new(1234);
        assert_eq!(loader.target_pid, 1234);
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn load_returns_error_on_non_linux() {
        let mut loader = EbpfLoader::new(1);
        let err = loader.load().unwrap_err();
        assert!(matches!(err, EbpfError::ProgramLoad(_)));
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn attach_kprobes_returns_error_on_non_linux() {
        let mut loader = EbpfLoader::new(1);
        let err = loader.attach_kprobes().unwrap_err();
        assert!(matches!(err, EbpfError::ProbeAttach(_)));
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn update_path_filter_returns_error_on_non_linux() {
        let loader = EbpfLoader::new(1);
        let patterns = vec![PathPattern {
            pattern: "/etc/shadow".into(),
            verdict: PathVerdict::Deny,
        }];
        let err = loader.update_path_filter(&patterns).unwrap_err();
        assert!(matches!(err, EbpfError::MapUpdate(_)));
    }
}
