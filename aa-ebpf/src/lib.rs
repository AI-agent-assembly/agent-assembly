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
//! # Platform support
//!
//! eBPF is Linux-only. On macOS, this crate compiles but the `aya` and
//! `aya-log` dependencies are gated behind `cfg(target_os = "linux")`.
//! A future degraded mode using DTrace may provide partial observability
//! on macOS.

pub mod events;
pub mod lineage;
pub mod loader;
