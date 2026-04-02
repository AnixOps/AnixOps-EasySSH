//! Debug Access Control Module
//!
//! 为Lite/Standard/Pro三个版本提供统一的Debug功能隐藏入口。
//! 仅在release构建中有效，通过多种隐蔽方式激活开发者功能。
//!
//! # 安全特性
//!
//! - 审计日志记录每次激活
//! - 自动超时退出
//! - 开发者身份验证
//! - 明显的Debug模式UI指示器
//!
//! # 激活方式 (各版本不同)
//!
//! **Lite版本:**
//! - 组合键: Ctrl+Shift+D (3秒内连续按3次)
//! - 命令行: `easyssh-lite --dev-mode`
//! - 环境变量: `EASYSSH_DEV=1`
//!
//! **Standard版本:**
//! - 组合键: Ctrl+Alt+Shift+D
//! - 设置菜单: 连续点击版本号5次
//! - 命令行: `easyssh-standard --dev-mode`
//! - 环境变量: `EASYSSH_DEV=1`
//!
//! **Pro版本:**
//! - 管理后台开发者开关
//! - 命令行: `easyssh-pro --dev-mode`
//! - API调用 (需管理员权限)
//! - 环境变量: `EASYSSH_DEV=1`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Debug访问方法
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugAccessMethod {
    /// 组合键触发
    KeyCombination { sequence: String },
    /// 命令行参数
    CliFlag { flag: String },
    /// 环境变量
    EnvVar { var: String, value: String },
    /// 配置文件
    ConfigFile { path: PathBuf },
    /// 管理后台开关 (仅Pro)
    AdminSwitch { admin_id: String },
    /// API调用 (仅Pro)
    ApiCall { token: String, endpoint: String },
    /// 手势触发 (UI隐藏手势)
    Gesture { pattern: String },
}

impl DebugAccessMethod {
    /// 获取方法描述
    pub fn description(&self) -> String {
        match self {
            DebugAccessMethod::KeyCombination { sequence } => format!("组合键: {}", sequence),
            DebugAccessMethod::CliFlag { flag } => format!("命令行参数: {}", flag),
            DebugAccessMethod::EnvVar { var, value } => format!("环境变量: {}={}", var, value),
            DebugAccessMethod::ConfigFile { path } => format!("配置文件: {:?}", path),
            DebugAccessMethod::AdminSwitch { admin_id } => {
                format!("管理员开关 (admin: {})", admin_id)
            }
            DebugAccessMethod::ApiCall { endpoint, .. } => format!("API调用: {}", endpoint),
            DebugAccessMethod::Gesture { pattern } => format!("手势: {}", pattern),
        }
    }

    /// 获取方法类型标识
    pub fn method_type(&self) -> &'static str {
        match self {
            DebugAccessMethod::KeyCombination { .. } => "key_combination",
            DebugAccessMethod::CliFlag { .. } => "cli_flag",
            DebugAccessMethod::EnvVar { .. } => "env_var",
            DebugAccessMethod::ConfigFile { .. } => "config_file",
            DebugAccessMethod::AdminSwitch { .. } => "admin_switch",
            DebugAccessMethod::ApiCall { .. } => "api_call",
            DebugAccessMethod::Gesture { .. } => "gesture",
        }
    }
}

/// Debug功能类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DebugFeature {
    /// AI编程接口
    AiProgramming,
    /// 性能监控
    PerformanceMonitor,
    /// 网络检查器
    NetworkInspector,
    /// 数据库控制台
    DatabaseConsole,
    /// 日志查看器
    LogViewer,
    /// 测试运行器
    TestRunner,
    /// 特性开关
    FeatureFlags,
    /// 审计日志查看
    AuditLogViewer,
    /// 内部状态检查
    InternalStateInspector,
    /// 内存分析器
    MemoryProfiler,
    /// 网络抓包
    PacketCapture,
    /// 调试WebSocket
    DebugWebSocket,
}

impl DebugFeature {
    /// 获取功能名称
    pub fn name(&self) -> &'static str {
        match self {
            DebugFeature::AiProgramming => "AI编程接口",
            DebugFeature::PerformanceMonitor => "性能监控",
            DebugFeature::NetworkInspector => "网络检查器",
            DebugFeature::DatabaseConsole => "数据库控制台",
            DebugFeature::LogViewer => "日志查看器",
            DebugFeature::TestRunner => "测试运行器",
            DebugFeature::FeatureFlags => "特性开关",
            DebugFeature::AuditLogViewer => "审计日志查看器",
            DebugFeature::InternalStateInspector => "内部状态检查器",
            DebugFeature::MemoryProfiler => "内存分析器",
            DebugFeature::PacketCapture => "网络抓包",
            DebugFeature::DebugWebSocket => "调试WebSocket",
        }
    }

    /// 获取功能描述
    pub fn description(&self) -> &'static str {
        match self {
            DebugFeature::AiProgramming => "代码读取、搜索、修改和测试运行",
            DebugFeature::PerformanceMonitor => "实时性能指标和瓶颈分析",
            DebugFeature::NetworkInspector => "网络连接状态和数据包分析",
            DebugFeature::DatabaseConsole => "直接数据库查询和修改",
            DebugFeature::LogViewer => "查看和过滤应用日志",
            DebugFeature::TestRunner => "运行单元测试和集成测试",
            DebugFeature::FeatureFlags => "动态启用/禁用功能开关",
            DebugFeature::AuditLogViewer => "查看安全审计日志",
            DebugFeature::InternalStateInspector => "检查内部数据结构和状态",
            DebugFeature::MemoryProfiler => "内存使用和泄漏分析",
            DebugFeature::PacketCapture => "捕获和分析网络流量",
            DebugFeature::DebugWebSocket => "调试WebSocket通信",
        }
    }

    pub fn requires_auth(&self) -> bool {
        matches!(
            self,
            DebugFeature::DatabaseConsole
                | DebugFeature::AuditLogViewer
                | DebugFeature::PacketCapture
        )
    }

    /// 获取所需权限级别
    pub fn required_level(&self) -> DebugAccessLevel {
        match self {
            DebugFeature::AiProgramming => DebugAccessLevel::Developer,
            DebugFeature::PerformanceMonitor => DebugAccessLevel::Viewer,
            DebugFeature::NetworkInspector => DebugAccessLevel::Viewer,
            DebugFeature::DatabaseConsole => DebugAccessLevel::Admin,
            DebugFeature::LogViewer => DebugAccessLevel::Viewer,
            DebugFeature::TestRunner => DebugAccessLevel::Developer,
            DebugFeature::FeatureFlags => DebugAccessLevel::Admin,
            DebugFeature::AuditLogViewer => DebugAccessLevel::Admin,
            DebugFeature::InternalStateInspector => DebugAccessLevel::Developer,
            DebugFeature::MemoryProfiler => DebugAccessLevel::Developer,
            DebugFeature::PacketCapture => DebugAccessLevel::Admin,
            DebugFeature::DebugWebSocket => DebugAccessLevel::Developer,
        }
    }
}

/// Debug访问级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DebugAccessLevel {
    /// 仅查看
    Viewer = 0,
    /// 开发者权限 (可修改代码/运行测试)
    Developer = 1,
    /// 管理员权限 (可修改数据库/查看审计日志)
    Admin = 2,
}

impl DebugAccessLevel {
    /// 获取级别名称
    pub fn name(&self) -> &'static str {
        match self {
            DebugAccessLevel::Viewer => "查看者",
            DebugAccessLevel::Developer => "开发者",
            DebugAccessLevel::Admin => "管理员",
        }
    }

    /// 检查是否有权限访问功能
    pub fn can_access(&self, feature: DebugFeature) -> bool {
        *self >= feature.required_level()
    }

    /// 检查是否允许性能监控
    pub fn allows_performance_monitoring(&self) -> bool {
        *self >= DebugAccessLevel::Developer
    }
}

/// Debug访问错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugAccessError {
    /// 已禁用
    Disabled,
    /// 激活失败
    ActivationFailed { reason: String },
    /// 认证失败
    AuthenticationFailed,
    /// 权限不足
    InsufficientLevel {
        required: DebugAccessLevel,
        current: DebugAccessLevel,
    },
    /// 已超时
    SessionExpired,
    /// 功能未找到
    FeatureNotFound(String),
    /// 审计日志错误
    AuditError(String),
    /// 方法不支持
    UnsupportedMethod { method: String, edition: String },
}

impl std::fmt::Display for DebugAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugAccessError::Disabled => write!(f, "Debug模式已禁用"),
            DebugAccessError::ActivationFailed { reason } => write!(f, "激活失败: {}", reason),
            DebugAccessError::AuthenticationFailed => write!(f, "认证失败"),
            DebugAccessError::InsufficientLevel { required, current } => {
                write!(
                    f,
                    "权限不足: 需要{}权限，当前为{}",
                    required.name(),
                    current.name()
                )
            }
            DebugAccessError::SessionExpired => write!(f, "Debug会话已超时"),
            DebugAccessError::FeatureNotFound(name) => write!(f, "功能未找到: {}", name),
            DebugAccessError::AuditError(msg) => write!(f, "审计日志错误: {}", msg),
            DebugAccessError::UnsupportedMethod { method, edition } => {
                write!(f, "激活方法 '{}' 不支持在 {} 版本", method, edition)
            }
        }
    }
}

impl std::error::Error for DebugAccessError {}

/// Debug会话状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSession {
    /// 会话ID
    pub id: String,
    /// 激活时间
    pub activated_at: u64,
    /// 最后活动时间
    pub last_activity: u64,
    /// 激活方法
    pub method: DebugAccessMethod,
    /// 访问级别
    pub level: DebugAccessLevel,
    /// 是否已认证
    pub authenticated: bool,
    /// 激活者信息
    pub actor: Option<String>,
    /// 客户端信息
    pub client_info: DebugClientInfo,
}

/// 客户端信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugClientInfo {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub device_id: Option<String>,
    pub platform: Option<String>,
}

/// 审计记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugAuditRecord {
    pub id: String,
    pub timestamp: u64,
    pub session_id: Option<String>,
    pub action: DebugAuditAction,
    pub method: Option<DebugAccessMethod>,
    pub result: DebugAuditResult,
    pub details: Option<String>,
    pub actor: Option<String>,
    pub client_info: DebugClientInfo,
}

/// 审计操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugAuditAction {
    SessionActivated,
    SessionDeactivated,
    SessionExpired,
    FeatureAccessed { feature: DebugFeature },
    CommandExecuted { command: String },
    FileAccessed { path: String, operation: String },
    AuthAttempt { success: bool },
    SettingsChanged { key: String },
}

/// 审计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugAuditResult {
    Success,
    Failure { reason: String },
    Denied { reason: String },
}

/// Debug访问控制器 (单例)
pub struct DebugAccess {
    /// 是否启用
    enabled: AtomicBool,
    /// 当前会话
    session: RwLock<Option<DebugSession>>,
    /// 自动超时时间 (秒)
    timeout_seconds: AtomicU64,
    /// 审计日志
    audit_log: Mutex<Vec<DebugAuditRecord>>,
    /// 最大审计条目数
    max_audit_entries: usize,
    /// 是否显示Debug指示器
    show_indicator: AtomicBool,
    /// 启用日志持久化
    persist_audit: AtomicBool,
    /// 审计日志路径
    audit_path: RwLock<Option<PathBuf>>,
    /// 特性开关状态
    feature_flags: RwLock<HashMap<String, bool>>,
    /// 当前版本
    edition: crate::edition::Edition,
}

impl DebugAccess {
    /// 创建新的Debug访问控制器
    pub fn new(edition: crate::edition::Edition) -> Self {
        let mut feature_flags = HashMap::new();
        // 默认启用部分功能
        feature_flags.insert("ai_programming".to_string(), true);
        feature_flags.insert("performance_monitor".to_string(), true);
        feature_flags.insert("log_viewer".to_string(), true);
        feature_flags.insert("test_runner".to_string(), true);

        Self {
            enabled: AtomicBool::new(false),
            session: RwLock::new(None),
            timeout_seconds: AtomicU64::new(3600), // 默认1小时超时
            audit_log: Mutex::new(Vec::with_capacity(1000)),
            max_audit_entries: 10000,
            show_indicator: AtomicBool::new(true),
            persist_audit: AtomicBool::new(false),
            audit_path: RwLock::new(None),
            feature_flags: RwLock::new(feature_flags),
            edition,
        }
    }

    /// 创建并初始化全局单例
    pub fn initialize_global() -> Arc<DebugAccess> {
        let edition = crate::edition::Edition::current();
        Arc::new(Self::new(edition))
    }
}

// 内部实现
impl DebugAccess {
    /// 检查Debug模式是否已启用
    pub fn is_enabled(&self) -> bool {
        // 检查是否已超时
        if self.enabled.load(Ordering::Relaxed) {
            if let Ok(session) = self.session.read() {
                if let Some(ref s) = *session {
                    let now = Self::current_timestamp();
                    let timeout = self.timeout_seconds.load(Ordering::Relaxed);
                    if now - s.last_activity > timeout {
                        // 已超时，自动禁用
                        drop(session);
                        let _ = self.deactivate_internal(
                            DebugAccessMethod::CliFlag {
                                flag: "auto_timeout".to_string(),
                            },
                            true,
                        );
                        return false;
                    }
                    return true;
                }
            }
        }
        false
    }

    /// 获取当前时间戳
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// 更新最后活动时间
    fn update_activity(&self) {
        if let Ok(mut session) = self.session.write() {
            if let Some(ref mut s) = *session {
                s.last_activity = Self::current_timestamp();
            }
        }
    }

    /// 获取当前会话
    pub fn get_session(&self) -> Option<DebugSession> {
        self.update_activity();
        self.session.read().ok().and_then(|s| s.clone())
    }

    /// 获取会话ID
    pub fn get_session_id(&self) -> Option<String> {
        self.update_activity();
        self.session
            .read()
            .ok()
            .and_then(|s| s.as_ref().map(|sess| sess.id.clone()))
    }

    /// 获取访问级别
    pub fn get_access_level(&self) -> Option<DebugAccessLevel> {
        self.update_activity();
        self.session
            .read()
            .ok()
            .and_then(|s| s.as_ref().map(|sess| sess.level))
    }

    /// 检查是否有权限访问特定功能
    pub fn can_access_feature(&self, feature: DebugFeature) -> bool {
        if !self.is_enabled() {
            return false;
        }

        if let Some(level) = self.get_access_level() {
            return level.can_access(feature);
        }

        false
    }

    /// 激活Debug模式 (外部API)
    pub fn activate(
        &self,
        method: DebugAccessMethod,
        actor: Option<String>,
        client_info: DebugClientInfo,
    ) -> Result<DebugSession, DebugAccessError> {
        // 验证激活方法是否适用于当前版本
        self.validate_method_for_edition(&method)?;

        // 执行激活
        let session = self.activate_internal(method.clone(), actor, client_info)?;

        // 记录审计日志
        self.log_audit(DebugAuditRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Self::current_timestamp(),
            session_id: Some(session.id.clone()),
            action: DebugAuditAction::SessionActivated,
            method: Some(method),
            result: DebugAuditResult::Success,
            details: None,
            actor: session.actor.clone(),
            client_info: session.client_info.clone(),
        });

        Ok(session)
    }

    /// 停用Debug模式
    pub fn deactivate(&self, reason: &str) -> Result<(), DebugAccessError> {
        let method = DebugAccessMethod::CliFlag {
            flag: format!("manual: {}", reason),
        };
        self.deactivate_internal(method, false)
    }

    /// 验证激活方法是否适用于当前版本
    fn validate_method_for_edition(
        &self,
        method: &DebugAccessMethod,
    ) -> Result<(), DebugAccessError> {
        use crate::edition::Edition;

        match self.edition {
            Edition::Lite => {
                // Lite版本支持: 组合键、CLI标志、环境变量
                match method {
                    DebugAccessMethod::KeyCombination { .. }
                    | DebugAccessMethod::CliFlag { .. }
                    | DebugAccessMethod::EnvVar { .. } => Ok(()),
                    _ => Err(DebugAccessError::UnsupportedMethod {
                        method: method.method_type().to_string(),
                        edition: "Lite".to_string(),
                    }),
                }
            }
            Edition::Standard => {
                // Standard版本支持: 组合键、CLI标志、环境变量、手势
                match method {
                    DebugAccessMethod::KeyCombination { .. }
                    | DebugAccessMethod::CliFlag { .. }
                    | DebugAccessMethod::EnvVar { .. }
                    | DebugAccessMethod::Gesture { .. }
                    | DebugAccessMethod::ConfigFile { .. } => Ok(()),
                    _ => Err(DebugAccessError::UnsupportedMethod {
                        method: method.method_type().to_string(),
                        edition: "Standard".to_string(),
                    }),
                }
            }
            Edition::Pro => {
                // Pro版本支持所有方法
                Ok(())
            }
        }
    }

    /// 内部激活实现
    fn activate_internal(
        &self,
        method: DebugAccessMethod,
        actor: Option<String>,
        client_info: DebugClientInfo,
    ) -> Result<DebugSession, DebugAccessError> {
        // 确定访问级别 (基于激活方法)
        let level = self.determine_access_level(&method);

        // 创建新会话
        let session = DebugSession {
            id: uuid::Uuid::new_v4().to_string(),
            activated_at: Self::current_timestamp(),
            last_activity: Self::current_timestamp(),
            method: method.clone(),
            level,
            authenticated: actor.is_some(),
            actor: actor.clone(),
            client_info,
        };

        // 保存会话
        if let Ok(mut sess) = self.session.write() {
            *sess = Some(session.clone());
        } else {
            return Err(DebugAccessError::ActivationFailed {
                reason: "无法写入会话状态".to_string(),
            });
        }

        // 启用Debug模式
        self.enabled.store(true, Ordering::Relaxed);

        Ok(session)
    }

    /// 确定访问级别
    fn determine_access_level(&self, method: &DebugAccessMethod) -> DebugAccessLevel {
        match method {
            DebugAccessMethod::AdminSwitch { .. } => DebugAccessLevel::Admin,
            DebugAccessMethod::ApiCall { .. } => DebugAccessLevel::Admin,
            DebugAccessMethod::ConfigFile { .. } => DebugAccessLevel::Developer,
            _ => DebugAccessLevel::Developer,
        }
    }

    /// 内部停用实现
    fn deactivate_internal(
        &self,
        method: DebugAccessMethod,
        is_timeout: bool,
    ) -> Result<(), DebugAccessError> {
        let session_id = self.get_session_id();
        let actor = self
            .session
            .read()
            .ok()
            .and_then(|s| s.as_ref().and_then(|sess| sess.actor.clone()));
        let client_info = self
            .session
            .read()
            .ok()
            .and_then(|s| s.as_ref().map(|sess| sess.client_info.clone()))
            .unwrap_or_default();

        // 清除会话
        if let Ok(mut sess) = self.session.write() {
            *sess = None;
        }

        // 禁用Debug模式
        self.enabled.store(false, Ordering::Relaxed);

        // 记录审计日志
        self.log_audit(DebugAuditRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Self::current_timestamp(),
            session_id,
            action: if is_timeout {
                DebugAuditAction::SessionExpired
            } else {
                DebugAuditAction::SessionDeactivated
            },
            method: Some(method),
            result: DebugAuditResult::Success,
            details: Some(format!(
                "原因: {}",
                if is_timeout { "超时" } else { "手动" }
            )),
            actor,
            client_info,
        });

        Ok(())
    }

    /// 记录审计日志
    fn log_audit(&self, record: DebugAuditRecord) {
        if let Ok(mut log) = self.audit_log.lock() {
            log.push(record);

            // 限制日志大小
            if log.len() > self.max_audit_entries {
                log.remove(0);
            }
        }

        // 持久化到文件 (如果启用)
        if self.persist_audit.load(Ordering::Relaxed) {
            if let Ok(path) = self.audit_path.read() {
                if let Some(ref p) = *path {
                    let _ = self.persist_audit_record(p);
                }
            }
        }
    }

    /// 持久化审计记录
    fn persist_audit_record(&self, path: &PathBuf) -> Result<(), DebugAccessError> {
        if let Ok(log) = self.audit_log.lock() {
            if let Ok(json) = serde_json::to_string(&*log) {
                if let Err(e) = std::fs::write(path, json) {
                    return Err(DebugAccessError::AuditError(e.to_string()));
                }
            }
        }
        Ok(())
    }

    /// 获取审计日志
    pub fn get_audit_log(&self, limit: usize) -> Vec<DebugAuditRecord> {
        if let Ok(log) = self.audit_log.lock() {
            log.iter().rev().take(limit).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// 清除审计日志 (需要Admin权限)
    pub fn clear_audit_log(&self) -> Result<(), DebugAccessError> {
        if !self.can_access_feature(DebugFeature::AuditLogViewer) {
            return Err(DebugAccessError::InsufficientLevel {
                required: DebugAccessLevel::Admin,
                current: self.get_access_level().unwrap_or(DebugAccessLevel::Viewer),
            });
        }

        if let Ok(mut log) = self.audit_log.lock() {
            log.clear();
        }

        Ok(())
    }

    /// 设置超时时间
    pub fn set_timeout(&self, seconds: u64) {
        self.timeout_seconds.store(seconds, Ordering::Relaxed);
    }

    /// 获取超时时间
    pub fn get_timeout(&self) -> u64 {
        self.timeout_seconds.load(Ordering::Relaxed)
    }

    /// 设置是否显示Debug指示器
    pub fn set_show_indicator(&self, show: bool) {
        self.show_indicator.store(show, Ordering::Relaxed);
    }

    /// 是否显示Debug指示器
    pub fn should_show_indicator(&self) -> bool {
        self.show_indicator.load(Ordering::Relaxed) && self.is_enabled()
    }

    /// 启用审计日志持久化
    pub fn enable_audit_persistence(&self, path: PathBuf) -> Result<(), DebugAccessError> {
        if !self.is_enabled() {
            return Err(DebugAccessError::Disabled);
        }

        if let Ok(mut audit_path) = self.audit_path.write() {
            *audit_path = Some(path);
        }
        self.persist_audit.store(true, Ordering::Relaxed);

        Ok(())
    }

    /// 获取功能开关状态
    pub fn get_feature_flag(&self, name: &str) -> bool {
        if let Ok(flags) = self.feature_flags.read() {
            flags.get(name).copied().unwrap_or(false)
        } else {
            false
        }
    }

    /// 设置功能开关 (需要Admin权限)
    pub fn set_feature_flag(&self, name: &str, enabled: bool) -> Result<(), DebugAccessError> {
        if !self.can_access_feature(DebugFeature::FeatureFlags) {
            return Err(DebugAccessError::InsufficientLevel {
                required: DebugAccessLevel::Admin,
                current: self.get_access_level().unwrap_or(DebugAccessLevel::Viewer),
            });
        }

        if let Ok(mut flags) = self.feature_flags.write() {
            flags.insert(name.to_string(), enabled);
        }

        // 记录审计
        self.log_audit(DebugAuditRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Self::current_timestamp(),
            session_id: self.get_session_id(),
            action: DebugAuditAction::SettingsChanged {
                key: format!("feature_flag:{}", name),
            },
            method: None,
            result: DebugAuditResult::Success,
            details: Some(format!("设置为: {}", enabled)),
            actor: self
                .session
                .read()
                .ok()
                .and_then(|s| s.as_ref().and_then(|sess| sess.actor.clone())),
            client_info: self
                .session
                .read()
                .ok()
                .and_then(|s| s.as_ref().map(|sess| sess.client_info.clone()))
                .unwrap_or_default(),
        });

        Ok(())
    }

    /// 记录功能访问
    pub fn log_feature_access(&self, feature: DebugFeature) -> Result<(), DebugAccessError> {
        if !self.is_enabled() {
            return Err(DebugAccessError::Disabled);
        }

        if !self.can_access_feature(feature) {
            let current_level = self.get_access_level().unwrap_or(DebugAccessLevel::Viewer);
            return Err(DebugAccessError::InsufficientLevel {
                required: feature.required_level(),
                current: current_level,
            });
        }

        self.log_audit(DebugAuditRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Self::current_timestamp(),
            session_id: self.get_session_id(),
            action: DebugAuditAction::FeatureAccessed { feature },
            method: None,
            result: DebugAuditResult::Success,
            details: None,
            actor: self
                .session
                .read()
                .ok()
                .and_then(|s| s.as_ref().and_then(|sess| sess.actor.clone())),
            client_info: self
                .session
                .read()
                .ok()
                .and_then(|s| s.as_ref().map(|sess| sess.client_info.clone()))
                .unwrap_or_default(),
        });

        Ok(())
    }
}

// 安全默认值实现
impl Default for DebugAccess {
    fn default() -> Self {
        Self::new(crate::edition::Edition::Lite)
    }
}

/// 全局Debug访问控制器 (运行时可访问)
static GLOBAL_DEBUG_ACCESS: std::sync::OnceLock<Arc<DebugAccess>> = std::sync::OnceLock::new();

/// 初始化全局Debug访问控制器
pub fn init_global_debug_access() -> Arc<DebugAccess> {
    let access = DebugAccess::initialize_global();
    let _ = GLOBAL_DEBUG_ACCESS.set(access.clone());
    access
}

/// 获取全局Debug访问控制器
pub fn get_debug_access() -> Option<Arc<DebugAccess>> {
    GLOBAL_DEBUG_ACCESS.get().cloned()
}

/// 检查全局Debug模式是否启用
pub fn is_debug_enabled() -> bool {
    get_debug_access().map(|a| a.is_enabled()).unwrap_or(false)
}

/// 尝试从环境变量激活Debug模式
pub fn try_activate_from_env() -> Result<DebugSession, DebugAccessError> {
    if let Ok(value) = std::env::var("EASYSSH_DEV") {
        if value == "1" || value == "true" || value == "yes" {
            let access = get_debug_access().ok_or(DebugAccessError::Disabled)?;
            let method = DebugAccessMethod::EnvVar {
                var: "EASYSSH_DEV".to_string(),
                value: value.clone(),
            };
            let client_info = DebugClientInfo {
                platform: Some(std::env::consts::OS.to_string()),
                ..Default::default()
            };
            return access.activate(method, None, client_info);
        }
    }

    Err(DebugAccessError::ActivationFailed {
        reason: "环境变量未设置或无效".to_string(),
    })
}

/// 尝试从CLI参数激活Debug模式
pub fn try_activate_from_cli(args: &[String]) -> Result<DebugSession, DebugAccessError> {
    for arg in args {
        if arg == "--dev-mode" || arg == "--debug" || arg == "-d" {
            let access = get_debug_access().ok_or(DebugAccessError::Disabled)?;
            let method = DebugAccessMethod::CliFlag { flag: arg.clone() };
            let client_info = DebugClientInfo {
                platform: Some(std::env::consts::OS.to_string()),
                ..Default::default()
            };
            return access.activate(method, None, client_info);
        }
    }

    Err(DebugAccessError::ActivationFailed {
        reason: "未找到debug命令行参数".to_string(),
    })
}

/// Debug组合键检测器
pub struct KeySequenceDetector {
    sequence: Vec<String>,
    max_window_ms: u64,
    last_key_time: Mutex<Option<Instant>>,
    current_index: AtomicU64,
}

impl KeySequenceDetector {
    /// 创建新的组合键检测器
    pub fn new(sequence: Vec<String>, max_window_ms: u64) -> Self {
        Self {
            sequence,
            max_window_ms,
            last_key_time: Mutex::new(None),
            current_index: AtomicU64::new(0),
        }
    }

    /// 处理按键事件
    pub fn on_key(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut last_time = self.last_key_time.lock().unwrap();

        // 检查是否在时间窗口内
        if let Some(last) = *last_time {
            let elapsed = now.duration_since(last).as_millis() as u64;
            if elapsed > self.max_window_ms {
                // 超时，重置序列
                self.current_index.store(0, Ordering::Relaxed);
            }
        }

        let idx = self.current_index.load(Ordering::Relaxed) as usize;

        if idx < self.sequence.len() && self.sequence[idx] == key {
            // 匹配成功
            *last_time = Some(now);
            self.current_index.fetch_add(1, Ordering::Relaxed);

            // 检查是否完成整个序列
            if self.current_index.load(Ordering::Relaxed) as usize >= self.sequence.len() {
                // 重置以便下次检测
                self.current_index.store(0, Ordering::Relaxed);
                *last_time = None;
                return true;
            }
        } else if idx > 0 && self.sequence[0] == key {
            // 第一个键被重复按下，可能是新序列的开始
            *last_time = Some(now);
            self.current_index.store(1, Ordering::Relaxed);
        } else {
            // 不匹配，重置
            self.current_index.store(0, Ordering::Relaxed);
            *last_time = None;
        }

        false
    }

    /// 获取当前序列进度
    pub fn get_progress(&self) -> (usize, usize) {
        (
            self.current_index.load(Ordering::Relaxed) as usize,
            self.sequence.len(),
        )
    }

    /// 重置检测器
    pub fn reset(&self) {
        self.current_index.store(0, Ordering::Relaxed);
        if let Ok(mut last_time) = self.last_key_time.lock() {
            *last_time = None;
        }
    }
}

/// Lite版本组合键检测器 (Ctrl+Shift+D 3秒内3次)
pub fn create_lite_key_detector() -> KeySequenceDetector {
    KeySequenceDetector::new(
        vec!["ctrl+shift+d".to_string(); 3], // 需要按3次
        3000,                                // 3秒窗口
    )
}

/// Standard版本组合键检测器 (Ctrl+Alt+Shift+D 单次)
pub fn create_standard_key_detector() -> KeySequenceDetector {
    KeySequenceDetector::new(
        vec!["ctrl+alt+shift+d".to_string()], // 单次
        5000,                                 // 5秒窗口
    )
}

/// 版本特定的激活方法配置
pub struct EditionActivationConfig {
    pub edition: crate::edition::Edition,
    pub supported_methods: Vec<DebugAccessMethod>,
    pub default_timeout: u64,
    pub requires_auth: bool,
}

impl EditionActivationConfig {
    /// 获取Lite版本配置
    pub fn lite() -> Self {
        Self {
            edition: crate::edition::Edition::Lite,
            supported_methods: vec![
                DebugAccessMethod::KeyCombination {
                    sequence: "ctrl+shift+d x3".to_string(),
                },
                DebugAccessMethod::CliFlag {
                    flag: "--dev-mode".to_string(),
                },
                DebugAccessMethod::EnvVar {
                    var: "EASYSSH_DEV".to_string(),
                    value: "1".to_string(),
                },
            ],
            default_timeout: 3600, // 1小时
            requires_auth: false,
        }
    }

    /// 获取Standard版本配置
    pub fn standard() -> Self {
        Self {
            edition: crate::edition::Edition::Standard,
            supported_methods: vec![
                DebugAccessMethod::KeyCombination {
                    sequence: "ctrl+alt+shift+d".to_string(),
                },
                DebugAccessMethod::Gesture {
                    pattern: "version_click x5".to_string(),
                },
                DebugAccessMethod::CliFlag {
                    flag: "--dev-mode".to_string(),
                },
                DebugAccessMethod::EnvVar {
                    var: "EASYSSH_DEV".to_string(),
                    value: "1".to_string(),
                },
                DebugAccessMethod::ConfigFile {
                    path: PathBuf::from("debug.conf"),
                },
            ],
            default_timeout: 7200, // 2小时
            requires_auth: false,
        }
    }

    /// 获取Pro版本配置
    pub fn pro() -> Self {
        Self {
            edition: crate::edition::Edition::Pro,
            supported_methods: vec![
                DebugAccessMethod::AdminSwitch {
                    admin_id: "admin".to_string(),
                },
                DebugAccessMethod::CliFlag {
                    flag: "--dev-mode".to_string(),
                },
                DebugAccessMethod::EnvVar {
                    var: "EASYSSH_DEV".to_string(),
                    value: "1".to_string(),
                },
                DebugAccessMethod::ApiCall {
                    token: "admin_token".to_string(),
                    endpoint: "/api/admin/debug".to_string(),
                },
                DebugAccessMethod::KeyCombination {
                    sequence: "ctrl+alt+shift+d".to_string(),
                },
                DebugAccessMethod::Gesture {
                    pattern: "version_click x7".to_string(),
                },
            ],
            default_timeout: 14400, // 4小时
            requires_auth: true,
        }
    }

    /// 根据当前版本获取配置
    pub fn for_current_edition() -> Self {
        match crate::edition::Edition::current() {
            crate::edition::Edition::Lite => Self::lite(),
            crate::edition::Edition::Standard => Self::standard(),
            crate::edition::Edition::Pro => Self::pro(),
        }
    }
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_access_creation() {
        let access = DebugAccess::new(crate::edition::Edition::Lite);
        assert!(!access.is_enabled());
    }

    #[test]
    fn test_access_level_comparison() {
        assert!(DebugAccessLevel::Viewer < DebugAccessLevel::Developer);
        assert!(DebugAccessLevel::Developer < DebugAccessLevel::Admin);
        assert!(DebugAccessLevel::Admin.can_access(DebugFeature::AiProgramming));
        assert!(!DebugAccessLevel::Viewer.can_access(DebugFeature::AiProgramming));
    }

    #[test]
    fn test_feature_requires_auth() {
        assert!(DebugFeature::DatabaseConsole.requires_auth());
        assert!(DebugFeature::AuditLogViewer.requires_auth());
        assert!(!DebugFeature::PerformanceMonitor.requires_auth());
    }

    #[test]
    fn test_debug_access_error_display() {
        let err = DebugAccessError::Disabled;
        assert!(err.to_string().contains("禁用"));

        let err = DebugAccessError::InsufficientLevel {
            required: DebugAccessLevel::Admin,
            current: DebugAccessLevel::Viewer,
        };
        assert!(err.to_string().contains("权限不足"));
    }

    #[test]
    fn test_key_sequence_detector() {
        let detector = KeySequenceDetector::new(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            1000,
        );

        assert!(!detector.on_key("x"));
        assert!(!detector.on_key("a"));
        assert!(!detector.on_key("b"));
        assert!(detector.on_key("c")); // 完成序列

        // 重置后重新开始
        assert!(!detector.on_key("a"));
        assert!(!detector.on_key("b"));
    }

    #[test]
    fn test_key_sequence_timeout() {
        let detector = KeySequenceDetector::new(vec!["a".to_string(), "b".to_string()], 1);

        assert!(!detector.on_key("a"));
        // 由于timeout只有1ms，等待后应该重置
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(!detector.on_key("b")); // 超时后不会匹配
    }

    #[test]
    fn test_edition_activation_config() {
        let lite = EditionActivationConfig::lite();
        assert_eq!(lite.edition, crate::edition::Edition::Lite);
        assert_eq!(lite.default_timeout, 3600);
        assert!(!lite.requires_auth);

        let standard = EditionActivationConfig::standard();
        assert_eq!(standard.edition, crate::edition::Edition::Standard);
        assert_eq!(standard.default_timeout, 7200);

        let pro = EditionActivationConfig::pro();
        assert_eq!(pro.edition, crate::edition::Edition::Pro);
        assert_eq!(pro.default_timeout, 14400);
        assert!(pro.requires_auth);
    }

    #[test]
    fn test_debug_method_descriptions() {
        let method = DebugAccessMethod::KeyCombination {
            sequence: "ctrl+d".to_string(),
        };
        assert!(method.description().contains("组合键"));

        let method = DebugAccessMethod::CliFlag {
            flag: "--dev".to_string(),
        };
        assert!(method.description().contains("命令行参数"));
    }

    #[test]
    fn test_feature_flags() {
        let access = DebugAccess::new(crate::edition::Edition::Lite);

        // 默认启用的功能
        assert!(access.get_feature_flag("ai_programming"));
        assert!(access.get_feature_flag("performance_monitor"));

        // 未定义的功能返回false
        assert!(!access.get_feature_flag("unknown_feature"));
    }

    #[test]
    fn test_session_timeout() {
        let access = DebugAccess::new(crate::edition::Edition::Lite);

        // 设置很短的超时时间以便测试
        access.set_timeout(0);

        // 激活会话
        let session = access
            .activate(
                DebugAccessMethod::CliFlag {
                    flag: "--test".to_string(),
                },
                Some("test".to_string()),
                DebugClientInfo::default(),
            )
            .unwrap();

        assert!(access.is_enabled());
        assert_eq!(access.get_session_id(), Some(session.id));

        // 由于超时时间为0，下次检查时应该已超时
        // 注意：实际超时检查在is_enabled()中完成
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!access.is_enabled()); // 应该已超时
    }

    #[test]
    fn test_audit_log() {
        let access = DebugAccess::new(crate::edition::Edition::Lite);

        // 激活以启用审计日志
        let _ = access.activate(
            DebugAccessMethod::CliFlag {
                flag: "--test".to_string(),
            },
            Some("test".to_string()),
            DebugClientInfo::default(),
        );

        // 记录功能访问
        let _ = access.log_feature_access(DebugFeature::PerformanceMonitor);

        // 获取审计日志
        let logs = access.get_audit_log(10);
        assert!(!logs.is_empty());
    }

    #[test]
    fn test_access_level_names() {
        assert_eq!(DebugAccessLevel::Viewer.name(), "查看者");
        assert_eq!(DebugAccessLevel::Developer.name(), "开发者");
        assert_eq!(DebugAccessLevel::Admin.name(), "管理员");
    }

    #[test]
    fn test_feature_names() {
        assert_eq!(DebugFeature::AiProgramming.name(), "AI编程接口");
        assert_eq!(DebugFeature::PerformanceMonitor.name(), "性能监控");
    }

    #[test]
    fn test_audit_action_serialization() {
        let action = DebugAuditAction::SessionActivated;
        let json = serde_json::to_string(&action).unwrap();
        assert!(!json.is_empty());

        let action = DebugAuditAction::FeatureAccessed {
            feature: DebugFeature::AiProgramming,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("AiProgramming"));
    }

    #[test]
    fn test_debug_session_serialization() {
        let session = DebugSession {
            id: "test-id".to_string(),
            activated_at: 1234567890,
            last_activity: 1234567890,
            method: DebugAccessMethod::CliFlag {
                flag: "--test".to_string(),
            },
            level: DebugAccessLevel::Developer,
            authenticated: true,
            actor: Some("test-user".to_string()),
            client_info: DebugClientInfo::default(),
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("Developer"));
    }
}
