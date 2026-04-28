#![no_std]
#![no_main]

mod helpers;
mod maps;

use aya_ebpf::{macros::kprobe, programs::ProbeContext};
use aya_log_ebpf::info;

/// Minimal kprobe attached to `__x64_sys_write` — validates the BPF
/// compilation pipeline end-to-end. Replace with real probes in
/// AAASM-37 / AAASM-38 / AAASM-39.
#[kprobe]
pub fn aa_hello(ctx: ProbeContext) -> u32 {
    match try_aa_hello(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_aa_hello(ctx: ProbeContext) -> Result<u32, u32> {
    info!(&ctx, "aa-hello: __x64_sys_write intercepted");
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
