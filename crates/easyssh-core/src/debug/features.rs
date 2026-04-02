//! Debug功能特性定义
//!
//! 这是对 `crate::debug_access::DebugFeature` 的扩展

use crate::debug::can_access_feature;
pub use crate::debug_access::DebugFeature;

/// 获取功能显示名称
pub fn feature_display_name(feature: DebugFeature) -> &'static str {
    feature.name()
}

/// 获取功能描述
pub fn feature_description(feature: DebugFeature) -> &'static str {
    feature.description()
}

/// 功能分组
#[derive(Debug, Clone)]
pub struct FeatureGroup {
    pub name: &'static str,
    pub description: &'static str,
    pub features: Vec<DebugFeature>,
}

/// 获取功能分组列表
pub fn get_feature_groups() -> Vec<FeatureGroup> {
    use DebugFeature::*;

    vec![
        FeatureGroup {
            name: "System",
            description: "System diagnostics and monitoring",
            features: vec![NetworkInspector, LogViewer, PerformanceMonitor],
        },
        FeatureGroup {
            name: "Development",
            description: "Development and debugging tools",
            features: vec![AiProgramming, TestRunner, FeatureFlags, DebugWebSocket],
        },
        FeatureGroup {
            name: "Enterprise",
            description: "Enterprise and team features",
            features: vec![
                AuditLogViewer,
                DatabaseConsole,
                InternalStateInspector,
                MemoryProfiler,
                PacketCapture,
            ],
        },
    ]
}

/// 获取当前可用功能列表
pub fn get_available_features() -> Vec<DebugFeature> {
    use DebugFeature::*;

    let all = vec![
        AiProgramming,
        PerformanceMonitor,
        NetworkInspector,
        DatabaseConsole,
        LogViewer,
        TestRunner,
        FeatureFlags,
        AuditLogViewer,
        InternalStateInspector,
        MemoryProfiler,
        PacketCapture,
        DebugWebSocket,
    ];

    all.into_iter().filter(|f| can_access_feature(*f)).collect()
}

/// 运行时特性开关存储
pub mod feature_flags {
    use std::collections::HashMap;
    use std::sync::RwLock;

    lazy_static::lazy_static! {
        static ref FLAGS: RwLock<HashMap<String, bool>> = RwLock::new(HashMap::new());
    }

    /// 设置功能开关
    pub fn set(name: &str, enabled: bool) {
        if let Ok(mut flags) = FLAGS.write() {
            flags.insert(name.to_string(), enabled);
        }
    }

    /// 获取功能开关状态
    pub fn get(name: &str) -> bool {
        FLAGS
            .read()
            .ok()
            .and_then(|flags| flags.get(name).copied())
            .unwrap_or(false)
    }

    /// 检查功能开关是否存在
    pub fn exists(name: &str) -> bool {
        FLAGS
            .read()
            .ok()
            .map(|flags| flags.contains_key(name))
            .unwrap_or(false)
    }

    /// 列出所有功能开关
    pub fn list_all() -> HashMap<String, bool> {
        FLAGS.read().ok().map(|f| f.clone()).unwrap_or_default()
    }

    /// 清除所有功能开关
    pub fn clear() {
        if let Ok(mut flags) = FLAGS.write() {
            flags.clear();
        }
    }

    /// 切换功能开关状态
    pub fn toggle(name: &str) -> bool {
        let current = get(name);
        set(name, !current);
        !current
    }
}

/// 功能开关信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeatureFlagInfo {
    pub name: String,
    pub enabled: bool,
    pub description: String,
    pub modified_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::DebugFeature;

    #[test]
    fn test_feature_names() {
        assert_eq!(DebugFeature::AiProgramming.name(), "AI编程接口");
        assert_eq!(DebugFeature::LogViewer.name(), "日志查看器");
    }

    #[test]
    fn test_get_feature_groups() {
        let groups = get_feature_groups();
        assert!(!groups.is_empty());

        // 确保有System组
        assert!(groups.iter().any(|g| g.name == "System"));
    }

    #[test]
    fn test_feature_flags() {
        feature_flags::set("test_feature", true);
        assert!(feature_flags::get("test_feature"));
        assert!(feature_flags::exists("test_feature"));

        feature_flags::set("test_feature", false);
        assert!(!feature_flags::get("test_feature"));

        let toggled = feature_flags::toggle("test_feature");
        assert!(toggled);
        assert!(feature_flags::get("test_feature"));

        feature_flags::clear();
        assert!(!feature_flags::exists("test_feature"));
    }
}
