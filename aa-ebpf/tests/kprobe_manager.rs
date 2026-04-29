// Integration test: KprobeManager attach/detach lifecycle on a live kernel.
//
// Gated on:
//   - target_os = "linux"  — eBPF is Linux-only
//   - feature = "integration-test" — opted-in explicitly; requires root (CAP_BPF)
//
// Run locally (Linux, as root or via sudo):
//   sudo env "PATH=$PATH" cargo test -p aa-ebpf --features integration-test --test kprobe_manager -- --nocapture
#![cfg(all(target_os = "linux", feature = "integration-test"))]

use aa_ebpf::{kprobe::KprobeManager, AA_FILE_IO_BPF};
use aya::Ebpf;

/// Verify that KprobeManager::attach() loads and attaches all 6 kprobes,
/// is_attached() returns true, and detach() cleans up without error.
#[test]
fn kprobe_manager_attach_detach_lifecycle() {
    let mut bpf =
        Ebpf::load(AA_FILE_IO_BPF).expect("failed to load aa-file-io BPF program — ensure the test is running as root");

    let mut mgr = KprobeManager::attach(&mut bpf, None).expect("KprobeManager::attach() failed");

    assert!(mgr.is_attached(), "manager should be attached after attach()");

    mgr.detach();

    assert!(!mgr.is_attached(), "manager should not be attached after detach()");
}

/// Verify that KprobeManager::attach() works with a specific target PID
/// (writes into the PID_FILTER map without error).
#[test]
fn kprobe_manager_attach_with_pid_filter() {
    let mut bpf =
        Ebpf::load(AA_FILE_IO_BPF).expect("failed to load aa-file-io BPF program — ensure the test is running as root");

    let mgr = KprobeManager::attach(&mut bpf, Some(std::process::id() as i32))
        .expect("KprobeManager::attach() with PID filter failed");

    assert!(mgr.is_attached());
    // mgr is dropped here — probes detach automatically.
}
