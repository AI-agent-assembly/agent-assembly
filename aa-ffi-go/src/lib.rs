//! Go C-ABI static library bindings for Agent Assembly.

use core::ffi::c_char;
use std::sync::Mutex;

pub type AaStatus = i32;

pub const AA_STATUS_OK: AaStatus = 0;
pub const AA_STATUS_NULL_POINTER: AaStatus = 1;
pub const AA_STATUS_INVALID_UTF8: AaStatus = 2;
pub const AA_STATUS_NOT_CONNECTED: AaStatus = 3;
pub const AA_STATUS_MUTEX_POISONED: AaStatus = 4;

#[repr(C)]
pub struct AaBytes {
    pub ptr: *mut u8,
    pub len: usize,
}

#[repr(C)]
pub struct AaString {
    pub ptr: *mut c_char,
}

#[repr(C)]
pub struct aa_client_handle {
    state: Mutex<ClientState>,
}

#[derive(Default)]
struct ClientState {
    endpoint: String,
    connected: bool,
    events_sent: u64,
}
