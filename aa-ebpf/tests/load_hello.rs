// Integration test: load and attach the aa-hello BPF program into a live kernel.
//
// Gated on:
//   - target_os = "linux"  — eBPF is Linux-only
//   - feature = "integration-test" — opted-in explicitly; requires root (CAP_BPF)
//
// Run locally (Linux, as root or via sudo):
//   sudo -E cargo test -p aa-ebpf --features integration-test --test load_hello -- --nocapture
#![cfg(all(target_os = "linux", feature = "integration-test"))]

use aa_ebpf::AA_HELLO_BPF;
use aya::{programs::KProbe, Ebpf};

/// Verify the full "compile → embed → load → attach" pipeline end-to-end.
///
/// 1. `Ebpf::load` parses and JIT-compiles the embedded BPF bytecode.
/// 2. `KProbe::load` submits the program to the kernel verifier.
/// 3. `KProbe::attach` hooks the probe onto `__x64_sys_write`.
///
/// The link guard returned by `attach` detaches the probe on drop, so the
/// kernel is left clean after the test regardless of pass/fail.
#[test]
fn aa_hello_loads_and_attaches() {
    let mut bpf = Ebpf::load(AA_HELLO_BPF)
        .expect("failed to load aa-hello BPF program — ensure the test is running as root");

    let program: &mut KProbe = bpf
        .program_mut("aa_hello")
        .expect("aa_hello program not found in BPF object")
        .try_into()
        .expect("aa_hello is not a KProbe program");

    program.load().expect("kernel verifier rejected aa_hello kprobe");

    let _link = program
        .attach("__x64_sys_write", 0)
        .expect("failed to attach aa_hello kprobe to __x64_sys_write");

    // Reaching this line confirms the full pipeline works.
    // _link is dropped here, which detaches the probe from the kernel.
}
