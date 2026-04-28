// Integration test: load and attach the aa-file-io BPF program into a live kernel.
//
// Gated on:
//   - target_os = "linux"  — eBPF is Linux-only
//   - feature = "integration-test" — opted-in explicitly; requires root (CAP_BPF)
//
// Run locally (Linux, as root or via sudo):
//   sudo env "PATH=$PATH" cargo test -p aa-ebpf --features integration-test --test load_hello -- --nocapture
#![cfg(all(target_os = "linux", feature = "integration-test"))]

use aa_ebpf::AA_FILE_IO_BPF;
use aya::{programs::KProbe, Ebpf};

/// Verify the full "compile → embed → load → attach" pipeline end-to-end.
///
/// 1. `Ebpf::load` parses and JIT-compiles the embedded BPF bytecode.
/// 2. `KProbe::load` submits the program to the kernel verifier.
/// 3. `KProbe::attach` hooks the probe onto `__x64_sys_openat`.
///
/// The link guard returned by `attach` detaches the probe on drop, so the
/// kernel is left clean after the test regardless of pass/fail.
#[test]
fn aa_file_io_loads_and_attaches() {
    let mut bpf =
        Ebpf::load(AA_FILE_IO_BPF).expect("failed to load aa-file-io BPF program — ensure the test is running as root");

    let program: &mut KProbe = bpf
        .program_mut("aa_sys_openat")
        .expect("aa_sys_openat program not found in BPF object")
        .try_into()
        .expect("aa_sys_openat is not a KProbe program");

    program.load().expect("kernel verifier rejected aa_sys_openat kprobe");

    let _link = program
        .attach("__x64_sys_openat", 0)
        .expect("failed to attach aa_sys_openat kprobe to __x64_sys_openat");

    // Reaching this line confirms the full pipeline works.
    // _link is dropped here, which detaches the probe from the kernel.
}
