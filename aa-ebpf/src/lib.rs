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
//! ## Platform support
//!
//! eBPF is Linux-only. On macOS, this crate compiles but all aya-dependent
//! modules (`loader`, `uprobe`, `kprobe`, `tracepoint`, `ringbuf`) are gated
//! with `#[cfg(target_os = "linux")]`.  The `events` and `lineage` modules
//! are unconditional and available on all platforms.

// Shared event type re-exports — unconditional (no aya dependency).
pub mod events;
pub mod lineage;

// aya-dependent modules — Linux only.
#[cfg(target_os = "linux")]
pub mod error;
#[cfg(target_os = "linux")]
pub mod kprobe;
#[cfg(target_os = "linux")]
pub mod loader;
#[cfg(target_os = "linux")]
pub mod ringbuf;
#[cfg(target_os = "linux")]
pub mod tracepoint;
#[cfg(target_os = "linux")]
pub mod uprobe;

#[cfg(target_os = "linux")]
pub use error::EbpfError;
#[cfg(target_os = "linux")]
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
