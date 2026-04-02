//! Debug访问控制包装模块
//!
//! 这是对 `crate::debug_access` 的包装，提供统一接口

pub use crate::debug_access::{
    DebugAccessLevel, DebugAccessError, DebugAccessMethod, DebugFeature, DebugClientInfo
};

use std::sync::atomic::{AtomicU8, Ordering};

// 内部访问级别存储（与debug_access同步）
static ACCESS_LEVEL: AtomicU8 = AtomicU8::new(0);

/// 设置Debug访问级别（包装函数）
pub fn set_access_level(level: DebugAccessLevel) {
    let value = match level {
        DebugAccessLevel::Viewer => 1,
        DebugAccessLevel::Developer => 2,
        DebugAccessLevel::Admin => 3,
    };
    ACCESS_LEVEL.store(value, Ordering::SeqCst);
}

/// 获取当前Debug访问级别（包装函数）
pub fn get_access_level() -> DebugAccessLevel {
    // 优先从debug_access获取
    if let Some(level) = crate::debug_access::get_debug_access()
        .and_then(|a| a.get_access_level()) {
        return level;
    }

    // 否则从本地存储获取
    match ACCESS_LEVEL.load(Ordering::SeqCst) {
        1 => DebugAccessLevel::Viewer,
        2 => DebugAccessLevel::Developer,
        3 => DebugAccessLevel::Admin,
        _ => DebugAccessLevel::Viewer,
    }
}

/// 检查当前访问级别是否满足要求
pub fn check_access(required: DebugAccessLevel) -> bool {
    let current = get_access_level() as u8;
    let required = required as u8;
    current >= required
}

/// 检查是否启用了Debug
pub fn is_enabled() -> bool {
    crate::debug_access::is_debug_enabled()
}

/// 访问控制错误类型
pub type AccessError = DebugAccessError;

/// 快捷键序列检测器
///
/// 用于通过快捷键组合启用Debug功能
pub struct ShortcutDetector {
    detector: crate::debug_access::KeySequenceDetector,
}

impl ShortcutDetector {
    /// 创建Lite版本检测器 (Ctrl+Shift+D 3次)
    pub fn lite() -> Self {
        Self {
            detector: crate::debug_access::create_lite_key_detector(),
        }
    }

    /// 创建Standard版本检测器 (Ctrl+Alt+Shift+D)
    pub fn standard() -> Self {
        Self {
            detector: crate::debug_access::create_standard_key_detector(),
        }
    }

    /// 处理按键事件
    ///
    /// # Arguments
    /// * `key` - 按键字符串，如 "ctrl+shift+d"
    ///
    /// # Returns
    /// * `true` - 组合键序列完成
    /// * `false` - 序列未完成或已重置
    pub fn record_press(&mut self, key: &str) -> bool {
        self.detector.on_key(key)
    }

    /// 获取当前进度
    pub fn get_progress(&self) -> (usize, usize) {
        self.detector.get_progress()
    }

    /// 重置检测器
    pub fn reset(&mut self) {
        self.detector.reset();
    }
}

/// 验证密码（占位实现）
///
/// 在实际应用中应该使用更安全的方式
pub fn verify_password(password: &str, edition: crate::edition::Edition) -> bool {
    // 密码格式: easyssh_debug_<edition>
    let expected = format!("easyssh_debug_{:?}", edition).to_lowercase();

    if password == expected {
        let level = match edition {
            crate::edition::Edition::Lite => DebugAccessLevel::Developer,
            crate::edition::Edition::Standard => DebugAccessLevel::Developer,
            crate::edition::Edition::Pro => DebugAccessLevel::Admin,
        };
        set_access_level(level);
        log::info!("Debug access enabled via password for {:?}", edition);
        true
    } else {
        log::warn!("Failed debug password attempt");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_level() {
        set_access_level(DebugAccessLevel::Viewer);
        assert_eq!(get_access_level(), DebugAccessLevel::Viewer);

        set_access_level(DebugAccessLevel::Developer);
        assert_eq!(get_access_level(), DebugAccessLevel::Developer);

        set_access_level(DebugAccessLevel::Admin);
        assert_eq!(get_access_level(), DebugAccessLevel::Admin);
    }

    #[test]
    fn test_check_access() {
        set_access_level(DebugAccessLevel::Developer);

        assert!(check_access(DebugAccessLevel::Viewer));
        assert!(check_access(DebugAccessLevel::Developer));
        assert!(!check_access(DebugAccessLevel::Admin));
    }

    #[test]
    fn test_shortcut_detector() {
        let mut detector = ShortcutDetector::lite();

        assert!(!detector.record_press("ctrl+shift+d"));
        assert!(!detector.record_press("ctrl+shift+d"));
        assert!(detector.record_press("ctrl+shift+d")); // 第三次完成

        // 测试进度
        let (current, total) = detector.get_progress();
        assert_eq!(current, 0); // 完成后重置
        assert_eq!(total, 3);
    }

    #[test]
    fn test_password_verification() {
        assert!(verify_password("easyssh_debug_lite", crate::edition::Edition::Lite));
        assert!(!verify_password("wrong_password", crate::edition::Edition::Lite));
    }
}
