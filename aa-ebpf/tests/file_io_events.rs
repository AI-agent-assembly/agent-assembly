// Integration test: verify that opening /etc/passwd generates a file I/O event.
//
// Gated on:
//   - target_os = "linux"  — eBPF is Linux-only
//   - feature = "integration-test" — opted-in explicitly; requires root (CAP_BPF)
//
// Run locally (Linux, as root or via sudo):
//   sudo env "PATH=$PATH" cargo test -p aa-ebpf --features integration-test --test file_io_events -- --nocapture
#![cfg(all(target_os = "linux", feature = "integration-test"))]

use aa_ebpf::events::FileIoEvent;
use aa_ebpf::AA_FILE_IO_BPF;
use aya::maps::perf::AsyncPerfEventArray;
use aya::programs::KProbe;
use aya::util::online_cpus;
use aya::Ebpf;
use bytes::BytesMut;
use std::fs;
use std::time::Duration;
use tokio::time::timeout;

/// Load all file I/O kprobes and verify that opening /etc/passwd produces
/// an Openat event with the correct path.
#[tokio::test]
async fn openat_etc_passwd_generates_event() {
    let mut bpf = Ebpf::load(AA_FILE_IO_BPF).expect("failed to load BPF program — ensure the test is running as root");

    // Insert our PID into the PID filter so the probes monitor us.
    let pid = std::process::id();
    let mut pid_filter: aya::maps::HashMap<_, u32, u8> = aya::maps::HashMap::try_from(bpf.map_mut("PID_FILTER").unwrap()).unwrap();
    pid_filter.insert(pid, 1, 0).unwrap();

    // Attach the openat entry kprobe.
    let program: &mut KProbe = bpf.program_mut("aa_sys_openat").unwrap().try_into().unwrap();
    program.load().unwrap();
    let _link_entry = program.attach("__x64_sys_openat", 0).unwrap();

    // Attach the openat return kprobe.
    let program: &mut KProbe = bpf.program_mut("aa_sys_openat_ret").unwrap().try_into().unwrap();
    program.load().unwrap();
    let _link_ret = program.attach("__x64_sys_openat", 0).unwrap();

    // Set up the perf event reader.
    let mut perf_array = AsyncPerfEventArray::try_from(bpf.map_mut("EVENTS").unwrap()).unwrap();

    let cpus = online_cpus().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<FileIoEvent>(64);

    for cpu_id in cpus {
        let mut buf = perf_array.open(cpu_id, None).unwrap();
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut buffers = vec![BytesMut::with_capacity(core::mem::size_of::<aa_ebpf_common::FileIoEventRaw>()); 10];
            loop {
                let events = buf.read_events(&mut buffers).await.unwrap();
                for i in 0..events.read {
                    let raw = unsafe { &*(buffers[i].as_ptr() as *const aa_ebpf_common::FileIoEventRaw) };
                    if let Ok(event) = FileIoEvent::from_raw(raw) {
                        let _ = tx.send(event).await;
                    }
                }
            }
        });
    }
    drop(tx);

    // Trigger the event: read /etc/passwd.
    let _contents = fs::read_to_string("/etc/passwd").expect("/etc/passwd should be readable");

    // Wait for the event (with timeout).
    let event = timeout(Duration::from_secs(5), async {
        while let Some(event) = rx.recv().await {
            if event.path.contains("/etc/passwd")
                && event.syscall == aa_ebpf::SyscallKind::Openat
            {
                return Some(event);
            }
        }
        None
    })
    .await
    .expect("timed out waiting for openat event")
    .expect("no matching event received");

    assert_eq!(event.pid, pid);
    assert!(event.path.contains("/etc/passwd"));
}
