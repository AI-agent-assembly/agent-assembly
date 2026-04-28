#![no_std]
#![no_main]

mod helpers;
mod maps;

use aa_ebpf_common::MAX_PATH_LEN;
use aya_ebpf::{
    helpers::bpf_probe_read_user_str_bytes,
    macros::{kprobe, kretprobe},
    programs::ProbeContext,
};

use crate::helpers::{get_pid_tgid, should_monitor};
use crate::maps::OPENAT_TMP;

/// kprobe on `sys_openat` — captures the filename argument and stashes
/// it in `OPENAT_TMP` keyed by `pid_tgid` so the kretprobe can pair it
/// with the returned fd.
#[kprobe]
pub fn aa_sys_openat(ctx: ProbeContext) -> u32 {
    match try_sys_openat(&ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_sys_openat(ctx: &ProbeContext) -> Result<u32, u32> {
    let (tgid, _pid) = get_pid_tgid();
    if !should_monitor(tgid) {
        return Ok(0);
    }

    // arg1 = const char __user *filename
    let filename_ptr: *const u8 = unsafe { ctx.arg(1).ok_or(1u32)? };

    let mut buf = [0u8; MAX_PATH_LEN];
    unsafe {
        let _ = bpf_probe_read_user_str_bytes(filename_ptr, &mut buf);
    }

    let pid_tgid = unsafe { aya_ebpf::helpers::bpf_get_current_pid_tgid() };
    let _ = OPENAT_TMP.insert(&pid_tgid, &buf, 0);

    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
