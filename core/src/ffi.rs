//! FFI bindings for EasySSH Core
//!
//! This module provides C-compatible interfaces for platform native UIs.

use crate::db::{Database, NewGroup, NewServer};
use crate::{AppState, Edition};
use std::ffi::{c_char, c_int, CStr, CString};
use std::ptr;
use std::sync::Mutex;

/// Opaque handle to AppState
pub struct EasySSHAppState {
    inner: Mutex<AppState>,
    db: Mutex<Option<Database>>,
}

/// Initialize the EasySSH core library
///
/// # Safety
/// Must be called before any other FFI function.
/// Returns a handle that must be freed with `easyssh_destroy`.
#[no_mangle]
pub extern "C" fn easyssh_init() -> *mut EasySSHAppState {
    let state = EasySSHAppState {
        inner: Mutex::new(AppState::new()),
        db: Mutex::new(None),
    };

    // Initialize database
    let db_path = crate::get_db_path();
    if let Ok(db) = Database::new(db_path) {
        if db.init().is_ok() {
            *state.db.lock().unwrap() = Some(db);
        }
    }

    Box::into_raw(Box::new(state))
}

/// Destroy the EasySSH app state
///
/// # Safety
/// Must be called with a valid handle returned from `easyssh_init`.
/// Handle must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn easyssh_destroy(handle: *mut EasySSHAppState) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Get the library version
///
/// # Safety
/// Returns a null-terminated string. Caller must not free.
#[no_mangle]
pub extern "C" fn easyssh_version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const c_char
}

/// Get edition info as JSON string
///
/// # Safety
/// Caller must free the returned string with `easyssh_free_string`.
#[no_mangle]
pub extern "C" fn easyssh_get_edition(handle: *mut EasySSHAppState) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let edition = Edition::current();
    let json = match serde_json::to_string(&edition) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

// ==================== Server Operations ====================

/// Get all servers as JSON array
///
/// # Safety
/// Caller must free the returned string with `easyssh_free_string`.
#[no_mangle]
pub unsafe extern "C" fn easyssh_get_servers(handle: *mut EasySSHAppState) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let state = unsafe { &*handle };
    let db_guard = state.db.lock().unwrap();

    if let Some(ref db) = *db_guard {
        match db.get_servers() {
            Ok(servers) => {
                let json = match serde_json::to_string(&servers) {
                    Ok(s) => s,
                    Err(_) => "[]".to_string(),
                };
                match CString::new(json) {
                    Ok(cstr) => cstr.into_raw(),
                    Err(_) => ptr::null_mut(),
                }
            }
            Err(_) => {
                match CString::new("[]") {
                    Ok(cstr) => cstr.into_raw(),
                    Err(_) => ptr::null_mut(),
                }
            }
        }
    } else {
        ptr::null_mut()
    }
}

/// Add a new server
///
/// # Safety
/// `json_config` must be a valid null-terminated UTF-8 string containing NewServer JSON.
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub unsafe extern "C" fn easyssh_add_server(
    handle: *mut EasySSHAppState,
    json_config: *const c_char,
) -> c_int {
    if handle.is_null() || json_config.is_null() {
        return -1;
    }

    let c_str = match CStr::from_ptr(json_config).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let new_server: NewServer = match serde_json::from_str(c_str) {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let state = &*handle;
    let db_guard = state.db.lock().unwrap();

    if let Some(ref db) = *db_guard {
        match db.add_server(&new_server) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Delete a server by ID
///
/// # Safety
/// `server_id` must be a valid null-terminated UTF-8 string.
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub unsafe extern "C" fn easyssh_delete_server(
    handle: *mut EasySSHAppState,
    server_id: *const c_char,
) -> c_int {
    if handle.is_null() || server_id.is_null() {
        return -1;
    }

    let id = match CStr::from_ptr(server_id).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let state = &*handle;
    let db_guard = state.db.lock().unwrap();

    if let Some(ref db) = *db_guard {
        match db.delete_server(id) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Connect to server using native terminal
///
/// # Safety
/// `server_id` must be a valid null-terminated UTF-8 string.
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub unsafe extern "C" fn easyssh_connect_native(
    handle: *mut EasySSHAppState,
    server_id: *const c_char,
) -> c_int {
    if handle.is_null() || server_id.is_null() {
        return -1;
    }

    let id = match CStr::from_ptr(server_id).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let state = &*handle;
    let inner = state.inner.lock().unwrap();

    match crate::connect_server(&inner, id) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// ==================== Group Operations ====================

/// Get all groups as JSON array
///
/// # Safety
/// Caller must free the returned string with `easyssh_free_string`.
#[no_mangle]
pub unsafe extern "C" fn easyssh_get_groups(handle: *mut EasySSHAppState) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let state = unsafe { &*handle };
    let db_guard = state.db.lock().unwrap();

    if let Some(ref db) = *db_guard {
        match db.get_groups() {
            Ok(groups) => {
                let json = match serde_json::to_string(&groups) {
                    Ok(s) => s,
                    Err(_) => "[]".to_string(),
                };
                match CString::new(json) {
                    Ok(cstr) => cstr.into_raw(),
                    Err(_) => ptr::null_mut(),
                }
            }
            Err(_) => {
                match CString::new("[]") {
                    Ok(cstr) => cstr.into_raw(),
                    Err(_) => ptr::null_mut(),
                }
            }
        }
    } else {
        ptr::null_mut()
    }
}

/// Add a new group
///
/// # Safety
/// `json_config` must be valid null-terminated UTF-8 JSON.
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub unsafe extern "C" fn easyssh_add_group(
    handle: *mut EasySSHAppState,
    json_config: *const c_char,
) -> c_int {
    if handle.is_null() || json_config.is_null() {
        return -1;
    }

    let c_str = match CStr::from_ptr(json_config).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let new_group: NewGroup = match serde_json::from_str(c_str) {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let state = &*handle;
    let db_guard = state.db.lock().unwrap();

    if let Some(ref db) = *db_guard {
        match db.add_group(&new_group) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

// ==================== Utility Functions ====================

/// Free a string returned by FFI functions
///
/// # Safety
/// Must be called with a string returned from easyssh_ functions.
/// String must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn easyssh_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Get the last error message
///
/// # Safety
/// Returns a null-terminated string. Caller must not free.
/// String is only valid until next FFI call.
#[no_mangle]
pub extern "C" fn easyssh_last_error() -> *const c_char {
    c"No error".as_ptr()
}
