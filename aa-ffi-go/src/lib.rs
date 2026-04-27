//! Go C-ABI static library bindings for Agent Assembly.

use core::ffi::c_char;
use std::ffi::{CStr, CString};
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

/// # Safety
///
/// `client` and `event_json` must be valid pointers for reads.
#[no_mangle]
pub unsafe extern "C" fn aa_send_event(
    client: *mut aa_client_handle,
    event_json: *const c_char,
) -> AaStatus {
    if client.is_null() || event_json.is_null() {
        return AA_STATUS_NULL_POINTER;
    }

    // SAFETY: `event_json` null-check above ensures pointer validity precondition.
    if unsafe { CStr::from_ptr(event_json) }.to_str().is_err() {
        return AA_STATUS_INVALID_UTF8;
    }

    // SAFETY: `client` null-check above ensures pointer validity precondition.
    let client_ref = unsafe { &*client };
    let mut state = match client_ref.state.lock() {
        Ok(guard) => guard,
        Err(_) => return AA_STATUS_MUTEX_POISONED,
    };

    if !state.connected {
        return AA_STATUS_NOT_CONNECTED;
    }

    state.events_sent = state.events_sent.saturating_add(1);
    AA_STATUS_OK
}

/// # Safety
///
/// `client`, `query_json`, and `out_response` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn aa_query_policy(
    client: *mut aa_client_handle,
    query_json: *const c_char,
    out_response: *mut *mut c_char,
) -> AaStatus {
    if client.is_null() || query_json.is_null() || out_response.is_null() {
        return AA_STATUS_NULL_POINTER;
    }

    // SAFETY: `query_json` null-check above ensures pointer validity precondition.
    let query = match unsafe { CStr::from_ptr(query_json) }.to_str() {
        Ok(value) => value.to_owned(),
        Err(_) => return AA_STATUS_INVALID_UTF8,
    };

    // SAFETY: `client` null-check above ensures pointer validity precondition.
    let client_ref = unsafe { &*client };
    let state = match client_ref.state.lock() {
        Ok(guard) => guard,
        Err(_) => return AA_STATUS_MUTEX_POISONED,
    };

    if !state.connected {
        return AA_STATUS_NOT_CONNECTED;
    }

    let response_json = serde_json::json!({
        "allow": true,
        "reason": "stub-policy",
        "endpoint": state.endpoint,
        "events_sent": state.events_sent,
        "query": query,
    })
    .to_string();

    let response = match CString::new(response_json) {
        Ok(value) => value,
        Err(_) => return AA_STATUS_INVALID_UTF8,
    };

    let raw_response = response.into_raw();

    // SAFETY: `out_response` null-check above ensures pointer validity precondition.
    unsafe {
        *out_response = raw_response;
    }

    AA_STATUS_OK
}
