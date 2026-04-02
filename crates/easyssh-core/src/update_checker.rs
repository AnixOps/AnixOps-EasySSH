//! Lite版本更新检测模块
//!
//! 为EasySSH Lite版本提供轻量化的版本检测功能：
//! - 检查GitHub Releases最新版本
//! - 比较当前版本
//! - 显示更新提示（引导用户到下载页面）
//! - 支持配置更新检查频率、通道、忽略特定版本
//!
//! 注意：这是Lite版本的简化实现，不提供自动下载和安装功能。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

/// 当前应用版本（编译时从Cargo.toml获取）
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub Releases API默认端点
pub const DEFAULT_GITHUB_API_URL: &str =
    "https://api.github.com/repos/AnixTeam/EasySSH/releases/latest";

/// GitHub Releases页面（用于引导用户下载）
pub const DEFAULT_GITHUB_RELEASES_URL: &str = "https://github.com/AnixTeam/EasySSH/releases";

/// HTTP请求超时时间（秒）
const DEFAULT_HTTP_TIMEOUT: u64 = 10;

/// 更新检查错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum UpdateCheckError {
    /// 网络请求失败
    #[error("网络请求失败: {0}")]
    Network(String),

    /// HTTP错误
    #[error("HTTP错误: {status} - {message}")]
    Http { status: u16, message: String },

    /// 解析响应失败
    #[error("解析响应失败: {0}")]
    Parse(String),

    /// 版本比较错误
    #[error("版本比较错误: {0}")]
    VersionCompare(String),

    /// 配置错误
    #[error("配置错误: {0}")]
    Config(String),

    /// 用户已禁用更新检查
    #[error("更新检查已禁用")]
    Disabled,
}

/// 更新检查通道
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateChannel {
    /// 稳定版 - 只检查正式发布的版本
    Stable,
    /// 预览版 - 包括预发布版本（beta, rc）
    Preview,
}

impl Default for UpdateChannel {
    fn default() -> Self {
        UpdateChannel::Stable
    }
}

impl std::fmt::Display for UpdateChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateChannel::Stable => write!(f, "stable"),
            UpdateChannel::Preview => write!(f, "preview"),
        }
    }
}

/// 更新检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckerConfig {
    /// 是否启用更新检查
    pub enabled: bool,
    /// GitHub API URL（默认为官方仓库）
    pub github_api_url: String,
    /// GitHub Releases页面URL
    pub github_releases_url: String,
    /// 检查频率（秒）
    pub check_interval_secs: u64,
    /// 更新通道
    pub channel: UpdateChannel,
    /// 忽略的版本列表
    pub ignored_versions: Vec<String>,
    /// 最后检查时间（Unix时间戳）
    pub last_check_timestamp: Option<i64>,
    /// HTTP超时时间（秒）
    pub http_timeout_secs: u64,
    /// 是否显示预览版更新提示
    pub notify_preview: bool,
}

impl Default for UpdateCheckerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            github_api_url: DEFAULT_GITHUB_API_URL.to_string(),
            github_releases_url: DEFAULT_GITHUB_RELEASES_URL.to_string(),
            check_interval_secs: 86400, // 默认每天检查一次
            channel: UpdateChannel::Stable,
            ignored_versions: Vec::new(),
            last_check_timestamp: None,
            http_timeout_secs: DEFAULT_HTTP_TIMEOUT,
            notify_preview: false,
        }
    }
}

impl UpdateCheckerConfig {
    /// 创建最小配置（仅检查稳定版）
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            check_interval_secs: 604800, // 每周检查一次
            ..Default::default()
        }
    }

    /// 创建开发配置（检查预览版，频繁检查）
    pub fn development() -> Self {
        Self {
            enabled: true,
            channel: UpdateChannel::Preview,
            check_interval_secs: 3600, // 每小时检查一次
            notify_preview: true,
            ..Default::default()
        }
    }

    /// 禁用更新检查
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// 检查是否应该忽略指定版本
    pub fn should_ignore(&self, version: &str) -> bool {
        self.ignored_versions.contains(&version.to_string())
    }

    /// 添加要忽略的版本
    pub fn ignore_version(&mut self, version: &str) {
        if !self.ignored_versions.contains(&version.to_string()) {
            self.ignored_versions.push(version.to_string());
        }
    }

    /// 移除忽略的版本
    pub fn unignore_version(&mut self, version: &str) {
        self.ignored_versions.retain(|v| v != version);
    }

    /// 检查是否应该进行检查（基于时间间隔）
    pub fn should_check(&self) -> bool {
        if !self.enabled {
            return false;
        }

        match self.last_check_timestamp {
            None => true,
            Some(last) => {
                let now = Utc::now().timestamp();
                let elapsed = now - last;
                elapsed >= self.check_interval_secs as i64
            }
        }
    }

    /// 更新最后检查时间
    pub fn update_last_check(&mut self) {
        self.last_check_timestamp = Some(Utc::now().timestamp());
    }
}

/// GitHub Release信息（从API获取）
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    /// 版本标签（如 "v1.2.3"）
    pub tag_name: String,
    /// 发布名称
    pub name: String,
    /// 发布说明
    pub body: Option<String>,
    /// 是否为预发布版本
    pub prerelease: bool,
    /// 发布日期
    pub published_at: String,
    /// 发布页面URL
    pub html_url: String,
    /// 资源列表
    pub assets: Vec<GitHubAsset>,
}

/// GitHub Release资源文件
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    /// 文件名
    pub name: String,
    /// 下载URL
    pub browser_download_url: String,
    /// 文件大小
    pub size: u64,
}

/// 更新信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// 最新版本号
    pub version: String,
    /// 当前版本号
    pub current_version: String,
    /// 是否有新版本
    pub has_update: bool,
    /// 是否为强制更新
    pub is_mandatory: bool,
    /// 发布日期
    pub release_date: DateTime<Utc>,
    /// 发布说明
    pub release_notes: String,
    /// 下载页面URL
    pub download_url: String,
    /// 是否包含适合当前平台的安装包
    pub has_compatible_asset: bool,
    /// 资源文件列表
    pub assets: Vec<AssetInfo>,
}

/// 资源文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    /// 文件名
    pub filename: String,
    /// 下载URL
    pub download_url: String,
    /// 文件大小
    pub size: u64,
    /// 是否适合当前平台
    pub is_for_current_platform: bool,
}

/// 版本比较结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionCompareResult {
    /// 当前版本较新
    CurrentNewer,
    /// 版本相同
    Same,
    /// 有新版本可用
    UpdateAvailable,
}

/// 更新检查结果
#[derive(Debug, Clone)]
pub enum UpdateCheckResult {
    /// 已是最新版本
    UpToDate,
    /// 有新版本可用
    UpdateAvailable(UpdateInfo),
    /// 检查被跳过（用户禁用或时间间隔未到）
    Skipped { reason: String },
    /// 检查出错
    Error(UpdateCheckError),
}

/// 更新检测器
#[derive(Debug, Clone)]
pub struct UpdateChecker {
    config: Arc<RwLock<UpdateCheckerConfig>>,
    current_version: String,
}

impl UpdateChecker {
    /// 创建新的更新检测器
    pub fn new(config: UpdateCheckerConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            current_version: CURRENT_VERSION.to_string(),
        }
    }

    /// 使用默认配置创建
    pub fn default() -> Self {
        Self::new(UpdateCheckerConfig::default())
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> UpdateCheckerConfig {
        self.config.read().await.clone()
    }

    /// 更新配置
    pub async fn set_config(&self, config: UpdateCheckerConfig) {
        *self.config.write().await = config;
    }

    /// 检查是否启用
    pub async fn is_enabled(&self) -> bool {
        self.config.read().await.enabled
    }

    /// 启用更新检查
    pub async fn enable(&self) {
        self.config.write().await.enabled = true;
    }

    /// 禁用更新检查
    pub async fn disable(&self) {
        self.config.write().await.enabled = false;
    }

    /// 获取当前版本
    pub fn get_current_version(&self) -> &str {
        &self.current_version
    }

    /// 执行版本检查（异步）
    pub async fn check(&self) -> UpdateCheckResult {
        let config = self.config.read().await.clone();

        // 检查是否启用
        if !config.enabled {
            return UpdateCheckResult::Skipped {
                reason: "更新检查已禁用".to_string(),
            };
        }

        // 检查时间间隔
        if !config.should_check() {
            return UpdateCheckResult::Skipped {
                reason: "距离上次检查时间太短".to_string(),
            };
        }

        // 执行检查
        match self.check_github_releases(&config).await {
            Ok(release) => {
                // 更新最后检查时间
                let mut cfg = self.config.write().await;
                cfg.update_last_check();
                drop(cfg);

                // 处理检查结果
                self.process_release(release, &config).await
            }
            Err(e) => UpdateCheckResult::Error(e),
        }
    }

    /// 从GitHub Releases检查更新
    async fn check_github_releases(
        &self,
        config: &UpdateCheckerConfig,
    ) -> Result<GitHubRelease, UpdateCheckError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.http_timeout_secs))
            .user_agent(format!("EasySSH/{} (UpdateChecker)", self.current_version))
            .build()
            .map_err(|e| UpdateCheckError::Network(e.to_string()))?;

        let response = client
            .get(&config.github_api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| UpdateCheckError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(UpdateCheckError::Http {
                status: status.as_u16(),
                message,
            });
        }

        let release: GitHubRelease = response
            .json()
            .await
            .map_err(|e| UpdateCheckError::Parse(e.to_string()))?;

        Ok(release)
    }

    /// 处理GitHub Release信息
    async fn process_release(
        &self,
        release: GitHubRelease,
        config: &UpdateCheckerConfig,
    ) -> UpdateCheckResult {
        // 提取版本号（移除 'v' 前缀）
        let latest_version = release.tag_name.trim_start_matches('v').to_string();

        // 检查是否被忽略
        if config.should_ignore(&latest_version) {
            return UpdateCheckResult::Skipped {
                reason: format!("版本 {} 已被用户忽略", latest_version),
            };
        }

        // 检查通道
        if release.prerelease && config.channel == UpdateChannel::Stable && !config.notify_preview {
            return UpdateCheckResult::Skipped {
                reason: "预览版更新（稳定版通道跳过）".to_string(),
            };
        }

        // 比较版本
        match compare_versions(&self.current_version, &latest_version) {
            Ok(VersionCompareResult::CurrentNewer) => {
                // 当前版本比远程新（可能是开发版本）
                UpdateCheckResult::UpToDate
            }
            Ok(VersionCompareResult::Same) => UpdateCheckResult::UpToDate,
            Ok(VersionCompareResult::UpdateAvailable) => {
                // 构建更新信息
                let release_date =
                    parse_github_date(&release.published_at).unwrap_or_else(|_| Utc::now());

                let assets: Vec<AssetInfo> = release
                    .assets
                    .iter()
                    .map(|asset| AssetInfo {
                        filename: asset.name.clone(),
                        download_url: asset.browser_download_url.clone(),
                        size: asset.size,
                        is_for_current_platform: is_asset_for_platform(&asset.name),
                    })
                    .collect();

                let has_compatible = assets.iter().any(|a| a.is_for_current_platform);

                let update_info = UpdateInfo {
                    version: latest_version,
                    current_version: self.current_version.clone(),
                    has_update: true,
                    is_mandatory: false, // Lite版本没有强制更新机制
                    release_date,
                    release_notes: release.body.unwrap_or_default(),
                    download_url: config.github_releases_url.clone(),
                    has_compatible_asset: has_compatible,
                    assets,
                };

                UpdateCheckResult::UpdateAvailable(update_info)
            }
            Err(e) => UpdateCheckResult::Error(e),
        }
    }

    /// 开始后台定期检查
    ///
    /// 返回一个任务句柄，可以通过 dropping 它来停止后台检查
    pub async fn start_background_checks(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval_secs = self.config.read().await.check_interval_secs;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                let config = self.config.read().await;
                if !config.enabled || !config.should_check() {
                    continue;
                }
                drop(config);

                // 执行检查但不处理结果（记录日志即可）
                match self.check().await {
                    UpdateCheckResult::UpdateAvailable(info) => {
                        log::info!(
                            "发现新版本: {} (当前: {})",
                            info.version,
                            info.current_version
                        );
                    }
                    UpdateCheckResult::Error(e) => {
                        log::warn!("更新检查失败: {}", e);
                    }
                    _ => {}
                }
            }
        })
    }

    /// 忽略指定版本
    pub async fn ignore_version(&self, version: &str) {
        let mut config = self.config.write().await;
        config.ignore_version(version);
    }

    /// 取消忽略版本
    pub async fn unignore_version(&self, version: &str) {
        let mut config = self.config.write().await;
        config.unignore_version(version);
    }

    /// 获取被忽略的版本列表
    pub async fn get_ignored_versions(&self) -> Vec<String> {
        self.config.read().await.ignored_versions.clone()
    }

    /// 立即检查（忽略时间间隔）
    pub async fn check_now(&self) -> UpdateCheckResult {
        let mut config = self.config.read().await.clone();
        config.last_check_timestamp = None; // 重置时间戳强制检查
        drop(config);

        self.check().await
    }
}

/// 比较两个版本号
///
/// 支持语义化版本格式: major.minor.patch[-prerelease]
fn compare_versions(current: &str, latest: &str) -> Result<VersionCompareResult, UpdateCheckError> {
    let current_parts = parse_version(current)?;
    let latest_parts = parse_version(latest)?;

    // 比较主版本号
    if latest_parts.0 > current_parts.0 {
        return Ok(VersionCompareResult::UpdateAvailable);
    } else if latest_parts.0 < current_parts.0 {
        return Ok(VersionCompareResult::CurrentNewer);
    }

    // 比较次版本号
    if latest_parts.1 > current_parts.1 {
        return Ok(VersionCompareResult::UpdateAvailable);
    } else if latest_parts.1 < current_parts.1 {
        return Ok(VersionCompareResult::CurrentNewer);
    }

    // 比较修订版本号
    if latest_parts.2 > current_parts.2 {
        return Ok(VersionCompareResult::UpdateAvailable);
    } else if latest_parts.2 < current_parts.2 {
        return Ok(VersionCompareResult::CurrentNewer);
    }

    // 版本号相同，检查预发布版本
    match (current_parts.3.as_ref(), latest_parts.3.as_ref()) {
        (None, None) => Ok(VersionCompareResult::Same),
        (Some(_), None) => Ok(VersionCompareResult::UpdateAvailable), // 当前是预发布，最新是正式版
        (None, Some(_)) => Ok(VersionCompareResult::CurrentNewer),    // 当前是正式版，最新是预发布
        (Some(c), Some(l)) => {
            // 比较预发布版本字符串
            if l > c {
                Ok(VersionCompareResult::UpdateAvailable)
            } else if l < c {
                Ok(VersionCompareResult::CurrentNewer)
            } else {
                Ok(VersionCompareResult::Same)
            }
        }
    }
}

/// 解析版本号字符串
///
/// 返回: (major, minor, patch, prerelease)
fn parse_version(version: &str) -> Result<(u32, u32, u32, Option<String>), UpdateCheckError> {
    let version = version.trim_start_matches('v');

    // 分割主版本和预发布版本
    let parts: Vec<&str> = version.split('-').collect();
    let main_version = parts[0];
    let prerelease = parts.get(1).map(|s| s.to_string());

    // 解析主版本号
    let nums: Vec<&str> = main_version.split('.').collect();
    if nums.len() < 2 {
        return Err(UpdateCheckError::VersionCompare(
            "版本号格式无效".to_string(),
        ));
    }

    let major = nums[0]
        .parse::<u32>()
        .map_err(|_| UpdateCheckError::VersionCompare("主版本号无效".to_string()))?;
    let minor = nums[1]
        .parse::<u32>()
        .map_err(|_| UpdateCheckError::VersionCompare("次版本号无效".to_string()))?;
    let patch = nums
        .get(2)
        .unwrap_or(&"0")
        .parse::<u32>()
        .map_err(|_| UpdateCheckError::VersionCompare("修订版本号无效".to_string()))?;

    Ok((major, minor, patch, prerelease))
}

/// 解析GitHub日期字符串
fn parse_github_date(date_str: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    // GitHub API返回的格式: 2024-01-15T10:30:00Z
    DateTime::parse_from_rfc3339(date_str).map(|dt| dt.with_timezone(&Utc))
}

/// 检查资源文件是否适合当前平台
fn is_asset_for_platform(filename: &str) -> bool {
    let platform = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let lowercase = filename.to_lowercase();

    match platform {
        "windows" => {
            lowercase.contains("windows")
                || lowercase.contains("win")
                || lowercase.ends_with(".exe")
                || lowercase.ends_with(".msi")
        }
        "macos" | "darwin" => {
            lowercase.contains("macos")
                || lowercase.contains("darwin")
                || lowercase.contains("mac")
                || lowercase.ends_with(".dmg")
                || lowercase.ends_with(".pkg")
                || lowercase.ends_with(".app.tar.gz")
        }
        "linux" => {
            let is_linux = lowercase.contains("linux")
                || lowercase.contains("ubuntu")
                || lowercase.contains("debian");

            // 检查架构
            let arch_match = match arch {
                "x86_64" => lowercase.contains("x86_64") || lowercase.contains("amd64"),
                "aarch64" => lowercase.contains("aarch64") || lowercase.contains("arm64"),
                _ => true, // 未知架构，假设匹配
            };

            is_linux && arch_match
        }
        _ => false,
    }
}

/// 获取当前平台的下载文件名建议
pub fn get_platform_asset_name() -> String {
    let platform = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match platform {
        "windows" => "EasySSH-Windows-x64.exe".to_string(),
        "macos" => format!("EasySSH-macOS-{}.dmg", arch),
        "linux" => format!("EasySSH-Linux-{}.AppImage", arch),
        _ => format!("EasySSH-{}-{}", platform, arch),
    }
}

/// 检查更新（便捷函数）
pub async fn check_update() -> UpdateCheckResult {
    let checker = UpdateChecker::default();
    checker.check().await
}

/// 快速检查是否有更新（仅返回布尔值）
pub async fn has_update() -> bool {
    match check_update().await {
        UpdateCheckResult::UpdateAvailable(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions() {
        // 新版本可用
        assert_eq!(
            compare_versions("1.0.0", "1.0.1").unwrap(),
            VersionCompareResult::UpdateAvailable
        );
        assert_eq!(
            compare_versions("1.0.0", "1.1.0").unwrap(),
            VersionCompareResult::UpdateAvailable
        );
        assert_eq!(
            compare_versions("1.0.0", "2.0.0").unwrap(),
            VersionCompareResult::UpdateAvailable
        );

        // 相同版本
        assert_eq!(
            compare_versions("1.0.0", "1.0.0").unwrap(),
            VersionCompareResult::Same
        );
        assert_eq!(
            compare_versions("1.0.0-alpha", "1.0.0-alpha").unwrap(),
            VersionCompareResult::Same
        );

        // 当前版本较新
        assert_eq!(
            compare_versions("1.0.1", "1.0.0").unwrap(),
            VersionCompareResult::CurrentNewer
        );

        // 预发布版本比较
        assert_eq!(
            compare_versions("1.0.0-beta", "1.0.0").unwrap(),
            VersionCompareResult::UpdateAvailable
        );
        assert_eq!(
            compare_versions("1.0.0", "1.0.0-beta").unwrap(),
            VersionCompareResult::CurrentNewer
        );
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.2.3").unwrap(), (1, 2, 3, None));
        assert_eq!(parse_version("v1.2.3").unwrap(), (1, 2, 3, None));
        assert_eq!(
            parse_version("1.2.3-alpha").unwrap(),
            (1, 2, 3, Some("alpha".to_string()))
        );
        assert_eq!(parse_version("1.2").unwrap(), (1, 2, 0, None));
    }

    #[test]
    fn test_is_asset_for_platform() {
        // Windows
        assert!(is_asset_for_platform("EasySSH-Windows-x64.exe"));
        assert!(is_asset_for_platform("easyssh-1.0.0-win64.msi"));

        // macOS
        assert!(is_asset_for_platform("EasySSH-macOS-x86_64.dmg"));
        assert!(is_asset_for_platform("easyssh-1.0.0-macos-arm64.pkg"));

        // Linux
        assert!(is_asset_for_platform("EasySSH-Linux-x86_64.AppImage"));
        assert!(is_asset_for_platform("easyssh-1.0.0-linux-amd64.deb"));
    }

    #[test]
    fn test_config_should_ignore() {
        let mut config = UpdateCheckerConfig::default();
        assert!(!config.should_ignore("1.0.0"));

        config.ignore_version("1.0.0");
        assert!(config.should_ignore("1.0.0"));
        assert!(!config.should_ignore("1.0.1"));

        config.unignore_version("1.0.0");
        assert!(!config.should_ignore("1.0.0"));
    }

    #[test]
    fn test_config_should_check() {
        let mut config = UpdateCheckerConfig::default();
        assert!(config.should_check()); // 从未检查过

        config.last_check_timestamp = Some(Utc::now().timestamp());
        assert!(!config.should_check()); // 刚刚检查过

        // 模拟时间过去
        config.last_check_timestamp = Some(Utc::now().timestamp() - 86401);
        assert!(config.should_check()); // 超过一天

        config.enabled = false;
        assert!(!config.should_check()); // 已禁用
    }
}
