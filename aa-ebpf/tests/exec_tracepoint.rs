// Integration test: load and attach the aa-exec-probes BPF program into a live kernel.
//
// Gated on:
//   - target_os = "linux"  — eBPF is Linux-only
//   - feature = "integration-test" — opted-in explicitly; requires root (CAP_BPF)
//
// Run locally (Linux, as root or via sudo):
//   sudo env "PATH=$PATH" cargo test -p aa-ebpf --features integration-test --test exec_tracepoint -- --nocapture
#![cfg(all(target_os = "linux", feature = "integration-test"))]

use aa_ebpf::AA_EXEC_BPF;
use aya::{programs::TracePoint, Ebpf};

/// Verify the full "compile → embed → load → attach" pipeline for exec tracepoints.
///
/// 1. `Ebpf::load` parses and JIT-compiles the embedded BPF bytecode.
/// 2. `TracePoint::load` submits the program to the kernel verifier.
/// 3. `TracePoint::attach` hooks onto `sched/sched_process_exec`.
///
/// The link guard returned by `attach` detaches the probe on drop, so the
/// kernel is left clean after the test regardless of pass/fail.
#[test]
fn aa_exec_probes_loads_and_attaches() {
    let mut bpf = Ebpf::load(AA_EXEC_BPF)
        .expect("failed to load aa-exec-probes BPF program — ensure the test is running as root");

    // Attach sched_process_exec tracepoint.
    let exec_program: &mut TracePoint = bpf
        .program_mut("handle_sched_process_exec")
        .expect("handle_sched_process_exec program not found in BPF object")
        .try_into()
        .expect("handle_sched_process_exec is not a TracePoint program");

    exec_program.load().expect("kernel verifier rejected handle_sched_process_exec");

    let _exec_link = exec_program
        .attach("sched", "sched_process_exec")
        .expect("failed to attach handle_sched_process_exec to sched/sched_process_exec");

    // Attach sched_process_exit tracepoint.
    let exit_program: &mut TracePoint = bpf
        .program_mut("handle_sched_process_exit")
        .expect("handle_sched_process_exit program not found in BPF object")
        .try_into()
        .expect("handle_sched_process_exit is not a TracePoint program");

    exit_program.load().expect("kernel verifier rejected handle_sched_process_exit");

    let _exit_link = exit_program
        .attach("sched", "sched_process_exit")
        .expect("failed to attach handle_sched_process_exit to sched/sched_process_exit");

    // Reaching this line confirms the full pipeline works.
    // Links are dropped here, which detaches the probes from the kernel.
}
