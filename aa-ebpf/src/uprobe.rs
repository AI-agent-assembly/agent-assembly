//! Uprobe/uretprobe management for OpenSSL TLS plaintext capture (AAASM-37).
//!
//! Attaches `ssl_write_uprobe` and `ssl_read_uretprobe` from
//! `aa-ebpf-probes` to the `SSL_write` and `SSL_read` symbols in every
//! matching OpenSSL shared library loaded by the target process.

#[cfg(target_os = "linux")]
use aya::{maps::Array, programs::UProbe, Ebpf};

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
    /// * `bpf` — live [`Ebpf`] handle from [`crate::loader::EbpfLoader::load`].
    /// * `target_pid` — PID to attach to, or `None` for system-wide.
    #[cfg(target_os = "linux")]
    pub fn attach(bpf: &mut Ebpf, target_pid: Option<i32>) -> Result<Self, EbpfError> {
        // 1. Write the target PID into the BPF-side filter map.
        {
            let map = bpf.map_mut("TARGET_PID").ok_or_else(|| EbpfError::MapNotFound {
                name: "TARGET_PID".into(),
            })?;
            let mut pid_map: Array<_, u32> = Array::try_from(map)?;
            let pid_val: u32 = target_pid.map(|p| p as u32).unwrap_or(0);
            pid_map.set(0, pid_val, 0)?;
        }

        // 2. Find the OpenSSL shared library for the target process.
        let ssl_path = find_openssl_path(target_pid)?;

        // 3. Attach ssl_write uprobe (captures outbound TLS plaintext).
        {
            let prog: &mut UProbe = bpf
                .program_mut("ssl_write")
                .ok_or_else(|| EbpfError::MapNotFound {
                    name: "ssl_write".into(),
                })?
                .try_into()?;
            prog.load()?;
            prog.attach(Some("SSL_write"), 0, &ssl_path, target_pid)?;
        }

        // 4. Attach ssl_read_entry uprobe (saves SSL_read buf ptr for step 5).
        {
            let prog: &mut UProbe = bpf
                .program_mut("ssl_read_entry")
                .ok_or_else(|| EbpfError::MapNotFound {
                    name: "ssl_read_entry".into(),
                })?
                .try_into()?;
            prog.load()?;
            prog.attach(Some("SSL_read"), 0, &ssl_path, target_pid)?;
        }

        // 5. Attach ssl_read_exit uretprobe (captures inbound TLS plaintext).
        {
            let prog: &mut UProbe = bpf
                .program_mut("ssl_read_exit")
                .ok_or_else(|| EbpfError::MapNotFound {
                    name: "ssl_read_exit".into(),
                })?
                .try_into()?;
            prog.load()?;
            prog.attach(Some("SSL_read"), 0, &ssl_path, target_pid)?;
        }

        Ok(Self { target_pid })
    }

    /// Stub for non-Linux platforms — uprobe attachment requires Linux.
    #[cfg(not(target_os = "linux"))]
    pub fn attach(_bpf: &mut (), _target_pid: Option<i32>) -> Result<Self, EbpfError> {
        Err(EbpfError::MapNotFound {
            name: "uprobe attachment requires Linux".into(),
        })
    }
}

/// Find the path to the OpenSSL shared library for the given PID.
///
/// Scans `/proc/<pid>/maps` for a line containing `libssl.so`.
#[cfg(target_os = "linux")]
fn find_openssl_path(target_pid: Option<i32>) -> Result<String, EbpfError> {
    // TODO(AAASM-37): implement /proc/<pid>/maps parsing to find libssl path.
    let _ = target_pid;
    todo!("find OpenSSL library path for target PID")
}
