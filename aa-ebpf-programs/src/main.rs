//! eBPF kernel-space programs for Agent Assembly — Layer 3 interception.
//!
//! Each function in this file is an eBPF program compiled for the
//! `bpfel-unknown-none` target and loaded into the Linux kernel by the
//! userspace loader in `aa-ebpf`.
//!
//! ## Programs
//!
//! | Program | Probe type | Task |
//! |---------|-----------|------|
//! | [`ssl_write_uprobe`] | uprobe on `SSL_write` | AAASM-37 |
//! | [`ssl_read_uretprobe`] | uretprobe on `SSL_read` | AAASM-37 |
//! | [`openat_kprobe`] | kprobe on `do_sys_openat2` | AAASM-38 |
//! | [`write_kprobe`] | kprobe on `ksys_write` | AAASM-38 |
//! | [`unlink_kprobe`] | kprobe on `do_unlinkat` | AAASM-38 |
//! | [`sched_process_exec`] | tracepoint on `sched/sched_process_exec` | AAASM-39 |

#![no_std]
#![no_main]

use aya_ebpf::{macros::kprobe, macros::tracepoint, macros::uprobe, macros::uretprobe};
use aya_ebpf::{programs::KProbeContext, programs::TracePointContext, programs::UProbeContext};

// ---------------------------------------------------------------------------
// AAASM-37 — OpenSSL TLS plaintext capture (uprobe / uretprobe)
// ---------------------------------------------------------------------------

/// Uprobe on `SSL_write`: captures plaintext before TLS encryption.
///
/// Target symbols: `SSL_write` (OpenSSL 1.1.x and 3.x).
/// TODO(AAASM-37): read plaintext buffer, write TlsCaptureEvent to ring buffer.
#[uprobe]
pub fn ssl_write_uprobe(_ctx: UProbeContext) -> u32 {
    0
}

/// Uretprobe on `SSL_read`: captures plaintext after TLS decryption.
///
/// Target symbols: `SSL_read` (OpenSSL 1.1.x and 3.x).
/// TODO(AAASM-37): read return buffer, write TlsCaptureEvent to ring buffer.
#[uretprobe]
pub fn ssl_read_uretprobe(_ctx: UProbeContext) -> u32 {
    0
}

// ---------------------------------------------------------------------------
// AAASM-38 — File I/O kprobes
// ---------------------------------------------------------------------------

/// Kprobe on `do_sys_openat2`: intercepts file open attempts.
///
/// TODO(AAASM-38): extract dfd + filename, write FileEvent to ring buffer.
#[kprobe]
pub fn openat_kprobe(_ctx: KProbeContext) -> u32 {
    0
}

/// Kprobe on `ksys_write`: intercepts data written to file descriptors.
///
/// TODO(AAASM-38): extract fd + count, write FileEvent to ring buffer.
#[kprobe]
pub fn write_kprobe(_ctx: KProbeContext) -> u32 {
    0
}

/// Kprobe on `do_unlinkat`: intercepts file deletion attempts.
///
/// TODO(AAASM-38): extract pathname, write FileEvent to ring buffer.
#[kprobe]
pub fn unlink_kprobe(_ctx: KProbeContext) -> u32 {
    0
}

// ---------------------------------------------------------------------------
// AAASM-39 — Process exec tracepoints
// ---------------------------------------------------------------------------

/// Tracepoint on `sched/sched_process_exec`: fires on every execve call.
///
/// TODO(AAASM-39): extract pid, ppid, uid, filename, argv, write ExecEvent
/// to ring buffer.
#[tracepoint]
pub fn sched_process_exec(_ctx: TracePointContext) -> u32 {
    0
}

// Required by #![no_std] / #![no_main] for the BPF target.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
