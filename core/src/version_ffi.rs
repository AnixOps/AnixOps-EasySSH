//! 版本标识系统 FFI 接口扩展
//!
//! 为各平台UI提供额外的构建信息和平台信息查询接口
//!
//! # 安全说明
//!
//! 所有字符串返回都通过 `version_free_string` 释放，避免内存泄漏

use crate::version::{FullBuildInfo, PlatformInfo, VersionCompatibility};
use crate::edition::Edition;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

/// 平台信息 C 结构体
#[repr(C)]
#[derive(Debug)]
pub struct CPlatformInfo {
    pub os: *mut c_char,
    pub arch: *mut c_char,
    pub family: *mut c_char,
    pub is_windows: c_int,
    pub is_macos: c_int,
    pub is_linux: c_int,
    pub is_64bit: c_int,
}

/// 完整构建信息 C 结构体
#[repr(C)]
#[derive(Debug)]
pub struct CFullBuildInfo {
    pub git_branch: *mut c_char,
    pub build_date: *mut c_char,
    pub rustc_version: *mut c_char,
    pub platform: *mut CPlatformInfo,
    pub user_agent: *mut c_char,
    pub version_id: *mut c_char,
}

/// 获取平台信息
///
/// # Safety
///
/// 调用者必须通过 `version_free_platform_info` 释放返回的结构体
#[no_mangle]
pub extern "C" fn version_get_platform_info() -> *mut CPlatformInfo {
    let platform = PlatformInfo::current();

    let c_info = CPlatformInfo {
        os: CString::new(platform.os).unwrap().into_raw(),
        arch: CString::new(platform.arch).unwrap().into_raw(),
        family: CString::new(platform.family).unwrap().into_raw(),
        is_windows: if platform.is_windows() { 1 } else { 0 },
        is_macos: if platform.is_macos() { 1 } else { 0 },
        is_linux: if platform.is_linux() { 1 } else { 0 },
        is_64bit: if platform.is_64bit() { 1 } else { 0 },
    };

    Box::into_raw(Box::new(c_info))
}

/// 释放平台信息结构体
///
/// # Safety
///
/// 必须传入由 `version_get_platform_info` 返回的有效指针
#[no_mangle]
pub unsafe extern "C" fn version_free_platform_info(info: *mut CPlatformInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);

    if !info.os.is_null() {
        let _ = CString::from_raw(info.os);
    }
    if !info.arch.is_null() {
        let _ = CString::from_raw(info.arch);
    }
    if !info.family.is_null() {
        let _ = CString::from_raw(info.family);
    }
}

/// 获取平台显示字符串
///
/// 格式: "os-arch"
#[no_mangle]
pub extern "C" fn version_get_platform_display() -> *mut c_char {
    let platform = PlatformInfo::current();
    CString::new(platform.display()).unwrap().into_raw()
}

/// 获取完整构建信息
///
/// # Safety
///
/// 调用者必须通过 `version_free_build_info` 释放返回的结构体
#[no_mangle]
pub extern "C" fn version_get_build_info() -> *mut CFullBuildInfo {
    let info = FullBuildInfo::current();

    let c_info = CFullBuildInfo {
        git_branch: info
            .git_branch
            .as_ref()
            .map(|s| CString::new(s.clone()).unwrap().into_raw())
            .unwrap_or(std::ptr::null_mut()),
        build_date: CString::new(info.build_date.clone()).unwrap().into_raw(),
        rustc_version: info
            .rustc_version
            .as_ref()
            .map(|s| CString::new(s.clone()).unwrap().into_raw())
            .unwrap_or(std::ptr::null_mut()),
        platform: version_get_platform_info(),
        user_agent: CString::new(info.user_agent.clone()).unwrap().into_raw(),
        version_id: CString::new(info.version_id.clone()).unwrap().into_raw(),
    };

    Box::into_raw(Box::new(c_info))
}

/// 释放完整构建信息结构体
///
/// # Safety
///
/// 必须传入由 `version_get_build_info` 返回的有效指针
#[no_mangle]
pub unsafe extern "C" fn version_free_build_info(info: *mut CFullBuildInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);

    if !info.git_branch.is_null() {
        let _ = CString::from_raw(info.git_branch);
    }
    if !info.build_date.is_null() {
        let _ = CString::from_raw(info.build_date);
    }
    if !info.rustc_version.is_null() {
        let _ = CString::from_raw(info.rustc_version);
    }
    if !info.platform.is_null() {
        version_free_platform_info(info.platform);
    }
    if !info.user_agent.is_null() {
        let _ = CString::from_raw(info.user_agent);
    }
    if !info.version_id.is_null() {
        let _ = CString::from_raw(info.version_id);
    }
}

/// 释放字符串
///
/// # Safety
///
/// 必须传入由本模块函数返回的有效指针
#[no_mangle]
pub unsafe extern "C" fn version_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// 获取User-Agent字符串
#[no_mangle]
pub extern "C" fn version_get_user_agent() -> *mut c_char {
    let info = FullBuildInfo::current();
    CString::new(info.user_agent.clone()).unwrap().into_raw()
}

/// 获取版本ID字符串
///
/// 格式: "edition/version/build_type"
#[no_mangle]
pub extern "C" fn version_get_version_id() -> *mut c_char {
    let info = FullBuildInfo::current();
    CString::new(info.version_id.clone()).unwrap().into_raw()
}

/// 获取详细版本信息字符串
#[no_mangle]
pub extern "C" fn version_get_verbose_info() -> *mut c_char {
    let info = FullBuildInfo::current();
    CString::new(info.display_verbose()).unwrap().into_raw()
}

/// 获取版本摘要字符串
#[no_mangle]
pub extern "C" fn version_get_summary() -> *mut c_char {
    let info = FullBuildInfo::current();
    CString::new(info.summary()).unwrap().into_raw()
}

/// 检查版本兼容性
///
/// # Arguments
///
/// * `from_edition` - 源版本（0=Lite, 1=Standard, 2=Pro）
/// * `to_edition` - 目标版本
///
/// # Returns
///
/// 1 表示兼容，0 表示不兼容
#[no_mangle]
pub extern "C" fn version_check_compatibility(from_edition: c_int, to_edition: c_int) -> c_int {
    let from = edition_from_int(from_edition);
    let to = edition_from_int(to_edition);

    if VersionCompatibility::is_compatible(from, to) {
        1
    } else {
        0
    }
}

/// 获取版本迁移建议
///
/// # Arguments
///
/// * `from_edition` - 源版本（0=Lite, 1=Standard, 2=Pro）
/// * `to_edition` - 目标版本
///
/// # Returns
///
/// 建议字符串（需通过 `version_free_string` 释放）
#[no_mangle]
pub extern "C" fn version_get_migration_advice(
    from_edition: c_int,
    to_edition: c_int,
) -> *mut c_char {
    let from = edition_from_int(from_edition);
    let to = edition_from_int(to_edition);

    let advice = VersionCompatibility::migration_advice(from, to);
    CString::new(advice).unwrap().into_raw()
}

/// 将整数转换为Edition
fn edition_from_int(value: c_int) -> Edition {
    match value {
        0 => Edition::Lite,
        1 => Edition::Standard,
        2 => Edition::Pro,
        _ => Edition::Lite, // 默认回退到Lite
    }
}

/// 获取构建日期
#[no_mangle]
pub extern "C" fn version_get_build_date() -> *mut c_char {
    let info = FullBuildInfo::current();
    CString::new(info.build_date.clone()).unwrap().into_raw()
}

/// 获取Git分支
///
/// 如果没有Git信息，返回null
#[no_mangle]
pub extern "C" fn version_get_git_branch() -> *mut c_char {
    let info = FullBuildInfo::current();
    info.git_branch
        .as_ref()
        .map(|s| CString::new(s.clone()).unwrap().into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// 获取Rust编译器版本
///
/// 如果没有信息，返回null
#[no_mangle]
pub extern "C" fn version_get_rustc_version() -> *mut c_char {
    let info = FullBuildInfo::current();
    info.rustc_version
        .as_ref()
        .map(|s| CString::new(s.clone()).unwrap().into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// 获取构建信息JSON
///
/// 返回完整的构建信息JSON字符串
#[no_mangle]
pub extern "C" fn version_get_build_info_json() -> *mut c_char {
    let info = FullBuildInfo::current();
    let json = info.to_json().unwrap_or_default();
    CString::new(json).unwrap().into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_platform_info_roundtrip() {
        let c_info = version_get_platform_info();
        assert!(!c_info.is_null());

        unsafe {
            let info = &*c_info;
            assert!(!info.os.is_null());
            assert!(!info.arch.is_null());

            // 平台只能是其中之一
            let is_one_of = info.is_windows == 1 || info.is_macos == 1 || info.is_linux == 1;
            assert!(is_one_of);

            version_free_platform_info(c_info);
        }
    }

    #[test]
    fn test_c_build_info_roundtrip() {
        let c_info = version_get_build_info();
        assert!(!c_info.is_null());

        unsafe {
            let info = &*c_info;
            assert!(!info.build_date.is_null());
            assert!(!info.user_agent.is_null());
            assert!(!info.version_id.is_null());
            assert!(!info.platform.is_null());

            version_free_build_info(c_info);
        }
    }

    #[test]
    fn test_version_compatibility_ffi() {
        assert_eq!(version_check_compatibility(0, 1), 1); // Lite -> Standard
        assert_eq!(version_check_compatibility(0, 2), 1); // Lite -> Pro
        assert_eq!(version_check_compatibility(1, 2), 1); // Standard -> Pro
        assert_eq!(version_check_compatibility(2, 1), 0); // Pro -> Standard (不兼容)
        assert_eq!(version_check_compatibility(2, 0), 1); // Pro -> Lite (兼容，但丢失数据)
    }

    #[test]
    fn test_user_agent() {
        let ua = version_get_user_agent();
        assert!(!ua.is_null());

        unsafe {
            let ua_str = CStr::from_ptr(ua).to_string_lossy();
            assert!(ua_str.starts_with("EasySSH/"));
            version_free_string(ua);
        }
    }

    #[test]
    fn test_version_id() {
        let id = version_get_version_id();
        assert!(!id.is_null());

        unsafe {
            let id_str = CStr::from_ptr(id).to_string_lossy();
            assert!(id_str.contains('/'));
            version_free_string(id);
        }
    }

    #[test]
    fn test_build_info_json() {
        let json = version_get_build_info_json();
        assert!(!json.is_null());

        unsafe {
            let json_str = CStr::from_ptr(json).to_string_lossy();
            assert!(json_str.contains("version_info"));
            assert!(json_str.contains("platform"));
            version_free_string(json);
        }
    }
}
