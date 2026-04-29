//! Kprobe management for file I/O interception (AAASM-38).
//!
//! Attaches `openat_kprobe`, `write_kprobe`, and `unlink_kprobe` from
//! `aa-ebpf-programs` to the corresponding kernel functions, filtered by
//! the target PID stored in a BPF map.

#[cfg(target_os = "linux")]
use aya::Ebpf;

use crate::error::EbpfError;

/// Attaches and manages file I/O kprobe programs.
///
/// Create via [`KprobeManager::attach`]. The probes stay active until the
/// `KprobeManager` is dropped.
pub struct KprobeManager {
    /// Target PID to filter inside the eBPF program.
    target_pid: Option<i32>,
    /// Live kprobe link handles. Dropping them detaches the probes from the
    /// kernel. Stored as type-erased `Box<dyn Any>` to avoid coupling to
    /// aya's internal link-id type (matches `UprobeManager` convention).
    #[cfg(target_os = "linux")]
    _links: Vec<Box<dyn std::any::Any>>,
}

impl KprobeManager {
    /// Attach file I/O kprobes (`openat`, `write`, `unlink`) for the target PID.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::Attach`] if a kernel symbol cannot be found
    /// (e.g., the running kernel uses a different internal function name).
    ///
    /// # Arguments
    ///
    /// * `bpf` — live [`Ebpf`] handle from [`crate::loader::EbpfLoader::load`].
    /// * `target_pid` — PID to filter, or `None` for system-wide monitoring.
    #[cfg(target_os = "linux")]
    pub fn attach(bpf: &mut Ebpf, target_pid: Option<i32>) -> Result<Self, EbpfError> {
        // Write target PID into the BPF-side filter map so the kernel-space
        // probes only emit events for the monitored process.
        if let Some(pid) = target_pid {
            let mut pid_filter: aya::maps::HashMap<_, u32, u8> = aya::maps::HashMap::try_from(
                bpf.map_mut("PID_FILTER")
                    .ok_or_else(|| EbpfError::ProbeAttach("PID_FILTER map not found".into()))?,
            )
            .map_err(|e| EbpfError::ProbeAttach(e.to_string()))?;

            pid_filter
                .insert(pid as u32, 1, 0)
                .map_err(|e| EbpfError::ProbeAttach(e.to_string()))?;
        }

        // Attach all file I/O kprobe programs to their kernel functions.
        let probes: &[(&str, &str)] = &[
            ("aa_sys_openat", "__x64_sys_openat"),
            ("aa_sys_openat_ret", "__x64_sys_openat"),
            ("aa_sys_read", "__x64_sys_read"),
            ("aa_sys_write", "__x64_sys_write"),
            ("aa_sys_unlink", "__x64_sys_unlinkat"),
            ("aa_sys_rename", "__x64_sys_renameat2"),
        ];

        let mut links: Vec<Box<dyn std::any::Any>> = Vec::with_capacity(probes.len());

        for (prog_name, fn_name) in probes {
            let program: &mut aya::programs::KProbe = bpf
                .program_mut(prog_name)
                .ok_or_else(|| EbpfError::ProbeAttach(format!("{prog_name} program not found")))?
                .try_into()
                .map_err(|e: aya::programs::ProgramError| EbpfError::ProbeAttach(e.to_string()))?;

            program
                .load()
                .map_err(|e| EbpfError::ProbeAttach(format!("{prog_name} load failed: {e}")))?;

            let link = program
                .attach(fn_name, 0)
                .map_err(|e| EbpfError::ProbeAttach(format!("{prog_name} attach to {fn_name} failed: {e}")))?;

            links.push(Box::new(link));
            tracing::info!(program = prog_name, function = fn_name, "kprobe attached");
        }

        Ok(Self {
            target_pid,
            _links: links,
        })
    }

    /// Attach kprobes — non-Linux stub.
    ///
    /// Returns an error immediately since eBPF is not supported on this platform.
    #[cfg(not(target_os = "linux"))]
    pub fn attach(_bpf: &mut (), _target_pid: Option<i32>) -> Result<Self, EbpfError> {
        Err(EbpfError::ProbeAttach(
            "kprobe attachment requires Linux".into(),
        ))
    }
}
