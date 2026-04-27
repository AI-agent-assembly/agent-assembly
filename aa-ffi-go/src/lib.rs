//! Go C-ABI static library bindings for Agent Assembly.

use core::ffi::c_char;
use std::ffi::CStr;
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

/// # Safety
///
/// `endpoint` and `out_client` must be valid pointers for reads/writes.
#[no_mangle]
pub unsafe extern "C" fn aa_connect(
    endpoint: *const c_char,
    out_client: *mut *mut aa_client_handle,
) -> AaStatus {
    if endpoint.is_null() || out_client.is_null() {
        return AA_STATUS_NULL_POINTER;
    }

    // SAFETY: `endpoint` null-check above ensures pointer validity precondition.
    let endpoint = match unsafe { CStr::from_ptr(endpoint) }.to_str() {
        Ok(value) => value.to_owned(),
        Err(_) => return AA_STATUS_INVALID_UTF8,
    };

    let handle = aa_client_handle {
        state: Mutex::new(ClientState {
            endpoint,
            connected: true,
            events_sent: 0,
        }),
    };

    let raw_handle = Box::into_raw(Box::new(handle));

    // SAFETY: `out_client` null-check above ensures pointer validity precondition.
    unsafe {
        *out_client = raw_handle;
    }

    AA_STATUS_OK
}
