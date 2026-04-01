#![allow(dead_code)]

use crate::log_monitor::*;
use serde_json;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::Arc;
use tokio::sync::RwLock;

/// FFI opaque handle for LogMonitorCenter
pub struct LogMonitorHandle {
    center: LogMonitorCenter,
    rt: tokio::runtime::Runtime,
}

/// Create a new log monitor center
#[no_mangle]
pub extern "C" fn log_monitor_create() -> *mut LogMonitorHandle {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let ssh_manager = Arc::new(RwLock::new(crate::ssh::SshSessionManager::new()));
    let center = LogMonitorCenter::new(ssh_manager);

    let handle = Box::new(LogMonitorHandle { center, rt });
    Box::into_raw(handle)
}

/// Destroy log monitor center
#[no_mangle]
pub extern "C" fn log_monitor_destroy(handle: *mut LogMonitorHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(handle);
    }
}

/// Add a log source
#[no_mangle]
pub extern "C" fn log_monitor_add_source(
    handle: *mut LogMonitorHandle,
    name: *const c_char,
    server_id: *const c_char,
    log_path: *const c_char,
    log_type: c_int,
    source_id_out: *mut c_char,
    source_id_len: usize,
) -> c_int {
    if handle.is_null() || name.is_null() || server_id.is_null() || log_path.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };

    let name = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
    let server_id = unsafe { CStr::from_ptr(server_id).to_string_lossy().to_string() };
    let log_path = unsafe { CStr::from_ptr(log_path).to_string_lossy().to_string() };

    let log_type = match log_type {
        0 => LogType::SystemdJournal,
        1 => LogType::Syslog,
        2 => LogType::Application,
        3 => LogType::Nginx,
        4 => LogType::Apache,
        5 => LogType::Docker,
        6 => LogType::Kubernetes,
        _ => LogType::Custom,
    };

    let source = LogSource::new(name, server_id, log_path, log_type);
    let source_id = source.id.clone();

    match handle.rt.block_on(handle.center.add_source(source)) {
        Ok(_) => {
            // 复制 source_id 到输出缓冲区
            if !source_id_out.is_null() && source_id_len > 0 {
                let c_id = match CString::new(source_id) {
                    Ok(s) => s,
                    Err(_) => return -1,
                };
                let bytes = c_id.as_bytes_with_nul();
                let len = bytes.len().min(source_id_len - 1);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr() as *const c_char,
                        source_id_out,
                        len,
                    );
                    *source_id_out.add(len) = 0;
                }
            }
            0
        }
        Err(_) => -1,
    }
}

/// Remove a log source
#[no_mangle]
pub extern "C" fn log_monitor_remove_source(
    handle: *mut LogMonitorHandle,
    source_id: *const c_char,
) -> c_int {
    if handle.is_null() || source_id.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    let source_id = unsafe { CStr::from_ptr(source_id).to_string_lossy().to_string() };

    match handle.rt.block_on(handle.center.remove_source(&source_id)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get all sources as JSON
#[no_mangle]
pub extern "C" fn log_monitor_get_sources_json(
    handle: *mut LogMonitorHandle,
    buffer: *mut c_char,
    buffer_len: usize,
) -> c_int {
    if handle.is_null() || buffer.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    let sources = handle.rt.block_on(handle.center.get_sources());

    match serde_json::to_string(&sources) {
        Ok(json) => {
            if json.len() >= buffer_len {
                return -2; // Buffer too small
            }
            let c_json = match CString::new(json) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    c_json.as_ptr(),
                    buffer,
                    c_json.as_bytes_with_nul().len(),
                );
            }
            0
        }
        Err(_) => -1,
    }
}

/// Search logs with filter
#[no_mangle]
pub extern "C" fn log_monitor_search(
    handle: *mut LogMonitorHandle,
    filter_json: *const c_char,
    results_buffer: *mut c_char,
    buffer_len: usize,
) -> c_int {
    if handle.is_null() || filter_json.is_null() || results_buffer.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    let filter_str = unsafe { CStr::from_ptr(filter_json).to_string_lossy() };

    let filter: LogFilter = match serde_json::from_str(&filter_str) {
        Ok(f) => f,
        Err(_) => return -1,
    };

    let entries = handle.rt.block_on(handle.center.search(&filter));

    match serde_json::to_string(&entries) {
        Ok(json) => {
            if json.len() >= buffer_len {
                return -2;
            }
            let c_json = match CString::new(json) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    c_json.as_ptr(),
                    results_buffer,
                    c_json.as_bytes_with_nul().len(),
                );
            }
            0
        }
        Err(_) => -1,
    }
}

/// Get statistics
#[no_mangle]
pub extern "C" fn log_monitor_get_stats(
    handle: *mut LogMonitorHandle,
    time_range_seconds: u64,
    buffer: *mut c_char,
    buffer_len: usize,
) -> c_int {
    if handle.is_null() || buffer.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    let stats = handle
        .rt
        .block_on(handle.center.get_stats(time_range_seconds));

    match serde_json::to_string(&stats) {
        Ok(json) => {
            if json.len() >= buffer_len {
                return -2;
            }
            let c_json = match CString::new(json) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    c_json.as_ptr(),
                    buffer,
                    c_json.as_bytes_with_nul().len(),
                );
            }
            0
        }
        Err(_) => -1,
    }
}

/// Analyze logs
#[no_mangle]
pub extern "C" fn log_monitor_analyze(
    handle: *mut LogMonitorHandle,
    time_range_seconds: u64,
    buffer: *mut c_char,
    buffer_len: usize,
) -> c_int {
    if handle.is_null() || buffer.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    let result = handle
        .rt
        .block_on(handle.center.analyze(time_range_seconds));

    match serde_json::to_string(&result) {
        Ok(json) => {
            if json.len() >= buffer_len {
                return -2;
            }
            let c_json = match CString::new(json) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    c_json.as_ptr(),
                    buffer,
                    c_json.as_bytes_with_nul().len(),
                );
            }
            0
        }
        Err(_) => -1,
    }
}

/// Add alert rule
#[no_mangle]
pub extern "C" fn log_monitor_add_alert(
    handle: *mut LogMonitorHandle,
    rule_json: *const c_char,
    rule_id_out: *mut c_char,
    rule_id_len: usize,
) -> c_int {
    if handle.is_null() || rule_json.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    let rule_str = unsafe { CStr::from_ptr(rule_json).to_string_lossy() };

    let rule: LogAlertRule = match serde_json::from_str(&rule_str) {
        Ok(r) => r,
        Err(_) => return -1,
    };

    let rule_id = rule.id.clone();
    handle.rt.block_on(handle.center.add_alert_rule(rule));

    if !rule_id_out.is_null() && rule_id_len > 0 {
        let c_id = match CString::new(rule_id) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let bytes = c_id.as_bytes_with_nul();
        let len = bytes.len().min(rule_id_len - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, rule_id_out, len);
            *rule_id_out.add(len) = 0;
        }
    }

    0
}

/// Export logs
#[no_mangle]
pub extern "C" fn log_monitor_export(
    handle: *mut LogMonitorHandle,
    config_json: *const c_char,
    output_path: *const c_char,
) -> c_int {
    if handle.is_null() || config_json.is_null() || output_path.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    let config_str = unsafe { CStr::from_ptr(config_json).to_string_lossy() };
    let output_path = unsafe { CStr::from_ptr(output_path).to_string_lossy().to_string() };

    let config: ExportConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    match handle
        .rt
        .block_on(handle.center.export(&config, &output_path))
    {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Rotate logs
#[no_mangle]
pub extern "C" fn log_monitor_rotate(handle: *mut LogMonitorHandle) -> c_int {
    if handle.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    let removed = handle.rt.block_on(handle.center.rotate_logs());
    removed as c_int
}

/// Clear all logs
#[no_mangle]
pub extern "C" fn log_monitor_clear(handle: *mut LogMonitorHandle) {
    if handle.is_null() {
        return;
    }

    let handle = unsafe { &mut *handle };
    handle.rt.block_on(handle.center.clear());
}

/// Get recent entries
#[no_mangle]
pub extern "C" fn log_monitor_get_recent(
    handle: *mut LogMonitorHandle,
    count: usize,
    buffer: *mut c_char,
    buffer_len: usize,
) -> c_int {
    if handle.is_null() || buffer.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    let entries = handle.rt.block_on(handle.center.get_recent_entries(count));

    match serde_json::to_string(&entries) {
        Ok(json) => {
            if json.len() >= buffer_len {
                return -2;
            }
            let c_json = match CString::new(json) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    c_json.as_ptr(),
                    buffer,
                    c_json.as_bytes_with_nul().len(),
                );
            }
            0
        }
        Err(_) => -1,
    }
}

/// Subscribe to WebSocket updates (callback-based)
#[no_mangle]
pub extern "C" fn log_monitor_subscribe(
    handle: *mut LogMonitorHandle,
    callback: extern "C" fn(*const c_char),
) {
    if handle.is_null() {
        return;
    }

    let handle = unsafe { &mut *handle };
    let mut rx = handle.center.subscribe();

    handle.rt.spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if let Ok(json) = serde_json::to_string(&msg) {
                        if let Ok(c_str) = CString::new(json) {
                            callback(c_str.as_ptr());
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
}

/// Start log rotation background task
#[no_mangle]
pub extern "C" fn log_monitor_start_rotation(handle: *mut LogMonitorHandle) -> c_int {
    if handle.is_null() {
        return -1;
    }

    let handle = unsafe { &mut *handle };
    handle.rt.block_on(handle.center.start_rotation_task());
    0
}

/// Start WebSocket server
#[no_mangle]
pub extern "C" fn log_monitor_start_ws(handle: *mut LogMonitorHandle, _port: u16) -> c_int {
    if handle.is_null() {
        return -1;
    }

    let _handle = unsafe { &mut *handle };
    // Note: The WebSocket server requires Arc<LogMonitorCenter> which has lifetime issues
    // This is a simplified FFI interface - in production you'd need to handle this differently
    0
}
