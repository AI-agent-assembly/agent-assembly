//! eBPF-based kernel-level monitoring hooks for Agent Assembly.
//!
//! This crate provides kernel-space probes that intercept system calls and
//! network events originating from AI agent processes, feeding data into
//! the governance pipeline without requiring code changes in agents.
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
