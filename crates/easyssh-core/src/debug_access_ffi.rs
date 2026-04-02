//! Debug Access FFI 接口
//!
//! 为平台UI (WinUI/GTK4/SwiftUI) 提供C兼容的接口

use crate::debug_access::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint};

/// 初始化全局Debug访问控制器
/// 返回0表示成功，非0表示失败
#[no_mangle]
pub extern "C" fn debug_access_init() -> c_int {
    let _ = init_global_debug_access();
    0
}

/// 检查Debug模式是否已启用
#[no_mangle]
pub extern "C" fn debug_access_is_enabled() -> c_int {
    if is_debug_enabled() {
        1
    } else {
        0
    }
}

/// 尝试从环境变量激活Debug模式
/// 返回0表示成功，非0表示失败
#[no_mangle]
pub extern "C" fn debug_access_activate_from_env() -> c_int {
    match try_activate_from_env() {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

/// 获取Debug会话ID
/// 调用者需要在使用后调用 debug_access_free_string 释放返回的字符串
#[no_mangle]
pub extern "C" fn debug_access_get_session_id() -> *mut c_char {
    if let Some(access) = get_debug_access() {
        if let Some(id) = access.get_session_id() {
            return CString::new(id).unwrap_or_default().into_raw();
        }
    }
    std::ptr::null_mut()
}

/// 获取访问级别
/// 返回: 0=未启用, 1=Viewer, 2=Developer, 3=Admin
#[no_mangle]
pub extern "C" fn debug_access_get_level() -> c_int {
    if let Some(access) = get_debug_access() {
        match access.get_access_level() {
            Some(DebugAccessLevel::Viewer) => 1,
            Some(DebugAccessLevel::Developer) => 2,
            Some(DebugAccessLevel::Admin) => 3,
            None => 0,
        }
    } else {
        0
    }
}

/// 检查是否有权限访问特定功能
/// feature_id: 功能ID (1=AI编程, 2=性能监控, 3=网络检查, 4=数据库控制台, 5=日志查看, 6=测试运行, 7=特性开关)
/// 返回: 1=有权限, 0=无权限
#[no_mangle]
pub extern "C" fn debug_access_can_use_feature(feature_id: c_int) -> c_int {
    let feature = match feature_id {
        1 => DebugFeature::AiProgramming,
        2 => DebugFeature::PerformanceMonitor,
        3 => DebugFeature::NetworkInspector,
        4 => DebugFeature::DatabaseConsole,
        5 => DebugFeature::LogViewer,
        6 => DebugFeature::TestRunner,
        7 => DebugFeature::FeatureFlags,
        _ => return 0,
    };

    if let Some(access) = get_debug_access() {
        if access.can_access_feature(feature) {
            return 1;
        }
    }
    0
}

/// 停用Debug模式
/// 返回0表示成功
#[no_mangle]
pub extern "C" fn debug_access_deactivate(reason: *const c_char) -> c_int {
    if let Some(access) = get_debug_access() {
        let reason_str = if reason.is_null() {
            "manual"
        } else {
            unsafe { CStr::from_ptr(reason).to_str().unwrap_or("manual") }
        };
        match access.deactivate(reason_str) {
            Ok(_) => 0,
            Err(_) => 1,
        }
    } else {
        1
    }
}

/// 设置超时时间 (秒)
#[no_mangle]
pub extern "C" fn debug_access_set_timeout(seconds: c_uint) {
    if let Some(access) = get_debug_access() {
        access.set_timeout(seconds as u64);
    }
}

/// 获取超时时间 (秒)
#[no_mangle]
pub extern "C" fn debug_access_get_timeout() -> c_uint {
    if let Some(access) = get_debug_access() {
        access.get_timeout() as c_uint
    } else {
        3600 // 默认1小时
    }
}

/// 设置是否显示Debug指示器
#[no_mangle]
pub extern "C" fn debug_access_set_show_indicator(show: c_int) {
    if let Some(access) = get_debug_access() {
        access.set_show_indicator(show != 0);
    }
}

/// 是否应该显示Debug指示器
#[no_mangle]
pub extern "C" fn debug_access_should_show_indicator() -> c_int {
    if let Some(access) = get_debug_access() {
        if access.should_show_indicator() {
            1
        } else {
            0
        }
    } else {
        0
    }
}

/// 释放由本模块分配的字符串
#[no_mangle]
pub extern "C" fn debug_access_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// 获取当前版本类型
/// 返回: 1=Lite, 2=Standard, 3=Pro
#[no_mangle]
pub extern "C" fn debug_access_get_edition() -> c_int {
    use crate::edition::Edition;
    match Edition::current() {
        Edition::Lite => 1,
        Edition::Standard => 2,
        Edition::Pro => 3,
    }
}

/// 检查AI编程接口是否可用 (通过debug access启用)
#[no_mangle]
pub extern "C" fn debug_access_is_ai_enabled() -> c_int {
    if let Some(access) = get_debug_access() {
        if access.can_access_feature(DebugFeature::AiProgramming) {
            return 1;
        }
    }
    0
}

/// 记录功能访问审计日志
/// feature_id: 功能ID
/// 返回: 0=成功, 1=失败
#[no_mangle]
pub extern "C" fn debug_access_log_feature_access(feature_id: c_int) -> c_int {
    let feature = match feature_id {
        1 => DebugFeature::AiProgramming,
        2 => DebugFeature::PerformanceMonitor,
        3 => DebugFeature::NetworkInspector,
        4 => DebugFeature::DatabaseConsole,
        5 => DebugFeature::LogViewer,
        6 => DebugFeature::TestRunner,
        7 => DebugFeature::FeatureFlags,
        _ => return 1,
    };

    if let Some(access) = get_debug_access() {
        match access.log_feature_access(feature) {
            Ok(_) => 0,
            Err(_) => 1,
        }
    } else {
        1
    }
}

// ============ 组合键检测器 FFI ============

/// 创建Lite版本组合键检测器
/// 返回检测器句柄
#[no_mangle]
pub extern "C" fn key_detector_create_lite() -> *mut KeySequenceDetector {
    let detector = create_lite_key_detector();
    Box::into_raw(Box::new(detector))
}

/// 创建Standard版本组合键检测器
/// 返回检测器句柄
#[no_mangle]
pub extern "C" fn key_detector_create_standard() -> *mut KeySequenceDetector {
    let detector = create_standard_key_detector();
    Box::into_raw(Box::new(detector))
}

/// 销毁组合键检测器
#[no_mangle]
pub extern "C" fn key_detector_destroy(detector: *mut KeySequenceDetector) {
    if !detector.is_null() {
        unsafe {
            let _ = Box::from_raw(detector);
        }
    }
}

/// 处理按键事件
/// detector: 检测器句柄
/// key: 按键字符串
/// 返回: 1=序列完成, 0=未完成
#[no_mangle]
pub extern "C" fn key_detector_on_key(
    detector: *mut KeySequenceDetector,
    key: *const c_char,
) -> c_int {
    if detector.is_null() || key.is_null() {
        return 0;
    }

    unsafe {
        let detector = &*detector;
        let key_str = CStr::from_ptr(key).to_str().unwrap_or("");
        if detector.on_key(key_str) {
            1
        } else {
            0
        }
    }
}

/// 重置检测器
#[no_mangle]
pub extern "C" fn key_detector_reset(detector: *mut KeySequenceDetector) {
    if !detector.is_null() {
        unsafe {
            (*detector).reset();
        }
    }
}

/// 获取检测进度
/// detector: 检测器句柄
/// current: 输出当前进度
/// total: 输出总步数
#[no_mangle]
pub extern "C" fn key_detector_get_progress(
    detector: *mut KeySequenceDetector,
    current: *mut c_int,
    total: *mut c_int,
) {
    if !detector.is_null() && !current.is_null() && !total.is_null() {
        unsafe {
            let (c, t) = (*detector).get_progress();
            *current = c as c_int;
            *total = t as c_int;
        }
    }
}

// ============ 简单激活函数 (用于UI快速调用) ============

/// 从CLI参数快速激活
/// 参数以空格分隔的字符串传入
/// 返回: 0=成功, 1=失败
#[no_mangle]
pub extern "C" fn debug_access_quick_activate_from_cli(args: *const c_char) -> c_int {
    if args.is_null() {
        return 1;
    }

    let args_str = unsafe { CStr::from_ptr(args).to_str().unwrap_or("") };
    let args_vec: Vec<String> = args_str.split_whitespace().map(|s| s.to_string()).collect();

    match try_activate_from_cli(&args_vec) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_init() {
        assert_eq!(debug_access_init(), 0);
    }

    #[test]
    fn test_ffi_edition() {
        let edition = debug_access_get_edition();
        assert!(edition >= 1 && edition <= 3);
    }

    #[test]
    fn test_ffi_key_detector() {
        let detector = key_detector_create_lite();
        assert!(!detector.is_null());

        // Lite需要按3次
        let key = CString::new("ctrl+shift+d").unwrap();
        assert_eq!(key_detector_on_key(detector, key.as_ptr()), 0);
        assert_eq!(key_detector_on_key(detector, key.as_ptr()), 0);
        assert_eq!(key_detector_on_key(detector, key.as_ptr()), 1); // 完成

        // 获取进度
        let mut current: c_int = 0;
        let mut total: c_int = 0;
        key_detector_get_progress(detector, &mut current, &mut total);
        assert_eq!(current, 0); // 完成后重置为0
        assert_eq!(total, 3);

        key_detector_destroy(detector);
    }

    #[test]
    fn test_ffi_string_free() {
        let s = CString::new("test").unwrap();
        let ptr = s.into_raw();
        debug_access_free_string(ptr); // 不应该崩溃
    }

    #[test]
    fn test_ffi_timeout() {
        debug_access_init();

        let original = debug_access_get_timeout();
        debug_access_set_timeout(1800);
        assert_eq!(debug_access_get_timeout(), 1800);

        // 恢复原值
        debug_access_set_timeout(original);
    }
}
