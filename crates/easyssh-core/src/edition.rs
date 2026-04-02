//! 版本标识系统
//!
//! 为EasySSH Lite/Standard/Pro三版本提供统一、清晰的版本标识规范。
//!
//! # 功能
//!
//! - 编译时版本确定（通过feature flags）
//! - 运行时版本检测
//! - 构建类型区分（Release/Dev）
//! - 视觉标识支持（颜色、徽章）
//! - FFI导出供各平台UI调用
//!
//! # 使用示例
//!
//! ```rust
//! use easyssh_core::edition::{Edition, VersionInfo, BuildType};
//!
//! // 获取当前版本信息
//! let info = VersionInfo::current();
//! println!("Edition: {}", info.edition_name);
//! println!("Version: {}", info.version);
//! println!("Build: {:?}", info.build_type);
//!
//! // 检查功能可用性
//! if info.edition.has_embedded_terminal() {
//!     // 使用嵌入式终端
//! }
//! ```

/// 版本类型枚举
///
/// 定义EasySSH的三个产品版本：Lite、Standard、Pro
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edition {
    /// Lite版 - 极简配置管理 + 原生终端唤起
    /// 核心价值：SSH配置保险箱
    Lite,
    /// Standard版 - 嵌入式终端 + 分屏 + 监控
    /// 核心价值：全功能客户端
    Standard,
    /// Pro版 - 团队协作 + 审计 + SSO
    /// 核心价值：企业级协作平台
    Pro,
}

impl Edition {
    /// 获取当前编译版本
    ///
    /// 通过feature flags在编译时确定版本，优先级：pro > standard > lite
    pub const fn current() -> Self {
        #[cfg(feature = "pro")]
        return Edition::Pro;
        #[cfg(all(feature = "standard", not(feature = "pro")))]
        return Edition::Standard;
        #[cfg(not(any(feature = "standard", feature = "pro")))]
        return Edition::Lite;
    }

    /// 版本显示名称
    pub const fn name(&self) -> &'static str {
        match self {
            Edition::Lite => "Lite",
            Edition::Standard => "Standard",
            Edition::Pro => "Pro",
        }
    }

    /// 版本完整名称（带EasySSH前缀）
    pub const fn full_name(&self) -> &'static str {
        match self {
            Edition::Lite => "EasySSH Lite",
            Edition::Standard => "EasySSH Standard",
            Edition::Pro => "EasySSH Pro",
        }
    }

    /// 版本标识符（用于文件名、目录名）
    pub const fn identifier(&self) -> &'static str {
        match self {
            Edition::Lite => "lite",
            Edition::Standard => "standard",
            Edition::Pro => "pro",
        }
    }

    /// 版本短标识符（用于UI显示）
    pub const fn short_identifier(&self) -> &'static str {
        match self {
            Edition::Lite => "L",
            Edition::Standard => "S",
            Edition::Pro => "P",
        }
    }

    /// 主色调（十六进制，用于UI主题）
    pub const fn primary_color(&self) -> &'static str {
        match self {
            // 清新绿 - 简洁、轻便
            Edition::Lite => "#10B981",
            // 科技蓝 - 专业、可靠
            Edition::Standard => "#3B82F6",
            // 尊贵紫 - 企业级、高级
            Edition::Pro => "#8B5CF6",
        }
    }

    /// 次色调（十六进制，用于渐变、高亮）
    pub const fn secondary_color(&self) -> &'static str {
        match self {
            Edition::Lite => "#34D399",
            Edition::Standard => "#60A5FA",
            Edition::Pro => "#A78BFA",
        }
    }

    /// 强调色（十六进制，用于徽章、标签）
    pub const fn accent_color(&self) -> &'static str {
        match self {
            Edition::Lite => "#059669",
            Edition::Standard => "#2563EB",
            Edition::Pro => "#7C3AED",
        }
    }

    /// 版本描述语
    pub const fn tagline(&self) -> &'static str {
        match self {
            Edition::Lite => "Secure SSH Config Manager",
            Edition::Standard => "Full-Featured SSH Client",
            Edition::Pro => "Enterprise Collaboration Platform",
        }
    }

    /// 版本层级（用于升级路径判断）
    pub const fn tier(&self) -> u8 {
        match self {
            Edition::Lite => 1,
            Edition::Standard => 2,
            Edition::Pro => 3,
        }
    }

    /// 是否支持从指定版本升级
    pub fn can_upgrade_from(&self, from: Edition) -> bool {
        self.tier() > from.tier()
    }

    /// 是否支持嵌入式终端
    pub const fn has_embedded_terminal(&self) -> bool {
        #[cfg(feature = "embedded-terminal")]
        return true;
        #[cfg(not(feature = "embedded-terminal"))]
        return false;
    }

    /// 是否支持分屏
    pub const fn has_split_screen(&self) -> bool {
        #[cfg(feature = "split-screen")]
        return true;
        #[cfg(not(feature = "split-screen"))]
        return false;
    }

    /// 是否支持SFTP
    pub const fn has_sftp(&self) -> bool {
        #[cfg(feature = "sftp")]
        return true;
        #[cfg(not(feature = "sftp"))]
        return false;
    }

    /// 是否支持监控
    pub const fn has_monitoring(&self) -> bool {
        #[cfg(feature = "monitoring")]
        return true;
        #[cfg(not(feature = "monitoring"))]
        return false;
    }

    /// 是否支持团队功能
    pub const fn has_team(&self) -> bool {
        #[cfg(feature = "team")]
        return true;
        #[cfg(not(feature = "team"))]
        return false;
    }

    /// 是否支持审计
    pub const fn has_audit(&self) -> bool {
        #[cfg(feature = "audit")]
        return true;
        #[cfg(not(feature = "audit"))]
        return false;
    }

    /// 是否支持SSO
    pub const fn has_sso(&self) -> bool {
        #[cfg(feature = "sso")]
        return true;
        #[cfg(not(feature = "sso"))]
        return false;
    }

    /// 是否支持同步
    pub const fn has_sync(&self) -> bool {
        #[cfg(feature = "sync")]
        return true;
        #[cfg(not(feature = "sync"))]
        return false;
    }

    /// 是否支持协作
    pub const fn has_collaboration(&self) -> bool {
        #[cfg(feature = "pro")]
        return true;
        #[cfg(not(feature = "pro"))]
        return false;
    }

    /// 是否支持Docker管理
    pub const fn has_docker(&self) -> bool {
        #[cfg(feature = "docker")]
        return true;
        #[cfg(not(feature = "docker"))]
        return false;
    }

    /// 是否支持Kubernetes
    pub const fn has_kubernetes(&self) -> bool {
        #[cfg(feature = "kubernetes")]
        return true;
        #[cfg(not(feature = "kubernetes"))]
        return false;
    }

    /// 是否支持工作流自动化
    pub const fn has_workflow(&self) -> bool {
        #[cfg(feature = "workflow")]
        return true;
        #[cfg(not(feature = "workflow"))]
        return false;
    }

    /// 获取所有支持的功能列表
    pub fn supported_features(&self) -> Vec<&'static str> {
        let mut features = vec!["ssh", "keychain", "native-terminal"];

        if self.has_embedded_terminal() {
            features.push("embedded-terminal");
        }
        if self.has_split_screen() {
            features.push("split-screen");
        }
        if self.has_sftp() {
            features.push("sftp");
        }
        if self.has_monitoring() {
            features.push("monitoring");
        }
        if self.has_team() {
            features.push("team");
        }
        if self.has_audit() {
            features.push("audit");
        }
        if self.has_sso() {
            features.push("sso");
        }
        if self.has_sync() {
            features.push("sync");
        }
        if self.has_collaboration() {
            features.push("collaboration");
        }
        if self.has_docker() {
            features.push("docker");
        }
        if self.has_kubernetes() {
            features.push("kubernetes");
        }
        if self.has_workflow() {
            features.push("workflow");
        }

        features
    }
}

/// 构建类型
///
/// 区分Release构建和开发者模式（Dev）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildType {
    /// 正式版本
    Release,
    /// 开发者模式（通过隐藏入口激活）
    Dev,
}

impl BuildType {
    /// 获取当前构建类型
    pub const fn current() -> Self {
        #[cfg(debug_assertions)]
        return BuildType::Dev;
        #[cfg(not(debug_assertions))]
        return BuildType::Release;
    }

    /// 构建类型名称
    pub const fn name(&self) -> &'static str {
        match self {
            BuildType::Release => "Release",
            BuildType::Dev => "Dev",
        }
    }

    /// 是否显示开发者工具
    pub const fn show_dev_tools(&self) -> bool {
        matches!(self, BuildType::Dev)
    }

    /// 是否启用详细日志
    pub const fn verbose_logging(&self) -> bool {
        matches!(self, BuildType::Dev)
    }
}

/// 完整版本信息
///
/// 包含版本、构建类型、功能列表等所有版本相关信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionInfo {
    /// 版本类型
    pub edition: Edition,
    /// 版本显示名称
    pub edition_name: String,
    /// 版本完整名称
    pub edition_full_name: String,
    /// 语义化版本号（如 "0.3.0"）
    pub version: String,
    /// 构建类型
    pub build_type: BuildType,
    /// 构建类型名称
    pub build_type_name: String,
    /// Git commit hash（如果可用）
    pub git_hash: Option<String>,
    /// 构建时间
    pub build_time: String,
    /// 启用的功能列表
    pub features: Vec<String>,
    /// 主色调
    pub primary_color: String,
    /// 次色调
    pub secondary_color: String,
    /// 强调色
    pub accent_color: String,
    /// 版本描述
    pub tagline: String,
}

impl VersionInfo {
    /// 获取当前版本信息
    pub fn current() -> Self {
        let edition = Edition::current();
        let build_type = BuildType::current();

        Self {
            edition_name: edition.name().to_string(),
            edition_full_name: edition.full_name().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_type_name: build_type.name().to_string(),
            git_hash: Self::git_hash(),
            build_time: Self::build_time(),
            features: edition
                .supported_features()
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            primary_color: edition.primary_color().to_string(),
            secondary_color: edition.secondary_color().to_string(),
            accent_color: edition.accent_color().to_string(),
            tagline: edition.tagline().to_string(),
            edition,
            build_type,
        }
    }

    /// 获取Git commit hash
    fn git_hash() -> Option<String> {
        option_env!("EASYSSH_GIT_HASH").map(|s| s.to_string())
    }

    /// 获取构建时间
    fn build_time() -> String {
        // 组合日期和时间
        let date = option_env!("EASYSSH_BUILD_DATE").unwrap_or("unknown");
        let time = option_env!("EASYSSH_BUILD_TIME").unwrap_or("unknown");
        format!("{} {}", date, time)
    }

    /// 获取Git分支
    pub fn git_branch() -> Option<String> {
        option_env!("EASYSSH_GIT_BRANCH").map(|s| s.to_string())
    }

    /// 获取Rust编译器版本
    pub fn rustc_version() -> Option<String> {
        option_env!("EASYSSH_RUSTC_VERSION").map(|s| s.to_string())
    }

    /// 窗口标题格式
    pub fn window_title(&self) -> String {
        if self.build_type == BuildType::Dev {
            format!(
                "{} {} [{}]",
                self.edition_full_name, self.version, self.build_type_name
            )
        } else {
            format!("{} {}", self.edition_full_name, self.version)
        }
    }

    /// 短版本标识（用于状态栏、托盘图标提示）
    pub fn short_version(&self) -> String {
        format!(
            "{} {} {}",
            self.edition.short_identifier(),
            self.version,
            self.build_type_name
        )
    }

    /// 完整版本字符串（用于日志、错误报告）
    pub fn full_version_string(&self) -> String {
        let git_info = self
            .git_hash
            .as_ref()
            .map(|h| format!(" (git: {})", &h[..8.min(h.len())]))
            .unwrap_or_default();

        format!(
            "{} {} {} (build: {}){}",
            self.edition_full_name, self.version, self.build_type_name, self.build_time, git_info
        )
    }

    /// 检查是否支持指定功能
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    /// 获取构建产物文件名
    ///
    /// 格式: easyssh-{edition}-{version}-{arch}.{ext}
    /// 示例: easyssh-lite-0.3.0-x64.exe
    pub fn build_artifact_name(&self, arch: &str, platform: &str) -> String {
        let edition_id = self.edition.identifier();

        match platform {
            "windows" => format!("easyssh-{}-{}-{}.exe", edition_id, self.version, arch),
            "macos" => format!("easyssh-{}-{}-{}.app", edition_id, self.version, arch),
            "linux" => format!("easyssh-{}-{}-{}.AppImage", edition_id, self.version, arch),
            _ => format!("easyssh-{}-{}-{}", edition_id, self.version, arch),
        }
    }

    /// 获取MSI安装包名
    pub fn msi_name(&self, arch: &str) -> String {
        format!(
            "easyssh-{}-{}-{}.msi",
            self.edition.identifier(),
            self.version,
            arch
        )
    }

    /// 获取DMG镜像名
    pub fn dmg_name(&self, arch: &str) -> String {
        format!(
            "easyssh-{}-{}-{}.dmg",
            self.edition.identifier(),
            self.version,
            arch
        )
    }

    /// 获取Debian包名
    pub fn deb_name(&self, arch: &str) -> String {
        format!(
            "easyssh-{}_{}_{}.deb",
            self.edition.identifier(),
            self.version,
            arch
        )
    }

    /// 获取RPM包名
    pub fn rpm_name(&self, arch: &str) -> String {
        format!(
            "easyssh-{}-{}-1.{}.rpm",
            self.edition.identifier(),
            self.version,
            arch
        )
    }

    /// 获取图标文件名
    pub fn icon_filename(&self) -> String {
        format!("icon-{}.png", self.edition.identifier())
    }
}

/// 应用标识信息（用于系统注册、协议处理）
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppIdentity {
    /// 应用名称
    pub app_name: String,
    /// 应用标识符（反向DNS格式）
    pub bundle_id: String,
    /// 厂商名称
    pub vendor: String,
    /// 版本信息
    pub version: VersionInfo,
    /// 数据目录名
    pub data_dir_name: String,
    /// 配置文件名
    pub config_filename: String,
}

impl AppIdentity {
    /// 创建当前应用标识
    pub fn current() -> Self {
        let version = VersionInfo::current();
        let edition_id = version.edition.identifier();

        Self {
            app_name: version.edition.full_name().to_string(),
            bundle_id: format!("com.anixops.easyssh.{}", edition_id),
            vendor: "AnixOps".to_string(),
            data_dir_name: format!("easyssh-{}", edition_id),
            config_filename: "config.json".to_string(),
            version,
        }
    }

    /// 获取用户数据目录路径
    pub fn data_dir(&self) -> std::path::PathBuf {
        dirs::data_dir()
            .map(|d| d.join(&self.data_dir_name))
            .unwrap_or_else(|| std::path::PathBuf::from(&self.data_dir_name))
    }

    /// 获取配置文件路径
    pub fn config_path(&self) -> std::path::PathBuf {
        self.data_dir().join(&self.config_filename)
    }
}

/// 版本比较结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionComparison {
    /// 版本相同
    Equal,
    /// 第一个版本较新
    Newer,
    /// 第一个版本较旧
    Older,
    /// 版本格式不兼容
    Incompatible,
}

/// 版本比较工具
pub struct VersionComparator;

impl VersionComparator {
    /// 比较两个语义化版本号
    pub fn compare(v1: &str, v2: &str) -> VersionComparison {
        let parse_version = |v: &str| -> Option<Vec<u32>> {
            v.split('.')
                .take(3)
                .map(|s| s.parse::<u32>().ok())
                .collect::<Option<Vec<_>>>()
        };

        match (parse_version(v1), parse_version(v2)) {
            (Some(a), Some(b)) => {
                for (x, y) in a.iter().zip(b.iter()) {
                    match x.cmp(y) {
                        std::cmp::Ordering::Greater => return VersionComparison::Newer,
                        std::cmp::Ordering::Less => return VersionComparison::Older,
                        std::cmp::Ordering::Equal => continue,
                    }
                }
                match a.len().cmp(&b.len()) {
                    std::cmp::Ordering::Greater => VersionComparison::Newer,
                    std::cmp::Ordering::Less => VersionComparison::Older,
                    std::cmp::Ordering::Equal => VersionComparison::Equal,
                }
            }
            _ => VersionComparison::Incompatible,
        }
    }

    /// 检查是否需要更新
    pub fn needs_update(current: &str, latest: &str) -> bool {
        matches!(
            Self::compare(current, latest),
            VersionComparison::Older | VersionComparison::Incompatible
        )
    }
}

/// 版本特定代码块
#[macro_export]
macro_rules! edition_match {
    (lite => $lite:expr, standard => $standard:expr, pro => $pro:expr) => {
        match $crate::edition::Edition::current() {
            $crate::edition::Edition::Lite => $lite,
            $crate::edition::Edition::Standard => $standard,
            $crate::edition::Edition::Pro => $pro,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check_feature;

    #[test]
    fn test_edition_variants() {
        assert_eq!(Edition::Lite.name(), "Lite");
        assert_eq!(Edition::Standard.name(), "Standard");
        assert_eq!(Edition::Pro.name(), "Pro");
    }

    #[test]
    fn test_edition_full_name() {
        assert_eq!(Edition::Lite.full_name(), "EasySSH Lite");
        assert_eq!(Edition::Standard.full_name(), "EasySSH Standard");
        assert_eq!(Edition::Pro.full_name(), "EasySSH Pro");
    }

    #[test]
    fn test_edition_identifier() {
        assert_eq!(Edition::Lite.identifier(), "lite");
        assert_eq!(Edition::Standard.identifier(), "standard");
        assert_eq!(Edition::Pro.identifier(), "pro");
    }

    #[test]
    fn test_edition_colors() {
        // Verify colors are valid hex format
        for edition in [Edition::Lite, Edition::Standard, Edition::Pro] {
            let primary = edition.primary_color();
            let secondary = edition.secondary_color();
            let accent = edition.accent_color();

            assert!(primary.starts_with('#'));
            assert!(secondary.starts_with('#'));
            assert!(accent.starts_with('#'));

            assert_eq!(primary.len(), 7);
            assert_eq!(secondary.len(), 7);
            assert_eq!(accent.len(), 7);
        }
    }

    #[test]
    fn test_edition_tier() {
        assert_eq!(Edition::Lite.tier(), 1);
        assert_eq!(Edition::Standard.tier(), 2);
        assert_eq!(Edition::Pro.tier(), 3);
    }

    #[test]
    fn test_edition_upgrade_path() {
        assert!(Edition::Standard.can_upgrade_from(Edition::Lite));
        assert!(Edition::Pro.can_upgrade_from(Edition::Lite));
        assert!(Edition::Pro.can_upgrade_from(Edition::Standard));

        assert!(!Edition::Lite.can_upgrade_from(Edition::Standard));
        assert!(!Edition::Lite.can_upgrade_from(Edition::Pro));
        assert!(!Edition::Standard.can_upgrade_from(Edition::Pro));
        assert!(!Edition::Lite.can_upgrade_from(Edition::Lite));
    }

    #[test]
    fn test_build_type() {
        let build_type = BuildType::current();
        // Just verify it returns a valid variant
        assert!(matches!(build_type, BuildType::Release | BuildType::Dev));
    }

    #[test]
    fn test_version_info_current() {
        let info = VersionInfo::current();

        assert!(!info.version.is_empty());
        assert!(!info.edition_name.is_empty());
        assert!(!info.edition_full_name.is_empty());
        assert!(!info.features.is_empty());

        // Base features should always be present
        assert!(info.has_feature("ssh"));
        assert!(info.has_feature("keychain"));
        assert!(info.has_feature("native-terminal"));
    }

    #[test]
    fn test_version_info_window_title() {
        let mut info = VersionInfo::current();

        // Release build
        info.build_type = BuildType::Release;
        let title = info.window_title();
        assert!(title.contains(&info.edition_full_name));
        assert!(title.contains(&info.version));
        assert!(!title.contains("Dev"));

        // Dev build
        info.build_type = BuildType::Dev;
        let title = info.window_title();
        assert!(title.contains("Dev"));
    }

    #[test]
    fn test_version_info_build_artifact_name() {
        let info = VersionInfo::current();

        let windows_exe = info.build_artifact_name("x64", "windows");
        assert!(windows_exe.contains("easyssh"));
        assert!(windows_exe.contains(info.edition.identifier()));
        assert!(windows_exe.ends_with(".exe"));

        let macos_app = info.build_artifact_name("arm64", "macos");
        assert!(macos_app.ends_with(".app"));

        let linux_app = info.build_artifact_name("x64", "linux");
        assert!(linux_app.ends_with(".AppImage"));
    }

    #[test]
    fn test_app_identity() {
        let identity = AppIdentity::current();

        assert!(!identity.app_name.is_empty());
        assert!(identity.bundle_id.starts_with("com.anixops.easyssh"));
        assert_eq!(identity.vendor, "AnixOps");
    }

    #[test]
    fn test_version_comparator() {
        use VersionComparison::*;

        assert_eq!(VersionComparator::compare("1.0.0", "1.0.0"), Equal);
        assert_eq!(VersionComparator::compare("1.0.1", "1.0.0"), Newer);
        assert_eq!(VersionComparator::compare("1.0.0", "1.0.1"), Older);
        assert_eq!(VersionComparator::compare("1.1.0", "1.0.0"), Newer);
        assert_eq!(VersionComparator::compare("2.0.0", "1.0.0"), Newer);
        assert_eq!(VersionComparator::compare("1.0", "1.0.0"), Equal);
        assert_eq!(VersionComparator::compare("invalid", "1.0.0"), Incompatible);
    }

    #[test]
    fn test_version_needs_update() {
        assert!(VersionComparator::needs_update("1.0.0", "1.0.1"));
        assert!(VersionComparator::needs_update("1.0.0", "2.0.0"));
        assert!(VersionComparator::needs_update("1.0.0", "invalid"));
        assert!(!VersionComparator::needs_update("1.0.0", "1.0.0"));
        assert!(!VersionComparator::needs_update("1.0.1", "1.0.0"));
    }

    #[test]
    fn test_edition_serialize() {
        let lite_json = serde_json::to_string(&Edition::Lite).unwrap();
        assert_eq!(lite_json, "\"lite\"");

        let standard_json = serde_json::to_string(&Edition::Standard).unwrap();
        assert_eq!(standard_json, "\"standard\"");

        let pro_json = serde_json::to_string(&Edition::Pro).unwrap();
        assert_eq!(pro_json, "\"pro\"");
    }

    #[test]
    fn test_edition_deserialize() {
        let lite: Edition = serde_json::from_str("\"lite\"").unwrap();
        assert_eq!(lite, Edition::Lite);

        let standard: Edition = serde_json::from_str("\"standard\"").unwrap();
        assert_eq!(standard, Edition::Standard);

        let pro: Edition = serde_json::from_str("\"pro\"").unwrap();
        assert_eq!(pro, Edition::Pro);
    }

    #[test]
    fn test_check_feature_macro() {
        // Verify the macro compiles and returns booleans
        let _embedded = check_feature!(embedded_terminal);
        let _sftp = check_feature!(sftp);
        let _split = check_feature!(split_screen);
    }

    #[test]
    fn test_edition_match_macro() {
        let result = edition_match! {
            lite => "lite_value",
            standard => "standard_value",
            pro => "pro_value"
        };

        // Result should be one of the three values
        assert!(result == "lite_value" || result == "standard_value" || result == "pro_value");
    }

    #[test]
    fn test_supported_features() {
        let lite_features = Edition::Lite.supported_features();
        assert!(lite_features.contains(&"ssh"));
        assert!(lite_features.contains(&"keychain"));
        assert!(lite_features.contains(&"native-terminal"));
        // Lite should NOT have these
        assert!(!lite_features.contains(&"embedded-terminal"));
        assert!(!lite_features.contains(&"team"));

        // Standard should have more features
        let standard_features = Edition::Standard.supported_features();
        assert!(standard_features.contains(&"embedded-terminal"));
        assert!(standard_features.contains(&"split-screen"));
        assert!(standard_features.contains(&"sftp"));
        assert!(!standard_features.contains(&"team"));

        // Pro should have all features
        let pro_features = Edition::Pro.supported_features();
        assert!(pro_features.contains(&"team"));
        assert!(pro_features.contains(&"audit"));
        assert!(pro_features.contains(&"sso"));
        assert!(pro_features.contains(&"embedded-terminal"));
    }

    #[test]
    fn test_version_info_full_version_string() {
        let info = VersionInfo::current();
        let full = info.full_version_string();

        assert!(full.contains(&info.edition_full_name));
        assert!(full.contains(&info.version));
        assert!(full.contains(&info.build_type_name));
    }

    #[test]
    fn test_msi_dmg_deb_rpm_names() {
        let info = VersionInfo::current();

        let msi = info.msi_name("x64");
        assert!(msi.contains("easyssh"));
        assert!(msi.contains(info.edition.identifier()));
        assert!(msi.ends_with(".msi"));

        let dmg = info.dmg_name("arm64");
        assert!(dmg.ends_with(".dmg"));

        let deb = info.deb_name("amd64");
        assert!(deb.ends_with(".deb"));

        let rpm = info.rpm_name("x86_64");
        assert!(rpm.ends_with(".rpm"));
    }
}
