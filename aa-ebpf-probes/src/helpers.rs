//! Helper functions for BPF kprobe programs.

use aa_ebpf_common::FileIoEventRaw;
use aya_ebpf::{helpers::bpf_ktime_get_ns, EbpfContext};

use crate::maps::EVENTS;

/// Set the timestamp on a caller-constructed [`FileIoEventRaw`] and
/// submit it to the perf event array.
///
/// Generic over the BPF context type so it works from both kprobes
/// (`ProbeContext`) and kretprobes (`RetProbeContext`).
///
/// Accepts only two arguments (ctx + event) so it stays within the
/// BPF calling convention limit of 5 register arguments.
///
/// `#[inline(never)]` keeps this in its own stack frame. The caller
/// owns the `FileIoEventRaw` (~290 bytes), and this function adds
/// almost nothing — so neither frame exceeds the 512-byte BPF limit.
#[inline(never)]
pub fn emit_event<C: EbpfContext>(ctx: &C, event: &mut FileIoEventRaw) {
    event.timestamp_ns = unsafe { bpf_ktime_get_ns() };
    EVENTS.output(ctx, event, 0);
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
