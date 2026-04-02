//! 版本标识系统扩展
//!
//! 为EasySSH Lite/Standard/Pro三版本提供额外的版本标识功能。
//! 此模块与 `edition` 模块配合使用，补充构建时信息和平台信息。
//!
//! # 与 edition 模块的关系
//!
//! - `edition` 模块：核心版本类型定义、颜色、功能检查
//! - `version` 模块：构建信息、平台信息、兼容性分析、FFI扩展
//!
//! # 使用示例
//!
//! ```rust
//! use easyssh_core::edition::{Edition, VersionInfo};
//! use easyssh_core::version::{PlatformInfo, VersionCompatibility, FullBuildInfo};
//!
//! // 获取基本版本信息（来自 edition 模块）
//! let info = VersionInfo::current();
//!
//! // 获取完整构建信息（包含平台信息）
//! let build_info = FullBuildInfo::current();
//! println!("Platform: {}", build_info.platform.display());
//!
//! // 版本兼容性检查
//! let can_upgrade = VersionCompatibility::is_compatible(Edition::Lite, Edition::Standard);
//! ```

use crate::edition::{BuildType, Edition, VersionInfo};
use std::fmt;
use std::sync::OnceLock;

/// 平台信息
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PlatformInfo {
    /// 操作系统名称
    pub os: String,
    /// CPU架构
    pub arch: String,
    /// 操作系统家族
    pub family: String,
}

impl PlatformInfo {
    /// 获取当前平台信息
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            family: std::env::consts::FAMILY.to_string(),
        }
    }

    /// 格式化显示
    ///
    /// 格式: "os-arch"
    /// 示例: "windows-x86_64", "macos-aarch64", "linux-x86_64"
    pub fn display(&self) -> String {
        format!("{}-{}", self.os, self.arch)
    }

    /// 是否为Windows
    pub fn is_windows(&self) -> bool {
        self.os == "windows"
    }

    /// 是否为macOS
    pub fn is_macos(&self) -> bool {
        self.os == "macos"
    }

    /// 是否为Linux
    pub fn is_linux(&self) -> bool {
        self.os == "linux"
    }

    /// 是否为64位系统
    pub fn is_64bit(&self) -> bool {
        self.arch.contains("64")
    }

    /// 是否为ARM架构
    pub fn is_arm(&self) -> bool {
        self.arch.contains("aarch") || self.arch.contains("arm")
    }

    /// 获取User-Agent后缀
    pub fn user_agent_suffix(&self) -> String {
        format!("{}; {}", self.os, self.arch)
    }
}

impl fmt::Display for PlatformInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl Default for PlatformInfo {
    fn default() -> Self {
        Self::current()
    }
}

/// 完整构建信息
///
/// 包含版本、构建类型、平台信息等完整的构建时信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FullBuildInfo {
    /// 版本信息（来自 edition 模块）
    #[serde(flatten)]
    pub version_info: VersionInfo,
    /// Git分支
    pub git_branch: Option<String>,
    /// 构建日期
    pub build_date: String,
    /// Rust编译器版本
    pub rustc_version: Option<String>,
    /// 平台信息
    pub platform: PlatformInfo,
    /// User-Agent字符串
    pub user_agent: String,
    /// 版本标识符（用于API和日志）
    pub version_id: String,
}

impl FullBuildInfo {
    /// 获取当前完整构建信息
    ///
    /// 使用OnceLock确保只初始化一次
    pub fn current() -> &'static Self {
        static INFO: OnceLock<FullBuildInfo> = OnceLock::new();
        INFO.get_or_init(|| Self::collect())
    }

    /// 收集构建信息
    fn collect() -> Self {
        let version_info = VersionInfo::current();
        let platform = PlatformInfo::current();

        let user_agent = format!(
            "EasySSH/{}/{} ({}; {})",
            version_info.version,
            version_info.edition.identifier(),
            platform.display(),
            version_info.build_type.name().to_lowercase()
        );

        let version_id = format!(
            "{}/{}/{}",
            version_info.edition.identifier(),
            version_info.version,
            version_info.build_type.name().to_lowercase()
        );

        Self {
            version_info,
            git_branch: option_env!("EASYSSH_GIT_BRANCH").map(|s| s.to_string()),
            build_date: option_env!("EASYSSH_BUILD_DATE")
                .unwrap_or("unknown")
                .to_string(),
            rustc_version: option_env!("EASYSSH_RUSTC_VERSION").map(|s| s.to_string()),
            platform,
            user_agent,
            version_id,
        }
    }

    /// 格式化详细版本显示
    ///
    /// 示例:
    /// ```
    /// EasySSH Lite 0.3.0 [Dev]
    /// Build: 2024-01-15
    /// Git: abc1234 (main)
    /// Platform: windows-x86_64
    /// ```
    pub fn display_verbose(&self) -> String {
        let mut lines = vec![self.version_info.full_version_string()];

        lines.push(format!("Build Date: {}", self.build_date));

        if let Some(ref branch) = self.git_branch {
            lines.push(format!("Git Branch: {}", branch));
        }

        lines.push(format!("Platform: {}", self.platform.display()));

        if let Some(ref rustc) = self.rustc_version {
            lines.push(format!("Rustc: {}", rustc));
        }

        lines.join("\n")
    }

    /// 序列化为JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 获取版本摘要（用于日志和错误报告）
    pub fn summary(&self) -> String {
        format!(
            "{} {} ({}; {}; {})",
            self.version_info.edition.name(),
            self.version_info.version,
            self.build_date,
            self.platform.display(),
            self.version_info.build_type.name()
        )
    }
}

impl fmt::Display for FullBuildInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version_info.window_title())
    }
}

/// 版本兼容性检查
///
/// 用于检查版本之间的兼容性和升级路径
pub struct VersionCompatibility;

impl VersionCompatibility {
    /// 检查两个版本是否兼容（数据层面）
    ///
    /// Lite <-> Standard: 兼容（Standard包含Lite所有功能）
    /// Lite <-> Pro: 兼容（Pro包含Lite所有功能）
    /// Standard <-> Pro: 部分兼容
    ///
    /// # 示例
    ///
    /// ```rust
    /// use easyssh_core::version::VersionCompatibility;
    /// use easyssh_core::edition::Edition;
    ///
    /// // Lite可以升级到Standard
    /// assert!(VersionCompatibility::is_compatible(Edition::Lite, Edition::Standard));
    ///
    /// // Pro降级到Standard：数据可能丢失
    /// assert!(!VersionCompatibility::is_compatible(Edition::Pro, Edition::Standard));
    /// ```
    pub fn is_compatible(from: Edition, to: Edition) -> bool {
        match (from, to) {
            // 相同版本完全兼容
            (Edition::Lite, Edition::Lite) => true,
            (Edition::Standard, Edition::Standard) => true,
            (Edition::Pro, Edition::Pro) => true,
            // Lite可以升级到任何版本
            (Edition::Lite, _) => true,
            // 任何版本都可以降级到Lite（丢失功能）
            (_, Edition::Lite) => true,
            // Standard可以升级到Pro
            (Edition::Standard, Edition::Pro) => true,
            // Pro降级到Standard：部分兼容（团队协作数据可能丢失）
            (Edition::Pro, Edition::Standard) => false,
        }
    }

    /// 获取版本迁移建议
    ///
    /// 返回针对特定版本迁移的提示信息
    pub fn migration_advice(from: Edition, to: Edition) -> &'static str {
        match (from, to) {
            (Edition::Lite, Edition::Lite) => "无需迁移",
            (Edition::Standard, Edition::Standard) => "无需迁移",
            (Edition::Pro, Edition::Pro) => "无需迁移",
            (Edition::Lite, Edition::Standard) => "平滑升级：所有配置自动保留，新增功能默认可用",
            (Edition::Lite, Edition::Pro) => "平滑升级：所有配置自动保留，建议配置团队权限",
            (Edition::Standard, Edition::Pro) => "平滑升级：现有配置保留，建议启用审计日志",
            (Edition::Standard, Edition::Lite) => "降级注意：分屏布局、监控数据将不可用",
            (Edition::Pro, Edition::Lite) => "降级警告：团队数据、审计日志将丢失！建议先备份",
            (Edition::Pro, Edition::Standard) => {
                "降级警告：团队协作功能将不可用，建议先导出团队数据"
            }
        }
    }

    /// 检查升级路径
    ///
    /// 返回升级步骤列表
    pub fn upgrade_path(from: Edition, to: Edition) -> Vec<String> {
        if from.tier() >= to.tier() {
            return vec![];
        }

        match (from, to) {
            (Edition::Lite, Edition::Standard) => vec![
                "升级应用".to_string(),
                "重新打开现有连接".to_string(),
                "可选：配置分屏布局".to_string(),
            ],
            (Edition::Lite, Edition::Pro) => vec![
                "升级应用".to_string(),
                "创建或加入团队".to_string(),
                "配置团队权限".to_string(),
                "可选：启用审计日志".to_string(),
                "可选：配置SSO".to_string(),
            ],
            (Edition::Standard, Edition::Pro) => vec![
                "升级应用".to_string(),
                "创建或加入团队".to_string(),
                "配置团队权限".to_string(),
                "可选：启用审计日志".to_string(),
            ],
            _ => vec!["直接升级".to_string()],
        }
    }
}

/// 功能检查宏 - 编译时优化
///
/// 这些宏在编译时展开，用于条件编译
#[macro_export]
macro_rules! check_feature {
    (embedded_terminal) => {
        cfg!(feature = "embedded-terminal")
    };
    (split_screen) => {
        cfg!(feature = "split-screen")
    };
    (sftp) => {
        cfg!(feature = "sftp")
    };
    (monitoring) => {
        cfg!(feature = "monitoring")
    };
    (team) => {
        cfg!(feature = "team")
    };
    (audit) => {
        cfg!(feature = "audit")
    };
    (sso) => {
        cfg!(feature = "sso")
    };
    (remote_desktop) => {
        cfg!(feature = "remote-desktop")
    };
    (log_monitor) => {
        cfg!(feature = "log-monitor")
    };
    (docker) => {
        cfg!(feature = "docker")
    };
    (kubernetes) => {
        cfg!(feature = "kubernetes")
    };
    (database_client) => {
        cfg!(feature = "database-client")
    };
    (workflow) => {
        cfg!(feature = "workflow")
    };
    (auto_update) => {
        cfg!(feature = "auto-update")
    };
    (backup) => {
        cfg!(feature = "backup")
    };
    (sync) => {
        cfg!(feature = "sync")
    };
    (port_forwarding) => {
        cfg!(feature = "port-forwarding")
    };
    (telemetry) => {
        cfg!(feature = "telemetry")
    };
    (git) => {
        cfg!(feature = "git")
    };
    (dev_tools) => {
        cfg!(feature = "dev-tools")
    };
    (ssh) => {
        // ssh是基础功能，始终返回true
        true
    };
}

/// 版本检查宏 - 运行时检查
///
/// 在运行时检查当前版本是否满足最低要求
#[macro_export]
macro_rules! require_edition {
    ($min_edition:expr) => {
        if !$crate::edition::Edition::current().meets_requirement($min_edition) {
            return Err($crate::error::LiteError::Config(format!(
                "此功能需要 {} 版本或更高版本",
                $min_edition.name()
            )));
        }
    };
}

/// 功能要求宏 - 运行时检查
///
/// 检查特定功能在当前版本中是否可用
#[macro_export]
macro_rules! require_feature {
    ($feature:ident, $edition:expr) => {
        if !$edition.$feature() {
            return Err($crate::error::LiteError::Config(format!(
                "此功能需要 {} 版本",
                $edition.name()
            )));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edition::VersionInfo;

    #[test]
    fn test_platform_info() {
        let platform = PlatformInfo::current();

        assert!(!platform.os.is_empty());
        assert!(!platform.arch.is_empty());
        assert!(!platform.family.is_empty());

        // 只能是其中一个
        let is_one_of = platform.is_windows() || platform.is_macos() || platform.is_linux();
        assert!(is_one_of, "Platform should be Windows, macOS, or Linux");
    }

    #[test]
    fn test_platform_display() {
        let platform = PlatformInfo::current();
        let display = platform.display();

        assert!(display.contains(&platform.os));
        assert!(display.contains(&platform.arch));
    }

    #[test]
    fn test_full_build_info() {
        let info = FullBuildInfo::current();

        // 版本信息必须有效
        assert!(!info.version_info.version.is_empty());
        assert!(!info.build_date.is_empty());
        assert!(!info.user_agent.is_empty());
        assert!(!info.version_id.is_empty());

        // User-Agent格式检查
        assert!(info.user_agent.starts_with("EasySSH/"));
    }

    #[test]
    fn test_full_build_info_verbose() {
        let info = FullBuildInfo::current();
        let verbose = info.display_verbose();

        assert!(verbose.contains(&info.version_info.version));
        assert!(verbose.contains(&info.build_date));
    }

    #[test]
    fn test_full_build_info_summary() {
        let info = FullBuildInfo::current();
        let summary = info.summary();

        assert!(summary.contains(&info.version_info.version));
        assert!(summary.contains(&info.version_info.edition.name()));
    }

    #[test]
    fn test_version_compatibility() {
        // 相同版本兼容
        assert!(VersionCompatibility::is_compatible(
            Edition::Lite,
            Edition::Lite
        ));
        assert!(VersionCompatibility::is_compatible(
            Edition::Standard,
            Edition::Standard
        ));
        assert!(VersionCompatibility::is_compatible(
            Edition::Pro,
            Edition::Pro
        ));

        // Lite可以升级到任何版本
        assert!(VersionCompatibility::is_compatible(
            Edition::Lite,
            Edition::Standard
        ));
        assert!(VersionCompatibility::is_compatible(
            Edition::Lite,
            Edition::Pro
        ));

        // 任何版本都可以降级到Lite
        assert!(VersionCompatibility::is_compatible(
            Edition::Standard,
            Edition::Lite
        ));
        assert!(VersionCompatibility::is_compatible(
            Edition::Pro,
            Edition::Lite
        ));

        // Standard可以升级到Pro
        assert!(VersionCompatibility::is_compatible(
            Edition::Standard,
            Edition::Pro
        ));

        // Pro降级到Standard不兼容
        assert!(!VersionCompatibility::is_compatible(
            Edition::Pro,
            Edition::Standard
        ));
    }

    #[test]
    fn test_version_compatibility_advice() {
        // 验证每个组合都有建议
        let advice = VersionCompatibility::migration_advice(Edition::Lite, Edition::Standard);
        assert!(!advice.is_empty());

        let advice = VersionCompatibility::migration_advice(Edition::Pro, Edition::Lite);
        assert!(!advice.is_empty());
    }

    #[test]
    fn test_upgrade_path() {
        // Lite -> Standard
        let path = VersionCompatibility::upgrade_path(Edition::Lite, Edition::Standard);
        assert!(!path.is_empty());

        // Lite -> Pro
        let path = VersionCompatibility::upgrade_path(Edition::Lite, Edition::Pro);
        assert!(!path.is_empty());

        // Standard -> Pro
        let path = VersionCompatibility::upgrade_path(Edition::Standard, Edition::Pro);
        assert!(!path.is_empty());

        // 同版本无升级路径
        let path = VersionCompatibility::upgrade_path(Edition::Lite, Edition::Lite);
        assert!(path.is_empty());

        // 降级无升级路径
        let path = VersionCompatibility::upgrade_path(Edition::Pro, Edition::Lite);
        assert!(path.is_empty());
    }

    #[test]
    fn test_check_feature_macro() {
        // 验证宏可以编译并返回布尔值
        let _embedded = check_feature!(embedded_terminal);
        let _sftp = check_feature!(sftp);
        let _split = check_feature!(split_screen);

        // 宏应该在编译时展开为布尔常量
        const _: bool = check_feature!(ssh); // ssh特性不存在，应该返回false
    }

    #[test]
    fn test_full_build_info_to_json() {
        let info = FullBuildInfo::current();
        let json = info.to_json().expect("Failed to serialize to JSON");

        assert!(json.contains("version_info"));
        assert!(json.contains("platform"));
        assert!(json.contains("user_agent"));
    }

    #[test]
    fn test_version_info_current() {
        // 验证 edition 模块的 VersionInfo 仍然可用
        let info = VersionInfo::current();
        assert!(!info.version.is_empty());
        assert!(!info.edition_name.is_empty());
    }
}
