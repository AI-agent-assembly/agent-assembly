// Integration test: load and attach the aa-exec-probes BPF program into a live kernel.
//
// Gated on:
//   - target_os = "linux"  — eBPF is Linux-only
//   - feature = "integration-test" — opted-in explicitly; requires root (CAP_BPF)
//
// Run locally (Linux, as root or via sudo):
//   sudo env "PATH=$PATH" cargo test -p aa-ebpf --features integration-test --test exec_tracepoint -- --nocapture
#![cfg(all(target_os = "linux", feature = "integration-test"))]

use aa_ebpf::{tracepoint::TracepointManager, AA_EXEC_BPF};
use aya::Ebpf;

/// Verify the full "compile → embed → load → attach" pipeline for exec tracepoints
/// using the `TracepointManager` API.
///
/// 1. `Ebpf::load` parses and JIT-compiles the embedded BPF bytecode.
/// 2. `TracepointManager::attach` loads and attaches both tracepoints.
/// 3. Dropping the `TracepointManager` detaches the probes from the kernel.
#[test]
fn aa_exec_probes_loads_and_attaches() {
    let mut bpf = Ebpf::load(AA_EXEC_BPF)
        .expect("failed to load aa-exec-probes BPF program — ensure the test is running as root");

    let _manager = TracepointManager::attach(&mut bpf).expect("TracepointManager::attach failed");

    // Reaching this line confirms the full pipeline works.
    // TracepointManager is dropped here, which detaches the probes from the kernel.
}

/// Verify that `detach()` can be called explicitly without panic.
#[test]
fn tracepoint_manager_explicit_detach() {
    let mut bpf = Ebpf::load(AA_EXEC_BPF)
        .expect("failed to load aa-exec-probes BPF program — ensure the test is running as root");

    let mut manager = TracepointManager::attach(&mut bpf).expect("TracepointManager::attach failed");

    // Explicit detach should succeed.
    manager.detach();

    // Calling detach again should be a no-op (not panic).
    manager.detach();
}
