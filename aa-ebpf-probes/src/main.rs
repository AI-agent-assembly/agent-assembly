#![no_std]
#![no_main]

mod helpers;
mod maps;

use aa_ebpf_common::{FdPathKey, SyscallType, MAX_PATH_LEN};
use aya_ebpf::{
    helpers::bpf_probe_read_user_str_bytes,
    macros::{kprobe, kretprobe},
    programs::ProbeContext,
};

use crate::helpers::{emit_event, get_pid_tgid, should_monitor};
use crate::maps::{FD_PATH_MAP, OPENAT_TMP, PATH_BLOCKLIST};

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

/// kretprobe on `sys_openat` — pairs the returned fd with the filename
/// captured by the entry kprobe, caches it in `FD_PATH_MAP`, checks the
/// path blocklist, and emits a `FileIoEventRaw`.
#[kretprobe]
pub fn aa_sys_openat_ret(ctx: ProbeContext) -> u32 {
    match try_sys_openat_ret(&ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_sys_openat_ret(ctx: &ProbeContext) -> Result<u32, u32> {
    let (tgid, pid) = get_pid_tgid();
    if !should_monitor(tgid) {
        return Ok(0);
    }

    let pid_tgid = unsafe { aya_ebpf::helpers::bpf_get_current_pid_tgid() };

    // Retrieve the filename stashed by the entry kprobe.
    let path = unsafe { OPENAT_TMP.get(&pid_tgid).ok_or(1u32)? };
    let path_copy = *path;

    // Clean up the temporary entry.
    let _ = OPENAT_TMP.remove(&pid_tgid);

    // rc is the returned fd (or negative errno).
    let rc: i64 = ctx.ret().ok_or(1u32)?;

    // Cache (pid, fd) → path for read/write fd resolution.
    if rc >= 0 {
        let key = FdPathKey {
            pid: tgid,
            fd: rc as u64,
        };
        let _ = FD_PATH_MAP.insert(&key, &path_copy, 0);
    }

    // Determine flags: bit 0 = blocklist hit (sensitive path alert).
    let flags = if unsafe { PATH_BLOCKLIST.get(&path_copy).is_some() } {
        1u32
    } else {
        0u32
    };

    emit_event(ctx, tgid, pid, SyscallType::Openat, &path_copy, flags, rc);

    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
