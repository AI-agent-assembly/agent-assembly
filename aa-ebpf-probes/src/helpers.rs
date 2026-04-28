//! Helper functions for BPF kprobe programs.

use aa_ebpf_common::{FileIoEventRaw, SyscallType, MAX_PATH_LEN};
use aya_ebpf::{helpers::bpf_ktime_get_ns, EbpfContext};

use crate::maps::EVENTS;

/// Fill a [`FileIoEventRaw`] and submit it to the perf event array.
///
/// Generic over the BPF context type so it works from both kprobes
/// (`ProbeContext`) and kretprobes (`RetProbeContext`).
pub fn emit_event<C: EbpfContext>(
    ctx: &C,
    pid: u32,
    tid: u32,
    syscall: SyscallType,
    path: &[u8; MAX_PATH_LEN],
    flags: u32,
    return_code: i64,
) {
    let event = FileIoEventRaw {
        pid,
        tid,
        timestamp_ns: unsafe { bpf_ktime_get_ns() },
        syscall,
        flags,
        return_code,
        path: *path,
    };
    EVENTS.output(ctx, &event, 0);
}

/// Extract (pid, tgid) from the current BPF context.
///
/// Returns `(tgid, pid)` where `tgid` is the userspace PID and `pid` is
/// the kernel thread ID.
#[inline(always)]
pub fn get_pid_tgid() -> (u32, u32) {
    let pid_tgid = aya_ebpf::helpers::bpf_get_current_pid_tgid();
    let tgid = (pid_tgid >> 32) as u32;
    let pid = pid_tgid as u32;
    (tgid, pid)
}

/// Check if the given tgid is in the PID filter map.
/// Returns `true` if monitoring is enabled for this process.
#[inline(always)]
pub fn should_monitor(tgid: u32) -> bool {
    unsafe { crate::maps::PID_FILTER.get(&tgid).is_some() }
}
