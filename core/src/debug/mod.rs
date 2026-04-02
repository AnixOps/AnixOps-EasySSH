//! 统一Debug模块
//!
//! 为Lite/Standard/Pro三版本提供一致的Debug核心功能。
//! 这是对 `debug_access` 模块的重新导出和扩展。
//!
//! # 功能矩阵
//!
//! | 功能 | Lite | Standard | Pro |
//! |------|------|----------|-----|
//! | AI编程接口 | 基础 | 完整 | 完整 |
//! | 性能监控 | 基础 | 完整 | 完整 |
//! | 网络检查 | ✅ | ✅ | ✅ |
//! | 数据库控制台 | ❌ | ✅ | ✅ |
//! | 日志查看器 | 基础 | 完整 | 完整 |
//! | 测试运行器 | ❌ | ✅ | ✅ |
//! | 特性开关 | ❌ | ✅ | ✅ |
//!
//! # 访问控制
//!
//! Debug功能需要通过隐藏入口或快捷键激活：
//! - Lite: Ctrl+Shift+D 连续按3次
//! - Standard: Ctrl+Alt+Shift+D
//! - Pro: 管理后台开关或API调用

// 重新导出 debug_access 的核心类型
pub use crate::debug_access::{
    DebugAccess, DebugAccessError, DebugAccessLevel, DebugAccessMethod,
    DebugAuditAction, DebugAuditRecord, DebugAuditResult, DebugClientInfo,
    DebugFeature, DebugSession, EditionActivationConfig, KeySequenceDetector,
};

// 子模块
pub mod access;
pub mod ai_integration;
pub mod commands;
pub mod features;
pub mod network;
pub mod performance;
pub mod types;

// 条件编译子模块
#[cfg(feature = "database-client")]
pub mod database_console;
pub mod logging;

// 重新导出核心类型，保持向后兼容
pub use types::*;

/// Debug模块全局启用状态
pub fn is_debug_enabled() -> bool {
    crate::debug_access::is_debug_enabled()
}

/// 初始化Debug模块
///
/// # Example
/// ```
/// use easyssh_core::debug;
///
/// // 通过版本号自动检测
/// let edition = easyssh_core::edition::Edition::current();
/// debug::init_debug(edition);
/// ```
pub fn init_debug(edition: crate::edition::Edition) {
    crate::debug_access::init_global_debug_access();
    log::info!("Debug module initialized for edition: {:?}", edition);
}

/// 启用Debug功能（通过隐藏入口）
///
/// 这个函数可以被UI层通过快捷键或隐藏菜单调用
///
/// # Example
/// ```
/// use easyssh_core::debug;
/// use easyssh_core::edition::Edition;
///
/// debug::enable_debug_via_hidden_entry(Edition::Standard);
/// ```
pub fn enable_debug_via_hidden_entry(edition: crate::edition::Edition) {
    crate::debug_access::init_global_debug_access();

    let method = match edition {
        crate::edition::Edition::Lite => DebugAccessMethod::KeyCombination {
            sequence: "ctrl+shift+d x3".to_string(),
        },
        crate::edition::Edition::Standard => DebugAccessMethod::KeyCombination {
            sequence: "ctrl+alt+shift+d".to_string(),
        },
        crate::edition::Edition::Pro => DebugAccessMethod::AdminSwitch {
            admin_id: "admin".to_string(),
        },
    };

    if let Some(access) = crate::debug_access::get_debug_access() {
        let client_info = DebugClientInfo {
            platform: Some(std::env::consts::OS.to_string()),
            ..Default::default()
        };
        let _ = access.activate(method, None, client_info);
    }
}

/// 禁用Debug功能
pub fn disable_debug() {
    if let Some(access) = crate::debug_access::get_debug_access() {
        let _ = access.deactivate("manual");
    }
}

/// 执行 health check（所有版本可用）
pub fn health_check() -> types::HealthStatus {
    let debug_enabled = is_debug_enabled();
    let level = if let Some(access) = crate::debug_access::get_debug_access() {
        access
            .get_access_level()
            .map(|l| format!("{:?}", l))
            .unwrap_or_else(|| "Disabled".to_string())
    } else {
        "Disabled".to_string()
    };

    types::HealthStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        debug_enabled,
        access_level: level,
    }
}

/// 获取Debug功能清单
pub fn get_debug_capabilities() -> types::DebugCapabilities {
    if let Some(access) = crate::debug_access::get_debug_access() {
        types::DebugCapabilities {
            ai_programming: access.can_access_feature(DebugFeature::AiProgramming),
            performance_monitoring: access.can_access_feature(DebugFeature::PerformanceMonitor),
            network_check: access.can_access_feature(DebugFeature::NetworkInspector),
            database_console: access.can_access_feature(DebugFeature::DatabaseConsole),
            log_viewer: access.can_access_feature(DebugFeature::LogViewer),
            test_runner: access.can_access_feature(DebugFeature::TestRunner),
            feature_flags: access.can_access_feature(DebugFeature::FeatureFlags),
            audit_logs: access.can_access_feature(DebugFeature::AuditLogViewer),
        }
    } else {
        // Debug未初始化时的默认清单
        types::DebugCapabilities {
            ai_programming: false,
            performance_monitoring: false,
            network_check: true, // 网络检查是基础功能
            database_console: false,
            log_viewer: true, // 基础日志查看
            test_runner: false,
            feature_flags: false,
            audit_logs: false,
        }
    }
}

/// 获取当前访问级别
pub fn get_access_level() -> Option<DebugAccessLevel> {
    crate::debug_access::get_debug_access()
        .and_then(|a| a.get_access_level())
}

/// 检查是否有权限访问特定功能
pub fn can_access_feature(feature: DebugFeature) -> bool {
    crate::debug_access::get_debug_access()
        .map(|a| a.can_access_feature(feature))
        .unwrap_or(false)
}

/// 尝试从环境变量激活Debug模式
pub fn try_activate_from_env() -> Result<DebugSession, DebugAccessError> {
    crate::debug_access::try_activate_from_env()
}

/// 尝试从CLI参数激活Debug模式
pub fn try_activate_from_cli(args: &[String]) -> Result<DebugSession, DebugAccessError> {
    crate::debug_access::try_activate_from_cli(args)
}

/// 创建Lite版本组合键检测器
pub fn create_lite_key_detector() -> KeySequenceDetector {
    crate::debug_access::create_lite_key_detector()
}

/// 创建Standard版本组合键检测器
pub fn create_standard_key_detector() -> KeySequenceDetector {
    crate::debug_access::create_standard_key_detector()
}

/// Debug功能清单
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugCapabilitiesSummary {
    pub edition: String,
    pub debug_enabled: bool,
    pub access_level: Option<String>,
    pub available_features: Vec<String>,
    pub restricted_features: Vec<String>,
}

/// 获取能力摘要
pub fn get_capabilities_summary() -> DebugCapabilitiesSummary {
    let edition = format!("{:?}", crate::edition::Edition::current());
    let debug_enabled = is_debug_enabled();
    let access_level = get_access_level().map(|l| format!("{:?}", l));

    let all_features = vec![
        DebugFeature::AiProgramming,
        DebugFeature::PerformanceMonitor,
        DebugFeature::NetworkInspector,
        DebugFeature::DatabaseConsole,
        DebugFeature::LogViewer,
        DebugFeature::TestRunner,
        DebugFeature::FeatureFlags,
        DebugFeature::AuditLogViewer,
    ];

    let mut available = Vec::new();
    let mut restricted = Vec::new();

    for feature in all_features {
        let name = feature.name().to_string();
        if can_access_feature(feature) {
            available.push(name);
        } else {
            restricted.push(name);
        }
    }

    DebugCapabilitiesSummary {
        edition,
        debug_enabled,
        access_level,
        available_features: available,
        restricted_features: restricted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check() {
        let status = health_check();
        assert!(!status.version.is_empty());
        assert_eq!(status.status, "ok");
    }

    #[test]
    fn test_capabilities_summary() {
        let summary = get_capabilities_summary();
        assert!(!summary.edition.is_empty());
        // 版本应该是 Lite, Standard, 或 Pro 之一
        assert!(
            summary.edition.contains("Lite")
                || summary.edition.contains("Standard")
                || summary.edition.contains("Pro")
        );
    }

    #[test]
    fn test_key_detectors() {
        let lite_detector = create_lite_key_detector();
        let (current, total) = lite_detector.get_progress();
        assert_eq!(current, 0);
        assert_eq!(total, 3); // Lite需要3次

        let std_detector = create_standard_key_detector();
        let (current, total) = std_detector.get_progress();
        assert_eq!(current, 0);
        assert_eq!(total, 1); // Standard只需要1次
    }
}
