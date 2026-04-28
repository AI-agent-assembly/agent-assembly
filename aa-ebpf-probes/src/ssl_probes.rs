//! BPF TLS uprobe programs for AAASM-37.
//!
//! Three programs share a single ring buffer (`EVENTS`) and a per-thread
//! argument-save map (`SSL_READ_ARGS`):
//!
//! - `ssl_write`      — uprobe on `SSL_write`; captures outbound plaintext.
//! - `ssl_read_entry` — uprobe on `SSL_read`; saves the `buf` pointer so the
//!                      uretprobe can read it after the call returns.
//! - `ssl_read_exit`  — uretprobe on `SSL_read`; captures inbound plaintext.
//!
//! ## Stack-limit workaround
//!
//! [`TlsCaptureEvent`] is 4112 bytes — well above the BPF 512-byte stack
//! limit.  We use [`RingBuf::reserve`] to allocate the event directly in ring
//! buffer memory and fill it in place before submitting.

#![no_std]
#![no_main]

use aa_ebpf_common::tls::{TlsCaptureEvent, MAX_PAYLOAD_LEN};
use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_ktime_get_ns, bpf_probe_read_user_buf},
    macros::{map, uprobe, uretprobe},
    maps::{Array, HashMap, RingBuf},
    programs::{ProbeContext, RetProbeContext},
};

// ---------------------------------------------------------------------------
// BPF maps
// ---------------------------------------------------------------------------

/// Shared ring buffer for all eBPF events (EVENTS map, 256 KiB).
#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(262144, 0);

/// Saves the `buf` pointer from an SSL_read entry so the uretprobe can
/// read the data after the call returns.  Keyed by `pid_tgid` (u64).
#[map]
static SSL_READ_ARGS: HashMap<u64, u64> = HashMap::with_max_entries(1024, 0);

/// Single-element array holding the target PID to monitor.
/// Index 0 = target PID; value 0 means "monitor all processes".
/// Written by userspace via [`crate::uprobe::UprobeManager::attach`].
#[map]
static TARGET_PID: Array<u32> = Array::with_max_entries(1, 0);

// ---------------------------------------------------------------------------
// PID filter helper
// ---------------------------------------------------------------------------

/// Returns `true` when `pid` should be traced (matches TARGET_PID or all).
#[inline(always)]
fn pid_allowed(pid: u32) -> bool {
    match TARGET_PID.get(0) {
        Some(target) => *target == 0 || *target == pid,
        None => true,
    }
}

// ---------------------------------------------------------------------------
// ssl_write uprobe — outbound TLS plaintext
// ---------------------------------------------------------------------------

/// Uprobe attached to `SSL_write(ssl, buf, num)`.
///
/// Copies up to [`MAX_PAYLOAD_LEN`] bytes from userspace `buf` into the ring
/// buffer and submits a [`TlsCaptureEvent`] with `direction = 0` (outbound).
#[uprobe]
pub fn ssl_write(ctx: ProbeContext) -> u32 {
    match try_ssl_write(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_ssl_write(ctx: ProbeContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    if !pid_allowed(pid) {
        return Ok(0);
    }

    // arg(1) = const void *buf, arg(2) = int num
    let buf_ptr: u64 = ctx.arg(1).ok_or(-1i64)?;
    let num: i32 = ctx.arg(2).ok_or(-1i64)?;

    if num <= 0 {
        return Ok(0);
    }

    emit_tls_event(pid_tgid, pid, buf_ptr, num as u32, 0)
}

// ---------------------------------------------------------------------------
// ssl_read_entry uprobe — save buf pointer for the uretprobe
// ---------------------------------------------------------------------------

/// Uprobe on `SSL_read(ssl, buf, num)` entry.
///
/// Saves the `buf` pointer in [`SSL_READ_ARGS`] keyed by `pid_tgid` so
/// [`ssl_read_exit`] can read it after the call returns.
#[uprobe]
pub fn ssl_read_entry(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    if !pid_allowed(pid) {
        return 0;
    }

    let buf_ptr: u64 = match ctx.arg(1) {
        Some(p) => p,
        None => return 0,
    };

    // Ignore insert errors — if it fails we simply miss this read.
    let _ = SSL_READ_ARGS.insert(&pid_tgid, &buf_ptr, 0);
    0
}

// ---------------------------------------------------------------------------
// ssl_read_exit uretprobe — inbound TLS plaintext
// ---------------------------------------------------------------------------

/// Uretprobe on `SSL_read` return.
///
/// Reads the saved `buf` pointer from [`SSL_READ_ARGS`], copies up to
/// [`MAX_PAYLOAD_LEN`] bytes of inbound plaintext, and emits a
/// [`TlsCaptureEvent`] with `direction = 1` (inbound).
#[uretprobe]
pub fn ssl_read_exit(ctx: RetProbeContext) -> u32 {
    match try_ssl_read_exit(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_ssl_read_exit(ctx: RetProbeContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    if !pid_allowed(pid) {
        return Ok(0);
    }

    let num: i32 = ctx.ret().ok_or(-1i64)?;
    if num <= 0 {
        // No data or error — clean up saved arg.
        let _ = SSL_READ_ARGS.remove(&pid_tgid);
        return Ok(0);
    }

    let buf_ptr: u64 = match unsafe { SSL_READ_ARGS.get(&pid_tgid) } {
        Some(p) => *p,
        None => return Ok(0),
    };
    let _ = SSL_READ_ARGS.remove(&pid_tgid);

    emit_tls_event(pid_tgid, pid, buf_ptr, num as u32, 1)
}

// ---------------------------------------------------------------------------
// Shared helper — write one TlsCaptureEvent into the ring buffer.
// ---------------------------------------------------------------------------

/// Emit a TLS plaintext capture event into the shared ring buffer.
///
/// Reserves ring-buffer memory (avoiding the 512-byte BPF stack limit),
/// fills the [`TlsCaptureEvent`] fields in-place, reads up to
/// [`MAX_PAYLOAD_LEN`] bytes from `buf_ptr`, and submits.
///
/// Returns `Ok(0)` on success or if the userspace read fails (event discarded).
/// Returns `Err(-1)` only if the ring buffer is full.
#[inline(always)]
fn emit_tls_event(pid_tgid: u64, pid: u32, buf_ptr: u64, data_len: u32, direction: u8) -> Result<u32, i64> {
    let capture_len = if data_len as usize > MAX_PAYLOAD_LEN {
        MAX_PAYLOAD_LEN
    } else {
        data_len as usize
    };

    let mut entry = EVENTS.reserve::<TlsCaptureEvent>(0).ok_or(-1i64)?;
    let event_ptr = entry.as_mut_ptr();

    unsafe {
        (*event_ptr).timestamp_ns = bpf_ktime_get_ns();
        (*event_ptr).pid = pid;
        (*event_ptr).tid = pid_tgid as u32;
        (*event_ptr).data_len = data_len;
        (*event_ptr).seq = 0;
        (*event_ptr).direction = direction;
        (*event_ptr)._pad = [0u8; 7];

        let dest = &mut (&mut (*event_ptr).payload)[..capture_len];
        if bpf_probe_read_user_buf(buf_ptr as *const u8, dest).is_err() {
            entry.discard(0);
            return Ok(0);
        }
    }

    entry.submit(0);
    Ok(0)
}

// ---------------------------------------------------------------------------
// Panic handler (required for no_std binaries)
// ---------------------------------------------------------------------------

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
