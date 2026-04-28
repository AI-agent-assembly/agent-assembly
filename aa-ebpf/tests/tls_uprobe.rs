// Integration test: load and attach the TLS uprobe BPF programs.
//
// Gated on:
//   - target_os = "linux"  — eBPF is Linux-only
//   - feature = "integration-test" — opted-in explicitly; requires root (CAP_BPF)
//
// Run locally (Linux, as root or via sudo):
//   sudo env "PATH=$PATH" cargo test -p aa-ebpf --features integration-test --test tls_uprobe -- --nocapture
#![cfg(all(target_os = "linux", feature = "integration-test"))]

use aa_ebpf::AA_TLS_BPF;
use aya::{programs::UProbe, Ebpf};

/// Verify the TLS probe pipeline: compile → embed → load into kernel verifier.
///
/// Does *not* attach to a live process (that would require OpenSSL to be
/// present at a known path).  Loading through the verifier is sufficient to
/// confirm that:
///
/// 1. `Ebpf::load` successfully parses the embedded BPF ELF.
/// 2. All three programs pass the kernel verifier (`prog.load()`).
///
/// This test requires `CAP_BPF` + `CAP_PERFMON` (Linux 5.8+).
#[test]
fn aa_tls_probes_load_through_verifier() {
    let mut bpf =
        Ebpf::load(AA_TLS_BPF).expect("failed to load aa-tls-probes BPF object — ensure the test is running as root");

    // Verify ssl_write uprobe program loads.
    let ssl_write: &mut UProbe = bpf
        .program_mut("ssl_write")
        .expect("ssl_write program not found in BPF object")
        .try_into()
        .expect("ssl_write is not a UProbe program");
    ssl_write.load().expect("kernel verifier rejected ssl_write uprobe");

    // Verify ssl_read_entry uprobe program loads.
    let ssl_read_entry: &mut UProbe = bpf
        .program_mut("ssl_read_entry")
        .expect("ssl_read_entry program not found in BPF object")
        .try_into()
        .expect("ssl_read_entry is not a UProbe program");
    ssl_read_entry
        .load()
        .expect("kernel verifier rejected ssl_read_entry uprobe");

    // Verify ssl_read_exit uretprobe program loads.
    let ssl_read_exit: &mut UProbe = bpf
        .program_mut("ssl_read_exit")
        .expect("ssl_read_exit program not found in BPF object")
        .try_into()
        .expect("ssl_read_exit is not a UProbe program");
    ssl_read_exit
        .load()
        .expect("kernel verifier rejected ssl_read_exit uretprobe");
}
