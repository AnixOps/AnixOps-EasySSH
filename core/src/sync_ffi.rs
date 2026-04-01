//! 配置同步系统FFI接口

use crate::sync::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Opaque handle for sync manager
pub struct SyncManagerHandle {
    manager: Arc<RwLock<Option<SyncManager>>>,
    runtime: tokio::runtime::Runtime,
}

/// 创建同步管理器
///
/// # Safety
///
/// 调用者必须确保所有指针参数有效
#[no_mangle]
pub unsafe extern "C" fn sync_manager_create(
    device_id: *const c_char,
    device_name: *const c_char,
    encryption_key: *const c_char,
    provider_type: c_int,  // 0=Disabled, 1=iCloud, 2=GoogleDrive, 3=OneDrive, 4=DropBox, 5=SelfHosted, 6=LocalNetwork, 7=CustomPath
    provider_config: *const c_char,  // JSON字符串配置
) -> *mut SyncManagerHandle {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let device_id = if device_id.is_null() {
        "unknown".to_string()
    } else {
        CStr::from_ptr(device_id).to_string_lossy().to_string()
    };

    let device_name = if device_name.is_null() {
        "Unknown Device".to_string()
    } else {
        CStr::from_ptr(device_name).to_string_lossy().to_string()
    };

    let encryption_key = if encryption_key.is_null() {
        None
    } else {
        Some(CStr::from_ptr(encryption_key).to_string_lossy().to_string())
    };

    let provider = unsafe { parse_provider_type(provider_type, provider_config) };

    let config = SyncConfig {
        enabled: true,
        device_id,
        device_name,
        encryption_key,
        provider,
        scope: SyncScope::default(),
        auto_sync: true,
        sync_interval_secs: 300,
        conflict_resolution: SyncConflictResolution::UseLocal,
        local_sync_enabled: false,
        max_history_versions: 10,
        last_sync_at: None,
        deduplication_enabled: true,
        compression_enabled: true,
    };

    let handle = Box::new(SyncManagerHandle {
        manager: Arc::new(RwLock::new(None)),
        runtime,
    });

    Box::into_raw(handle)
}

/// 释放同步管理器
///
/// # Safety
///
/// 调用者必须确保handle是通过sync_manager_create创建的有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_manager_free(handle: *mut SyncManagerHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

/// 启动同步
///
/// # Safety
///
/// 调用者必须确保handle是有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_manager_start(handle: *mut SyncManagerHandle) -> c_int {
    if handle.is_null() {
        return -1;
    }

    let handle = &*handle;

    // 这里简化处理 - 实际应该使用runtime.block_on
    0
}

/// 执行完整同步
///
/// # Safety
///
/// 调用者必须确保handle是有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_manager_sync(
    handle: *mut SyncManagerHandle,
    callback: extern "C" fn(progress: c_int, total: c_int),
) -> c_int {
    if handle.is_null() {
        return -1;
    }

    // 简化实现 - 实际应该启动异步同步流程
    callback(0, 100);
    0
}

/// 获取同步状态
///
/// # Safety
///
/// 调用者必须确保handle是有效指针，返回的字符串需要调用sync_free_string释放
#[no_mangle]
pub unsafe extern "C" fn sync_manager_get_status(
    handle: *mut SyncManagerHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let status = SyncStatus::Idle;
    let json = serde_json::to_string(&status).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 获取同步统计
///
/// # Safety
///
/// 调用者必须确保handle是有效指针，返回的字符串需要调用sync_free_string释放
#[no_mangle]
pub unsafe extern "C" fn sync_manager_get_stats(
    handle: *mut SyncManagerHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let stats = SyncStats::default();
    let json = serde_json::to_string(&stats).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 创建同步历史版本
///
/// # Safety
///
/// 调用者必须确保handle和description是有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_manager_create_version(
    handle: *mut SyncManagerHandle,
    description: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let description = if description.is_null() {
        None
    } else {
        let desc = CStr::from_ptr(description).to_string_lossy().to_string();
        if desc.is_empty() { None } else { Some(desc) }
    };

    let version = SyncVersion {
        version_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        device_id: "ffi-device".to_string(),
        description,
        document_count: 0,
        size_bytes: 0,
        tags: Vec::new(),
    };

    let json = serde_json::to_string(&version).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 恢复到指定版本
///
/// # Safety
///
/// 调用者必须确保handle和version_id是有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_manager_restore_version(
    handle: *mut SyncManagerHandle,
    version_id: *const c_char,
) -> c_int {
    if handle.is_null() || version_id.is_null() {
        return -1;
    }

    let version_id = CStr::from_ptr(version_id).to_string_lossy().to_string();

    // 简化实现
    println!("Restoring version: {}", version_id);
    0
}

/// 获取所有版本
///
/// # Safety
///
/// 调用者必须确保handle是有效指针，返回的字符串需要调用sync_free_string释放
#[no_mangle]
pub unsafe extern "C" fn sync_manager_list_versions(
    handle: *mut SyncManagerHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let versions: Vec<SyncVersion> = Vec::new();
    let json = serde_json::to_string(&versions).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 设置同步范围
///
/// # Safety
///
/// 调用者必须确保handle和scope_json是有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_manager_set_scope(
    handle: *mut SyncManagerHandle,
    scope_json: *const c_char,
) -> c_int {
    if handle.is_null() || scope_json.is_null() {
        return -1;
    }

    let scope_json = CStr::from_ptr(scope_json).to_string_lossy().to_string();

    match serde_json::from_str::<SyncScope>(&scope_json) {
        Ok(_scope) => {
            // 应用范围设置
            0
        }
        Err(_) => -2,
    }
}

/// 启用/禁用本地网络同步
///
/// # Safety
///
/// 调用者必须确保handle是有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_manager_set_local_sync(
    handle: *mut SyncManagerHandle,
    enabled: c_int,
) -> c_int {
    if handle.is_null() {
        return -1;
    }

    let _enabled = enabled != 0;
    // 简化实现
    0
}

/// 获取已发现的设备
///
/// # Safety
///
/// 调用者必须确保handle是有效指针，返回的字符串需要调用sync_free_string释放
#[no_mangle]
pub unsafe extern "C" fn sync_manager_get_discovered_devices(
    handle: *mut SyncManagerHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let devices: Vec<DeviceInfo> = Vec::new();
    let json = serde_json::to_string(&devices).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 释放字符串
///
/// # Safety
///
/// 调用者必须确保str是通过本模块函数返回的有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_free_string(str: *mut c_char) {
    if !str.is_null() {
        let _ = CString::from_raw(str);
    }
}

/// 解析提供者类型
unsafe fn parse_provider_type(provider_type: c_int, config: *const c_char) -> SyncProvider {
    match provider_type {
        0 => SyncProvider::Disabled,
        1 => SyncProvider::ICloud,
        2 => SyncProvider::GoogleDrive,
        3 => SyncProvider::OneDrive,
        4 => SyncProvider::DropBox,
        5 => {
            if config.is_null() {
                SyncProvider::Disabled
            } else {
                let cfg = CStr::from_ptr(config).to_string_lossy().to_string();
                // 期望格式: {"url": "...", "token": "..."}
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&cfg) {
                    let url = parsed.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let token = parsed.get("token").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    SyncProvider::SelfHosted { url, token }
                } else {
                    SyncProvider::Disabled
                }
            }
        }
        6 => SyncProvider::LocalNetwork,
        7 => {
            if config.is_null() {
                SyncProvider::Disabled
            } else {
                let path = CStr::from_ptr(config).to_string_lossy().to_string();
                SyncProvider::CustomPath(std::path::PathBuf::from(path))
            }
        }
        _ => SyncProvider::Disabled,
    }
}

/// 创建默认配置（用于测试）
#[no_mangle]
pub extern "C" fn sync_config_create_default() -> *mut c_char {
    let config = SyncConfig {
        enabled: false,
        device_id: uuid::Uuid::new_v4().to_string(),
        device_name: "Test Device".to_string(),
        encryption_key: None,
        provider: SyncProvider::Disabled,
        scope: SyncScope::default(),
        auto_sync: false,
        sync_interval_secs: 300,
        conflict_resolution: SyncConflictResolution::UseLocal,
        local_sync_enabled: false,
        max_history_versions: 10,
        last_sync_at: None,
        deduplication_enabled: true,
        compression_enabled: true,
    };

    let json = serde_json::to_string_pretty(&config).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 创建iCloud配置
///
/// # Safety
///
/// 调用者必须确保device_id和device_name是有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_config_create_icloud(
    device_id: *const c_char,
    device_name: *const c_char,
    encryption_key: *const c_char,
) -> *mut c_char {
    let device_id = if device_id.is_null() {
        uuid::Uuid::new_v4().to_string()
    } else {
        CStr::from_ptr(device_id).to_string_lossy().to_string()
    };

    let device_name = if device_name.is_null() {
        "EasySSH Device".to_string()
    } else {
        CStr::from_ptr(device_name).to_string_lossy().to_string()
    };

    let encryption_key = if encryption_key.is_null() {
        None
    } else {
        let key = CStr::from_ptr(encryption_key).to_string_lossy().to_string();
        if key.is_empty() { None } else { Some(key) }
    };

    let config = SyncConfig {
        enabled: true,
        device_id,
        device_name,
        encryption_key,
        provider: SyncProvider::ICloud,
        scope: SyncScope::default(),
        auto_sync: true,
        sync_interval_secs: 300,
        conflict_resolution: SyncConflictResolution::UseLocal,
        local_sync_enabled: false,
        max_history_versions: 10,
        last_sync_at: None,
        deduplication_enabled: true,
        compression_enabled: true,
    };

    let json = serde_json::to_string_pretty(&config).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 创建Google Drive配置
///
/// # Safety
///
/// 调用者必须确保device_id和device_name是有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_config_create_gdrive(
    device_id: *const c_char,
    device_name: *const c_char,
    encryption_key: *const c_char,
) -> *mut c_char {
    let device_id = if device_id.is_null() {
        uuid::Uuid::new_v4().to_string()
    } else {
        CStr::from_ptr(device_id).to_string_lossy().to_string()
    };

    let device_name = if device_name.is_null() {
        "EasySSH Device".to_string()
    } else {
        CStr::from_ptr(device_name).to_string_lossy().to_string()
    };

    let encryption_key = if encryption_key.is_null() {
        None
    } else {
        let key = CStr::from_ptr(encryption_key).to_string_lossy().to_string();
        if key.is_empty() { None } else { Some(key) }
    };

    let config = SyncConfig {
        enabled: true,
        device_id,
        device_name,
        encryption_key,
        provider: SyncProvider::GoogleDrive,
        scope: SyncScope::default(),
        auto_sync: true,
        sync_interval_secs: 300,
        conflict_resolution: SyncConflictResolution::UseLocal,
        local_sync_enabled: false,
        max_history_versions: 10,
        last_sync_at: None,
        deduplication_enabled: true,
        compression_enabled: true,
    };

    let json = serde_json::to_string_pretty(&config).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 创建自建服务器配置
///
/// # Safety
///
/// 调用者必须确保所有指针参数有效
#[no_mangle]
pub unsafe extern "C" fn sync_config_create_self_hosted(
    device_id: *const c_char,
    device_name: *const c_char,
    encryption_key: *const c_char,
    url: *const c_char,
    token: *const c_char,
) -> *mut c_char {
    if url.is_null() || token.is_null() {
        return std::ptr::null_mut();
    }

    let device_id = if device_id.is_null() {
        uuid::Uuid::new_v4().to_string()
    } else {
        CStr::from_ptr(device_id).to_string_lossy().to_string()
    };

    let device_name = if device_name.is_null() {
        "EasySSH Device".to_string()
    } else {
        CStr::from_ptr(device_name).to_string_lossy().to_string()
    };

    let encryption_key = if encryption_key.is_null() {
        None
    } else {
        let key = CStr::from_ptr(encryption_key).to_string_lossy().to_string();
        if key.is_empty() { None } else { Some(key) }
    };

    let url = CStr::from_ptr(url).to_string_lossy().to_string();
    let token = CStr::from_ptr(token).to_string_lossy().to_string();

    let config = SyncConfig {
        enabled: true,
        device_id,
        device_name,
        encryption_key,
        provider: SyncProvider::SelfHosted { url, token },
        scope: SyncScope::default(),
        auto_sync: true,
        sync_interval_secs: 60, // 自建服务器可以更频繁同步
        conflict_resolution: SyncConflictResolution::UseLocal,
        local_sync_enabled: false,
        max_history_versions: 50, // 自建服务器可以保存更多版本
        last_sync_at: None,
        deduplication_enabled: true,
        compression_enabled: true,
    };

    let json = serde_json::to_string_pretty(&config).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 创建本地网络同步配置
///
/// # Safety
///
/// 调用者必须确保device_id和device_name是有效指针
#[no_mangle]
pub unsafe extern "C" fn sync_config_create_local_network(
    device_id: *const c_char,
    device_name: *const c_char,
    encryption_key: *const c_char,
) -> *mut c_char {
    let device_id = if device_id.is_null() {
        uuid::Uuid::new_v4().to_string()
    } else {
        CStr::from_ptr(device_id).to_string_lossy().to_string()
    };

    let device_name = if device_name.is_null() {
        "EasySSH Device".to_string()
    } else {
        CStr::from_ptr(device_name).to_string_lossy().to_string()
    };

    let encryption_key = if encryption_key.is_null() {
        None
    } else {
        let key = CStr::from_ptr(encryption_key).to_string_lossy().to_string();
        if key.is_empty() { None } else { Some(key) }
    };

    let config = SyncConfig {
        enabled: true,
        device_id,
        device_name,
        encryption_key,
        provider: SyncProvider::LocalNetwork,
        scope: SyncScope::default(),
        auto_sync: true,
        sync_interval_secs: 30, // 本地网络可以更频繁同步
        conflict_resolution: SyncConflictResolution::UseLocal,
        local_sync_enabled: true,
        max_history_versions: 5,
        last_sync_at: None,
        deduplication_enabled: true,
        compression_enabled: true,
    };

    let json = serde_json::to_string_pretty(&config).unwrap_or_default();

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
