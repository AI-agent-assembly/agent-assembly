//! eBPF-based kernel-level monitoring hooks for Agent Assembly — Layer 3.
//!
//! This crate is the **userspace** half of the aa-ebpf subsystem.  It loads
//! the compiled eBPF programs (from `aa-ebpf-probes`), attaches the probes
//! to the kernel, and reads structured events from the shared BPF ring buffer.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │  aa-ebpf (userspace)                         │
//! │                                              │
//! │  EbpfLoader ──► UprobeManager  (AAASM-37)   │
//! │             ──► KprobeManager  (AAASM-38)   │
//! │             ──► TracepointManager (AAASM-39) │
//! │                                              │
//! │  RingBufReader ◄── BPF ring buffer           │
//! └─────────────────────────────────────────────┘
//!          │ kernel boundary │
//! ┌─────────────────────────────────────────────┐
//! │  aa-ebpf-probes (bpfel-unknown-none)         │
//! │                                              │
//! │  ssl_write_uprobe / ssl_read_uretprobe       │
//! │  openat_kprobe / write_kprobe / unlink_kprobe│
//! │  sched_process_exec (tracepoint)             │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! ## Shared types
//!
//! Event structs shared between kernel-space and userspace live in
//! [`aa_ebpf_common`].  They are `#[repr(C)]` and `no_std` so they compile
//! for both targets without modification.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use aa_ebpf::{loader::EbpfLoader, ringbuf::RingBufReader};
//! use aa_ebpf::{uprobe::UprobeManager, kprobe::KprobeManager};
//! use aa_ebpf::tracepoint::TracepointManager;
//!
//! let mut bpf = EbpfLoader::load()?;
//! let _uprobes  = UprobeManager::attach(&mut bpf, Some(target_pid))?;
//! let _kprobes  = KprobeManager::attach(&mut bpf, Some(target_pid))?;
//! let _tp       = TracepointManager::attach(&mut bpf)?;
//! let mut reader = RingBufReader::new(bpf)?;
//!
//! while let Some(event) = reader.next().await? {
//!     // forward event to aa-runtime governance pipeline
//! }
//! ```

pub mod error;
pub mod events;
pub mod kprobe;
pub mod lineage;
pub mod loader;
pub mod ringbuf;
pub mod tracepoint;
pub mod uprobe;

pub use error::EbpfError;
pub use ringbuf::EbpfEvent;

/// Compiled BPF bytecode for the `aa-hello` probe program.
///
/// Embedded from `aa-ebpf-probes/src/main.rs` at build time via `aya-build`.
/// Pass this slice to [`aya::Ebpf::load`] to obtain a handle to all programs
/// in the probe crate.
///
/// Only meaningful on Linux — on other platforms this constant is absent.
#[cfg(target_os = "linux")]
pub static AA_HELLO_BPF: &[u8] = aya::include_bytes_aligned!(concat!(
    env!("OUT_DIR"),
    // Path layout: OUT_DIR/<package-name>/<target>/release/<binary-name>
    // Package name is "aa-ebpf-probes" (from Cargo.toml [package].name).
    // Binary name is "aa-hello" (from Cargo.toml [[bin]].name).
    "/aa-ebpf-probes/bpfel-unknown-none/release/aa-hello"
));
