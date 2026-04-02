//! 版本标识系统 FFI 接口
//!
//! 为各平台UI（Windows WinUI、Linux GTK4、macOS SwiftUI）提供版本信息查询接口
//!
//! # 安全说明
//!
//! 所有字符串返回都通过 `edition_free_string` 释放，避免内存泄漏

use crate::edition::{AppIdentity, BuildType, Edition, VersionComparator, VersionInfo};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

/// 版本信息 C 结构体
#[repr(C)]
#[derive(Debug)]
pub struct CVersionInfo {
    pub edition: c_int,       // 0=Lite, 1=Standard, 2=Pro
    pub build_type: c_int,    // 0=Release, 1=Dev
    pub version: *mut c_char, // 版本号字符串（需释放）
    pub edition_name: *mut c_char,
    pub full_name: *mut c_char,
    pub primary_color: *mut c_char,   // 主色调（如 "#10B981"）
    pub secondary_color: *mut c_char, // 次色调
    pub accent_color: *mut c_char,    // 强调色
    pub tagline: *mut c_char,         // 版本描述
    pub feature_count: c_int,         // 功能数量
    pub features: *mut *mut c_char,   // 功能列表（需释放）
}

/// 应用标识 C 结构体
#[repr(C)]
#[derive(Debug)]
pub struct CAppIdentity {
    pub app_name: *mut c_char,
    pub bundle_id: *mut c_char,
    pub vendor: *mut c_char,
    pub data_dir: *mut c_char,
    pub config_path: *mut c_char,
}

/// 获取当前版本信息
///
/// # Safety
///
/// 调用者必须通过 `edition_free_version_info` 释放返回的结构体
#[no_mangle]
pub extern "C" fn edition_get_version_info() -> *mut CVersionInfo {
    let info = VersionInfo::current();

    let features_ptr: Vec<*mut c_char> = info
        .features
        .iter()
        .map(|f| CString::new(f.clone()).unwrap().into_raw())
        .collect();

    let features_box = features_ptr.into_boxed_slice();
    let features_ptr_raw = Box::into_raw(features_box) as *mut *mut c_char;

    let c_info = CVersionInfo {
        edition: match info.edition {
            Edition::Lite => 0,
            Edition::Standard => 1,
            Edition::Pro => 2,
        },
        build_type: match info.build_type {
            BuildType::Release => 0,
            BuildType::Dev => 1,
        },
        version: CString::new(info.version).unwrap().into_raw(),
        edition_name: CString::new(info.edition_name).unwrap().into_raw(),
        full_name: CString::new(info.edition_full_name).unwrap().into_raw(),
        primary_color: CString::new(info.primary_color).unwrap().into_raw(),
        secondary_color: CString::new(info.secondary_color).unwrap().into_raw(),
        accent_color: CString::new(info.accent_color).unwrap().into_raw(),
        tagline: CString::new(info.tagline).unwrap().into_raw(),
        feature_count: info.features.len() as c_int,
        features: features_ptr_raw,
    };

    Box::into_raw(Box::new(c_info))
}

/// 释放版本信息结构体
///
/// # Safety
///
/// 必须传入由 `edition_get_version_info` 返回的有效指针
#[no_mangle]
pub unsafe extern "C" fn edition_free_version_info(info: *mut CVersionInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);

    // 释放所有字符串
    if !info.version.is_null() {
        let _ = CString::from_raw(info.version);
    }
    if !info.edition_name.is_null() {
        let _ = CString::from_raw(info.edition_name);
    }
    if !info.full_name.is_null() {
        let _ = CString::from_raw(info.full_name);
    }
    if !info.primary_color.is_null() {
        let _ = CString::from_raw(info.primary_color);
    }
    if !info.secondary_color.is_null() {
        let _ = CString::from_raw(info.secondary_color);
    }
    if !info.accent_color.is_null() {
        let _ = CString::from_raw(info.accent_color);
    }
    if !info.tagline.is_null() {
        let _ = CString::from_raw(info.tagline);
    }

    // 释放功能列表
    if !info.features.is_null() && info.feature_count > 0 {
        let features = Box::from_raw(std::slice::from_raw_parts_mut(
            info.features,
            info.feature_count as usize,
        ));
        for i in 0..info.feature_count as usize {
            if !features[i].is_null() {
                let _ = CString::from_raw(features[i]);
            }
        }
    }
}

/// 获取应用标识信息
#[no_mangle]
pub extern "C" fn edition_get_app_identity() -> *mut CAppIdentity {
    let identity = AppIdentity::current();

    // Get paths before moving identity fields
    let data_dir = identity.data_dir().to_string_lossy().to_string();
    let config_path = identity.config_path().to_string_lossy().to_string();

    let c_identity = CAppIdentity {
        app_name: CString::new(identity.app_name).unwrap().into_raw(),
        bundle_id: CString::new(identity.bundle_id).unwrap().into_raw(),
        vendor: CString::new(identity.vendor).unwrap().into_raw(),
        data_dir: CString::new(data_dir).unwrap().into_raw(),
        config_path: CString::new(config_path).unwrap().into_raw(),
    };

    Box::into_raw(Box::new(c_identity))
}

/// 释放应用标识结构体
///
/// # Safety
///
/// 调用者必须保证：
/// - `identity` 是由 `edition_get_app_identity` 返回的有效指针
/// - `identity` 不为 null
/// - 此函数只能被调用一次（释放后不可再次使用）
#[no_mangle]
pub unsafe extern "C" fn edition_free_app_identity(identity: *mut CAppIdentity) {
    if identity.is_null() {
        return;
    }

    let identity = Box::from_raw(identity);

    if !identity.app_name.is_null() {
        let _ = CString::from_raw(identity.app_name);
    }
    if !identity.bundle_id.is_null() {
        let _ = CString::from_raw(identity.bundle_id);
    }
    if !identity.vendor.is_null() {
        let _ = CString::from_raw(identity.vendor);
    }
    if !identity.data_dir.is_null() {
        let _ = CString::from_raw(identity.data_dir);
    }
    if !identity.config_path.is_null() {
        let _ = CString::from_raw(identity.config_path);
    }
}

/// 释放字符串
///
/// # Safety
///
/// 必须传入由本模块函数返回的有效指针
#[no_mangle]
pub unsafe extern "C" fn edition_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// 获取窗口标题
#[no_mangle]
pub extern "C" fn edition_get_window_title() -> *mut c_char {
    let info = VersionInfo::current();
    CString::new(info.window_title()).unwrap().into_raw()
}

/// 获取完整版本字符串（用于关于对话框）
#[no_mangle]
pub extern "C" fn edition_get_full_version_string() -> *mut c_char {
    let info = VersionInfo::current();
    CString::new(info.full_version_string()).unwrap().into_raw()
}

/// 获取短版本标识
#[no_mangle]
pub extern "C" fn edition_get_short_version() -> *mut c_char {
    let info = VersionInfo::current();
    CString::new(info.short_version()).unwrap().into_raw()
}

/// 获取构建产物文件名
///
/// # Safety
///
/// 调用者必须保证 `arch` 和 `platform` 是有效的 C 字符串指针
#[no_mangle]
pub unsafe extern "C" fn edition_get_build_artifact_name(
    arch: *const c_char,
    platform: *const c_char,
) -> *mut c_char {
    let arch_str = CStr::from_ptr(arch).to_string_lossy().to_string();
    let platform_str = CStr::from_ptr(platform).to_string_lossy().to_string();

    let info = VersionInfo::current();
    let name = info.build_artifact_name(&arch_str, &platform_str);
    CString::new(name).unwrap().into_raw()
}

/// 检查是否支持指定功能
///
/// # Arguments
///
/// * `feature` - 功能名称
///
/// # Returns
///
/// 1 表示支持，0 表示不支持
///
/// # Safety
///
/// 调用者必须保证 `feature` 是有效的 C 字符串指针
#[no_mangle]
pub unsafe extern "C" fn edition_has_feature(feature: *const c_char) -> c_int {
    let feature_str = CStr::from_ptr(feature).to_string_lossy().to_string();

    let info = VersionInfo::current();
    if info.has_feature(&feature_str) {
        1
    } else {
        0
    }
}

/// 获取当前版本类型（0=Lite, 1=Standard, 2=Pro）
#[no_mangle]
pub extern "C" fn edition_get_current() -> c_int {
    match Edition::current() {
        Edition::Lite => 0,
        Edition::Standard => 1,
        Edition::Pro => 2,
    }
}

/// 获取构建类型（0=Release, 1=Dev）
#[no_mangle]
pub extern "C" fn edition_get_build_type() -> c_int {
    match BuildType::current() {
        BuildType::Release => 0,
        BuildType::Dev => 1,
    }
}

/// 检查是否为开发者模式
#[no_mangle]
pub extern "C" fn edition_is_dev_mode() -> c_int {
    if BuildType::current() == BuildType::Dev {
        1
    } else {
        0
    }
}

/// 比较两个版本号
///
/// # Returns
///
/// - 0: 版本相同
/// - 1: v1 较新
/// - 2: v1 较旧
/// - 3: 不兼容
///
/// # Safety
///
/// 调用者必须保证 `v1` 和 `v2` 是有效的 C 字符串指针
#[no_mangle]
pub unsafe extern "C" fn edition_compare_versions(v1: *const c_char, v2: *const c_char) -> c_int {
    let v1_str = CStr::from_ptr(v1).to_string_lossy();
    let v2_str = CStr::from_ptr(v2).to_string_lossy();

    use crate::edition::VersionComparison;
    match VersionComparator::compare(&v1_str, &v2_str) {
        VersionComparison::Equal => 0,
        VersionComparison::Newer => 1,
        VersionComparison::Older => 2,
        VersionComparison::Incompatible => 3,
    }
}

/// 获取图标文件名
#[no_mangle]
pub extern "C" fn edition_get_icon_filename() -> *mut c_char {
    let info = VersionInfo::current();
    CString::new(info.icon_filename()).unwrap().into_raw()
}

/// 获取 MSI 安装包名
///
/// # Safety
///
/// 调用者必须保证 `arch` 是有效的 C 字符串指针
#[no_mangle]
pub unsafe extern "C" fn edition_get_msi_name(arch: *const c_char) -> *mut c_char {
    let arch_str = CStr::from_ptr(arch).to_string_lossy().to_string();

    let info = VersionInfo::current();
    CString::new(info.msi_name(&arch_str)).unwrap().into_raw()
}

/// 获取 DMG 镜像名
///
/// # Safety
///
/// 调用者必须保证 `arch` 是有效的 C 字符串指针
#[no_mangle]
pub unsafe extern "C" fn edition_get_dmg_name(arch: *const c_char) -> *mut c_char {
    let arch_str = CStr::from_ptr(arch).to_string_lossy().to_string();

    let info = VersionInfo::current();
    CString::new(info.dmg_name(&arch_str)).unwrap().into_raw()
}

/// 获取 Debian 包名
///
/// # Safety
///
/// 调用者必须保证 `arch` 是有效的 C 字符串指针
#[no_mangle]
pub unsafe extern "C" fn edition_get_deb_name(arch: *const c_char) -> *mut c_char {
    let arch_str = CStr::from_ptr(arch).to_string_lossy().to_string();

    let info = VersionInfo::current();
    CString::new(info.deb_name(&arch_str)).unwrap().into_raw()
}

/// 获取 RPM 包名
///
/// # Safety
///
/// 调用者必须保证 `arch` 是有效的 C 字符串指针
#[no_mangle]
pub unsafe extern "C" fn edition_get_rpm_name(arch: *const c_char) -> *mut c_char {
    let arch_str = CStr::from_ptr(arch).to_string_lossy().to_string();

    let info = VersionInfo::current();
    CString::new(info.rpm_name(&arch_str)).unwrap().into_raw()
}

/// 获取主色调（RGB格式，便于UI使用）
///
/// # Returns
///
/// RGB值打包为 0xRRGGBB
#[no_mangle]
pub extern "C" fn edition_get_primary_color_rgb() -> c_int {
    let info = VersionInfo::current();
    hex_to_rgb(&info.primary_color)
}

/// 获取次色调（RGB格式）
#[no_mangle]
pub extern "C" fn edition_get_secondary_color_rgb() -> c_int {
    let info = VersionInfo::current();
    hex_to_rgb(&info.secondary_color)
}

/// 获取强调色（RGB格式）
#[no_mangle]
pub extern "C" fn edition_get_accent_color_rgb() -> c_int {
    let info = VersionInfo::current();
    hex_to_rgb(&info.accent_color)
}

/// 将十六进制颜色转换为RGB整数
fn hex_to_rgb(hex: &str) -> c_int {
    if hex.len() != 7 || !hex.starts_with('#') {
        return 0;
    }

    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0) as c_int;
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0) as c_int;
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0) as c_int;

    (r << 16) | (g << 8) | b
}

/// 检查版本是否可以升级
///
/// # Arguments
///
/// * `from_edition` - 源版本（0=Lite, 1=Standard, 2=Pro）
/// * `to_edition` - 目标版本
///
/// # Returns
///
/// 1 表示可以升级，0 表示不可以
#[no_mangle]
pub extern "C" fn edition_can_upgrade(from_edition: c_int, to_edition: c_int) -> c_int {
    let from = edition_from_int(from_edition);
    let to = edition_from_int(to_edition);

    if to.can_upgrade_from(from) {
        1
    } else {
        0
    }
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

/// 获取所有版本类型信息
///
/// 返回JSON格式字符串，包含所有版本的详细信息
#[no_mangle]
pub extern "C" fn edition_get_all_editions_json() -> *mut c_char {
    let editions = [Edition::Lite, Edition::Standard, Edition::Pro];

    let editions_info: Vec<serde_json::Value> = editions
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.identifier(),
                "name": e.name(),
                "full_name": e.full_name(),
                "short_id": e.short_identifier(),
                "tier": e.tier(),
                "primary_color": e.primary_color(),
                "secondary_color": e.secondary_color(),
                "accent_color": e.accent_color(),
                "tagline": e.tagline(),
                "features": e.supported_features(),
            })
        })
        .collect();

    let json = serde_json::to_string(&editions_info).unwrap_or_default();
    CString::new(json).unwrap().into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_version_info_roundtrip() {
        let c_info = edition_get_version_info();
        assert!(!c_info.is_null());

        unsafe {
            let info = &*c_info;
            assert!(info.edition >= 0 && info.edition <= 2);
            assert!(info.build_type >= 0 && info.build_type <= 1);
            assert!(!info.version.is_null());
            assert!(!info.edition_name.is_null());
            assert!(info.feature_count >= 3); // 至少基础功能

            edition_free_version_info(c_info);
        }
    }

    #[test]
    fn test_c_app_identity_roundtrip() {
        let c_identity = edition_get_app_identity();
        assert!(!c_identity.is_null());

        unsafe {
            let identity = &*c_identity;
            assert!(!identity.app_name.is_null());
            assert!(!identity.bundle_id.is_null());
            assert!(!identity.vendor.is_null());

            edition_free_app_identity(c_identity);
        }
    }

    #[test]
    fn test_edition_get_current() {
        let edition = edition_get_current();
        assert!(edition >= 0 && edition <= 2);
    }

    #[test]
    fn test_hex_to_rgb() {
        assert_eq!(hex_to_rgb("#FF0000"), 0xFF0000);
        assert_eq!(hex_to_rgb("#00FF00"), 0x00FF00);
        assert_eq!(hex_to_rgb("#0000FF"), 0x0000FF);
        assert_eq!(hex_to_rgb("#10B981"), 0x10B981);
        assert_eq!(hex_to_rgb("invalid"), 0);
    }

    #[test]
    fn test_edition_can_upgrade() {
        assert_eq!(edition_can_upgrade(0, 1), 1); // Lite -> Standard
        assert_eq!(edition_can_upgrade(0, 2), 1); // Lite -> Pro
        assert_eq!(edition_can_upgrade(1, 2), 1); // Standard -> Pro
        assert_eq!(edition_can_upgrade(1, 0), 0); // Standard -> Lite (不可)
        assert_eq!(edition_can_upgrade(2, 0), 0); // Pro -> Lite (不可)
        assert_eq!(edition_can_upgrade(0, 0), 0); // Lite -> Lite (不可)
    }
}
