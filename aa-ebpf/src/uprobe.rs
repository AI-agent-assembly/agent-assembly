//! Uprobe/uretprobe management for OpenSSL TLS plaintext capture (AAASM-37).
//!
//! Attaches `ssl_write_uprobe` and `ssl_read_uretprobe` from
//! `aa-ebpf-programs` to the `SSL_write` and `SSL_read` symbols in every
//! matching OpenSSL shared library loaded by the target process.

use aya::Bpf;

use crate::error::EbpfError;

/// Attaches and manages OpenSSL uprobe/uretprobe programs.
///
/// Create via [`UprobeManager::attach`]. The probes stay active until the
/// `UprobeManager` is dropped.
#[allow(dead_code)]
pub struct UprobeManager {
    /// Target PID to monitor. `None` means monitor all processes.
    target_pid: Option<i32>,
}

impl UprobeManager {
    /// Attach `SSL_write` uprobe and `SSL_read` uretprobe to the target PID.
    ///
    /// Supports both OpenSSL 1.1.x (`SSL_write` symbol) and 3.x
    /// (`SSL_write_ex` symbol) — both are attached when present.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::Attach`] if the symbol cannot be resolved in any
    /// loaded OpenSSL library for the given PID.
    ///
    /// # Arguments
    ///
    /// * `bpf` — live [`Bpf`] handle from [`crate::loader::EbpfLoader::load`].
    /// * `target_pid` — PID to attach to, or `None` for system-wide.
    pub fn attach(bpf: &mut Bpf, target_pid: Option<i32>) -> Result<Self, EbpfError> {
        // TODO(AAASM-37): resolve OpenSSL library path for target_pid,
        // attach ssl_write_uprobe and ssl_read_uretprobe.
        let _ = (bpf, target_pid);
        todo!("attach SSL_write uprobe and SSL_read uretprobe")
    }
}
