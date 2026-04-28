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
    /// Loaded BPF object handle (Linux only).
    #[cfg(target_os = "linux")]
    bpf: Option<aya::Ebpf>,
}

impl EbpfLoader {
    /// Create a new loader targeting the given PID and its descendants.
    pub fn new(target_pid: u32) -> Self {
        Self {
            target_pid,
            #[cfg(target_os = "linux")]
            bpf: None,
        }
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
            let mut bpf = aya::Ebpf::load(crate::AA_FILE_IO_BPF)
                .map_err(|e| EbpfError::ProgramLoad(e.to_string()))?;

            // Insert the target PID into the PID filter map.
            let mut pid_filter: aya::maps::HashMap<_, u32, u8> = aya::maps::HashMap::try_from(
                bpf.map_mut("PID_FILTER")
                    .ok_or_else(|| EbpfError::ProgramLoad("PID_FILTER map not found".into()))?,
            )
            .map_err(|e| EbpfError::ProgramLoad(e.to_string()))?;

            pid_filter
                .insert(self.target_pid, 1, 0)
                .map_err(|e| EbpfError::ProgramLoad(e.to_string()))?;

            self.bpf = Some(bpf);
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
            use aya::programs::KProbe;

            let bpf = self
                .bpf
                .as_mut()
                .ok_or_else(|| EbpfError::ProbeAttach("BPF not loaded — call load() first".into()))?;

            let probes: &[(&str, &str)] = &[
                ("aa_sys_openat", "__x64_sys_openat"),
                ("aa_sys_openat_ret", "__x64_sys_openat"),
                ("aa_sys_read", "__x64_sys_read"),
                ("aa_sys_write", "__x64_sys_write"),
                ("aa_sys_unlink", "__x64_sys_unlinkat"),
                ("aa_sys_rename", "__x64_sys_renameat2"),
            ];

            for (prog_name, fn_name) in probes {
                let program: &mut KProbe = bpf
                    .program_mut(prog_name)
                    .ok_or_else(|| {
                        EbpfError::ProbeAttach(format!("{prog_name} program not found"))
                    })?
                    .try_into()
                    .map_err(|e: aya::programs::ProgramError| {
                        EbpfError::ProbeAttach(e.to_string())
                    })?;

                program
                    .load()
                    .map_err(|e| EbpfError::ProbeAttach(e.to_string()))?;
                program
                    .attach(fn_name, 0)
                    .map_err(|e| EbpfError::ProbeAttach(e.to_string()))?;

                tracing::info!(program = prog_name, function = fn_name, "kprobe attached");
            }

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
        use crate::maps::PathVerdict;

        let loader = EbpfLoader::new(1);
        let patterns = vec![PathPattern {
            pattern: "/etc/shadow".into(),
            verdict: PathVerdict::Deny,
        }];
        let err = loader.update_path_filter(&patterns).unwrap_err();
        assert!(matches!(err, EbpfError::MapUpdate(_)));
    }
}
