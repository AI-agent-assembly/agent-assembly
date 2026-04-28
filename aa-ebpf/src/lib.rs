//! eBPF-based kernel-level monitoring hooks for Agent Assembly (Layer 3).
//!
//! This crate is the **userspace** half of the eBPF subsystem. It loads
//! compiled BPF bytecode into the kernel, attaches tracepoints and kprobes,
//! and reads events back through ring buffers — feeding them into the
//! `aa-core` governance pipeline.
//!
//! # Crate architecture (three-crate split)
//!
//! eBPF programs run inside the Linux kernel and must be compiled to the
//! `bpfel-unknown-none` target. This forces a three-crate design:
//!
//! | Crate | Role | Target |
//! |---|---|---|
//! | **`aa-ebpf-common`** | Shared `repr(C)` types for maps and events | `no_std` (both) |
//! | **`aa-ebpf-ebpf`** | BPF programs (tracepoints, kprobes, uprobes) | `bpfel-unknown-none` |
//! | **`aa-ebpf`** (this crate) | Userspace loader, event consumer, lineage tracker | `std` (Linux) |
//!
//! # Data flow
//!
//! ```text
//! kernel: tracepoint fires → BPF program writes to ring buffer / map
//!                                         │
//! ────────────────────────────────────────────────────────────
//!                                         │
//! user:  EbpfLoader reads ring buffer → ProcessSpawnEvent
//!          → ProcessLineageTracker updates PID tree
//!          → ShellInjectionAlert emitted if suspicious
//!          → event forwarded to aa-core governance pipeline
//! ```
//!
//! # Loading BPF programs
//!
//! The compiled BPF bytecode is embedded at build time and exposed as the
//! [`AA_HELLO_BPF`] constant. Pass it to [`aya::Ebpf::load`] to load all
//! programs defined in the `aa-ebpf-probes` crate:
//!
//! ```no_run
//! # #[cfg(target_os = "linux")]
//! # {
//! use aya::Ebpf;
//! use aa_ebpf::AA_HELLO_BPF;
//!
//! let mut bpf = Ebpf::load(AA_HELLO_BPF).expect("failed to load BPF programs");
//! # }
//! ```
//!
//! # Platform support
//!
//! eBPF is Linux-only. On macOS, this crate compiles but the `aya` and
//! `aya-log` dependencies are gated behind `cfg(target_os = "linux")`.
//! A future degraded mode using DTrace may provide partial observability
//! on macOS.

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

pub mod error;
pub mod events;
pub mod kprobes;
pub mod lineage;
pub mod loader;
pub mod maps;
pub mod syscall;

pub use error::EbpfError;
pub use events::FileIoEvent;
pub use maps::{PathPattern, PathVerdict, MAX_PATH_LEN, MAX_PATH_PATTERNS};
pub use syscall::SyscallKind;
