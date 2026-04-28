//! Tracepoint management for process exec monitoring (AAASM-39).
//!
//! Attaches the `sched/sched_process_exec` and `sched/sched_process_exit`
//! tracepoints from the `aa-exec-probes` BPF binary.

#[cfg(target_os = "linux")]
use aya::Ebpf;

use crate::error::EbpfError;

/// Attaches and manages the `sched_process_exec` and `sched_process_exit`
/// tracepoint programs.
///
/// Create via [`TracepointManager::attach`]. The tracepoints stay active
/// until the `TracepointManager` is dropped.
pub struct TracepointManager {
    _private: (),
}

impl TracepointManager {
    /// Attach both `sched/sched_process_exec` and `sched/sched_process_exit`
    /// tracepoint programs.
    ///
    /// These tracepoints fire for every `execve`/`execveat` and process exit
    /// on the system. The BPF-side PID filter (`EXEC_PID_FILTER`) limits
    /// which events are emitted to the ring buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::ProbeAttach`] if the tracepoint category or name
    /// is not available on the running kernel.
    ///
    /// # Arguments
    ///
    /// * `bpf` — live [`Ebpf`] handle from loading [`crate::AA_EXEC_BPF`].
    #[cfg(target_os = "linux")]
    pub fn attach(bpf: &mut Ebpf) -> Result<Self, EbpfError> {
        use aya::programs::TracePoint;

        let tracepoints: &[(&str, &str, &str)] = &[
            ("handle_sched_process_exec", "sched", "sched_process_exec"),
            ("handle_sched_process_exit", "sched", "sched_process_exit"),
        ];

        for (prog_name, category, tp_name) in tracepoints {
            let program: &mut TracePoint = bpf
                .program_mut(prog_name)
                .ok_or_else(|| EbpfError::ProbeAttach(format!("{prog_name} program not found in BPF object")))?
                .try_into()
                .map_err(|e: aya::programs::ProgramError| EbpfError::ProbeAttach(e.to_string()))?;

            program
                .load()
                .map_err(|e| EbpfError::ProbeAttach(format!("{prog_name} load failed: {e}")))?;
            program.attach(category, tp_name).map_err(|e| {
                EbpfError::ProbeAttach(format!("{prog_name} attach to {category}/{tp_name} failed: {e}"))
            })?;

            tracing::info!(program = prog_name, tracepoint = %format!("{category}/{tp_name}"), "tracepoint attached");
        }

        Ok(Self { _private: () })
    }

    /// Attach tracepoints — non-Linux stub.
    ///
    /// Returns an error immediately since eBPF is not supported on this platform.
    #[cfg(not(target_os = "linux"))]
    pub fn attach_stub() -> Result<Self, EbpfError> {
        Err(EbpfError::ProgramLoad(
            "eBPF tracepoints are only supported on Linux".into(),
        ))
    }
}
