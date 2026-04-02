//! 审计日志模块 (Pro版本)
//! 提供完整的操作审计追踪功能

use crate::error::LiteError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Write;
use uuid::Uuid;

/// 审计操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // 认证相关
    Login,
    Logout,
    LoginFailed,
    PasswordChange,
    MfaEnabled,
    MfaDisabled,

    // 服务器相关
    ServerCreate,
    ServerUpdate,
    ServerDelete,
    ServerConnect,
    ServerDisconnect,
    ServerImport,
    ServerExport,

    // 会话相关
    SessionStart,
    SessionEnd,
    SessionRecord,
    SessionShare,

    // 团队相关
    TeamCreate,
    TeamUpdate,
    TeamDelete,
    MemberInvite,
    MemberJoin,
    MemberRemove,
    MemberRoleChange,

    // 权限相关
    PermissionGrant,
    PermissionRevoke,
    RoleCreate,
    RoleUpdate,
    RoleDelete,

    // 配置相关
    ConfigUpdate,
    LayoutSave,
    LayoutDelete,

    // 安全相关
    KeyUpload,
    KeyDelete,
    SecretView,
    SsoLogin,
    IpBlocked,
}

impl AuditAction {
    /// 获取操作分类
    pub fn category(&self) -> AuditCategory {
        match self {
            AuditAction::Login
            | AuditAction::Logout
            | AuditAction::LoginFailed
            | AuditAction::PasswordChange
            | AuditAction::MfaEnabled
            | AuditAction::MfaDisabled => AuditCategory::Authentication,
            AuditAction::ServerCreate
            | AuditAction::ServerUpdate
            | AuditAction::ServerDelete
            | AuditAction::ServerConnect
            | AuditAction::ServerDisconnect
            | AuditAction::ServerImport
            | AuditAction::ServerExport => AuditCategory::Server,
            AuditAction::SessionStart
            | AuditAction::SessionEnd
            | AuditAction::SessionRecord
            | AuditAction::SessionShare => AuditCategory::Session,
            AuditAction::TeamCreate
            | AuditAction::TeamUpdate
            | AuditAction::TeamDelete
            | AuditAction::MemberInvite
            | AuditAction::MemberJoin
            | AuditAction::MemberRemove
            | AuditAction::MemberRoleChange => AuditCategory::Team,
            AuditAction::PermissionGrant
            | AuditAction::PermissionRevoke
            | AuditAction::RoleCreate
            | AuditAction::RoleUpdate
            | AuditAction::RoleDelete => AuditCategory::Permission,
            AuditAction::ConfigUpdate | AuditAction::LayoutSave | AuditAction::LayoutDelete => {
                AuditCategory::Configuration
            }
            AuditAction::KeyUpload
            | AuditAction::KeyDelete
            | AuditAction::SecretView
            | AuditAction::SsoLogin
            | AuditAction::IpBlocked => AuditCategory::Security,
        }
    }

    /// 获取操作严重级别
    pub fn severity(&self) -> AuditSeverity {
        match self {
            AuditAction::LoginFailed | AuditAction::IpBlocked => AuditSeverity::Warning,
            AuditAction::Login
            | AuditAction::Logout
            | AuditAction::ServerConnect
            | AuditAction::ServerDisconnect
            | AuditAction::SessionStart
            | AuditAction::SessionEnd => AuditSeverity::Info,
            AuditAction::ServerCreate
            | AuditAction::ServerUpdate
            | AuditAction::MemberJoin
            | AuditAction::MemberInvite
            | AuditAction::ConfigUpdate
            | AuditAction::LayoutSave => AuditSeverity::Info,
            AuditAction::ServerDelete
            | AuditAction::TeamDelete
            | AuditAction::RoleDelete
            | AuditAction::KeyDelete
            | AuditAction::MemberRemove => AuditSeverity::High,
            _ => AuditSeverity::Medium,
        }
    }

    /// 获取操作描述（中文）
    pub fn description(&self) -> &'static str {
        match self {
            AuditAction::Login => "用户登录",
            AuditAction::Logout => "用户登出",
            AuditAction::LoginFailed => "登录失败",
            AuditAction::PasswordChange => "密码修改",
            AuditAction::MfaEnabled => "启用多因素认证",
            AuditAction::MfaDisabled => "禁用多因素认证",
            AuditAction::ServerCreate => "创建服务器",
            AuditAction::ServerUpdate => "更新服务器",
            AuditAction::ServerDelete => "删除服务器",
            AuditAction::ServerConnect => "连接服务器",
            AuditAction::ServerDisconnect => "断开服务器",
            AuditAction::ServerImport => "导入服务器",
            AuditAction::ServerExport => "导出服务器",
            AuditAction::SessionStart => "开始会话",
            AuditAction::SessionEnd => "结束会话",
            AuditAction::SessionRecord => "录制会话",
            AuditAction::SessionShare => "共享会话",
            AuditAction::TeamCreate => "创建团队",
            AuditAction::TeamUpdate => "更新团队",
            AuditAction::TeamDelete => "删除团队",
            AuditAction::MemberInvite => "邀请成员",
            AuditAction::MemberJoin => "成员加入",
            AuditAction::MemberRemove => "移除成员",
            AuditAction::MemberRoleChange => "变更角色",
            AuditAction::PermissionGrant => "授予权限",
            AuditAction::PermissionRevoke => "撤销权限",
            AuditAction::RoleCreate => "创建角色",
            AuditAction::RoleUpdate => "更新角色",
            AuditAction::RoleDelete => "删除角色",
            AuditAction::ConfigUpdate => "更新配置",
            AuditAction::LayoutSave => "保存布局",
            AuditAction::LayoutDelete => "删除布局",
            AuditAction::KeyUpload => "上传密钥",
            AuditAction::KeyDelete => "删除密钥",
            AuditAction::SecretView => "查看密钥",
            AuditAction::SsoLogin => "SSO登录",
            AuditAction::IpBlocked => "IP被阻止",
        }
    }
}

/// 审计分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Authentication,
    Server,
    Session,
    Team,
    Permission,
    Configuration,
    Security,
}

/// 审计严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Info,
    Warning,
    Medium,
    High,
    Critical,
}

/// 审计目标类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AuditTarget {
    User { id: String },
    Server { id: String, host: Option<String> },
    Team { id: String },
    Session { id: String },
    Key { id: String },
    Config { key: String },
    System,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub action: AuditAction,
    pub actor: Actor,
    pub target: AuditTarget,
    pub details: Option<AuditDetails>,
    pub result: AuditResult,
    pub client_info: ClientInfo,
    // Tamper protection fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// 操作者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub user_id: String,
    pub username: String,
    pub team_id: Option<String>,
    pub role: Option<String>,
}

/// 操作详情
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditDetails {
    pub changes: Vec<ChangeRecord>,
    pub metadata: Option<serde_json::Value>,
    pub reason: Option<String>,
}

/// 变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// 操作结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditResult {
    Success,
    Failure,
    Denied,
    Error,
}

/// 客户端信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientInfo {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub device_id: Option<String>,
}

impl AuditEntry {
    /// 创建新的审计条目
    pub fn new(action: AuditAction, actor: Actor, target: AuditTarget) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            action,
            actor,
            target,
            details: None,
            result: AuditResult::Success,
            client_info: ClientInfo::default(),
            previous_hash: None,
            entry_hash: None,
            signature: None,
        }
    }

    /// 计算条目哈希 (用于防篡改)
    pub fn compute_hash(&self) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();

        // Hash critical fields
        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(format!("{:?}", self.action).as_bytes());
        hasher.update(self.actor.user_id.as_bytes());
        hasher.update(format!("{:?}", self.target).as_bytes());
        hasher.update(format!("{:?}", self.result).as_bytes());

        // Include previous hash if exists (chain hashing)
        if let Some(ref prev_hash) = self.previous_hash {
            hasher.update(prev_hash.as_bytes());
        }

        hasher.finalize().to_hex().to_string()
    }

    /// 设置哈希并签名条目
    pub fn seal(mut self, previous_hash: Option<&str>, signing_key: Option<&[u8]>) -> Self {
        self.previous_hash = previous_hash.map(|s| s.to_string());

        // Compute entry hash
        let hash = self.compute_hash();
        self.entry_hash = Some(hash.clone());

        // Sign if key provided
        if let Some(key) = signing_key {
            self.signature = Some(Self::sign_hash(&hash, key));
        }

        self
    }

    /// 验证条目完整性 (不包括签名验证,因为签名验证需要原始密钥)
    pub fn verify(&self) -> bool {
        // 如果没有哈希,说明条目未密封
        let stored_hash = match &self.entry_hash {
            Some(h) => h,
            None => return false,
        };

        // 重新计算哈希
        let computed_hash = self.compute_hash();
        computed_hash == *stored_hash
    }

    /// 使用提供的密钥验证签名
    pub fn verify_with_key(&self, key: &[u8]) -> bool {
        // 首先验证哈希
        if !self.verify() {
            return false;
        }

        // 如果有签名,验证签名
        if let Some(ref sig) = self.signature {
            let hash = self.entry_hash.as_ref().unwrap();
            return Self::verify_signature(hash, sig, key);
        }

        true
    }

    /// 签名哈希 (simplified HMAC for demo, should use proper key pair in production)
    fn sign_hash(hash: &str, key: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(hash.as_bytes());

        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// 验证签名
    fn verify_signature(hash: &str, signature: &str, key: &[u8]) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = match HmacSha256::new_from_slice(key) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(hash.as_bytes());

        let expected = Self::sign_hash(hash, key);
        signature == expected
    }

    /// 添加详情
    pub fn with_details(mut self, details: AuditDetails) -> Self {
        self.details = Some(details);
        self
    }

    /// 设置结果
    pub fn with_result(mut self, result: AuditResult) -> Self {
        self.result = result;
        self
    }

    /// 设置客户端信息
    pub fn with_client_info(mut self, info: ClientInfo) -> Self {
        self.client_info = info;
        self
    }

    /// 获取分类
    pub fn category(&self) -> AuditCategory {
        self.action.category()
    }

    /// 获取严重级别
    pub fn severity(&self) -> AuditSeverity {
        self.action.severity()
    }

    /// 转换为CSV格式
    pub fn to_csv(&self) -> String {
        format!(
            "{},{},{},{},{},{},{}",
            self.timestamp.to_rfc3339(),
            self.id,
            self.action.description(),
            self.actor.username,
            self.actor.team_id.as_deref().unwrap_or("-"),
            format!("{:?}", self.result),
            self.client_info.ip_address.as_deref().unwrap_or("-")
        )
    }
}

/// 审计过滤条件
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub actions: Option<Vec<AuditAction>>,
    pub categories: Option<Vec<AuditCategory>>,
    pub severities: Option<Vec<AuditSeverity>>,
    pub actor_id: Option<String>,
    pub team_id: Option<String>,
    pub target_id: Option<String>,
    pub result: Option<AuditResult>,
    pub ip_address: Option<String>,
}

impl AuditFilter {
    /// 检查条目是否匹配过滤条件
    pub fn matches(&self, entry: &AuditEntry) -> bool {
        if let Some(start) = self.start_time {
            if entry.timestamp < start {
                return false;
            }
        }

        if let Some(end) = self.end_time {
            if entry.timestamp > end {
                return false;
            }
        }

        if let Some(ref actions) = self.actions {
            if !actions.contains(&entry.action) {
                return false;
            }
        }

        if let Some(ref categories) = self.categories {
            if !categories.contains(&entry.action.category()) {
                return false;
            }
        }

        if let Some(ref severities) = self.severities {
            if !severities.contains(&entry.action.severity()) {
                return false;
            }
        }

        if let Some(ref actor_id) = self.actor_id {
            if entry.actor.user_id != *actor_id {
                return false;
            }
        }

        if let Some(ref team_id) = self.team_id {
            if entry.actor.team_id.as_ref() != Some(team_id) {
                return false;
            }
        }

        if let Some(ref result) = self.result {
            if entry.result != *result {
                return false;
            }
        }

        if let Some(ref ip) = self.ip_address {
            if entry.client_info.ip_address.as_ref() != Some(ip) {
                return false;
            }
        }

        true
    }
}

/// 审计统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_entries: usize,
    pub entries_by_category: std::collections::HashMap<String, usize>,
    pub entries_by_severity: std::collections::HashMap<String, usize>,
    pub failed_logins: usize,
    pub unique_actors: usize,
    pub time_range_start: Option<DateTime<Utc>>,
    pub time_range_end: Option<DateTime<Utc>>,
}

/// 审计日志管理器 (支持防篡改保护)
pub struct AuditLogger {
    entries: VecDeque<AuditEntry>,
    max_entries: usize,
    persist_path: Option<std::path::PathBuf>,
    signing_key: Option<Vec<u8>>,
    last_hash: Option<String>,
    tamper_check_enabled: bool,
}

impl AuditLogger {
    /// 创建新的审计日志管理器
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(10000),
            max_entries: 10000,
            persist_path: None,
            signing_key: None,
            last_hash: None,
            tamper_check_enabled: false,
        }
    }

    /// 启用防篡改保护
    pub fn with_tamper_protection(mut self, key: &[u8]) -> Self {
        self.signing_key = Some(key.to_vec());
        self.tamper_check_enabled = true;
        self
    }

    /// 设置最大条目数
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// 设置持久化路径
    pub fn with_persist_path(mut self, path: std::path::PathBuf) -> Self {
        self.persist_path = Some(path);
        self
    }

    /// 记录审计事件 (带防篡改保护)
    pub fn log(&mut self, entry: AuditEntry) {
        // Seal the entry with hash chain and signature if protection enabled
        let sealed_entry = if self.tamper_check_enabled {
            let key_ref = self.signing_key.as_deref();
            entry.seal(self.last_hash.as_deref(), key_ref)
        } else {
            entry
        };

        // Update last hash for chain
        if let Some(ref hash) = sealed_entry.entry_hash {
            self.last_hash = Some(hash.clone());
        }

        // If at capacity, remove oldest
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }

        self.entries.push_back(sealed_entry);

        // Optional: persist to file
        if let Some(ref path) = self.persist_path {
            if let Ok(file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let mut writer = std::io::LineWriter::new(file);
                if let Ok(json) = serde_json::to_string(&self.entries.back()) {
                    let _ = writeln!(writer, "{}", json);
                }
            }
        }
    }

    /// 验证整个审计日志的完整性
    pub fn verify_integrity(&self) -> AuditVerificationResult {
        if !self.tamper_check_enabled {
            return AuditVerificationResult {
                valid: true,
                total_entries: self.entries.len(),
                tampered_entries: vec![],
                broken_chain_at: None,
                error_message: Some("Tamper protection not enabled".to_string()),
            };
        }

        let mut result = AuditVerificationResult {
            valid: true,
            total_entries: self.entries.len(),
            tampered_entries: vec![],
            broken_chain_at: None,
            error_message: None,
        };

        let entries_vec: Vec<_> = self.entries.iter().collect();

        for (i, entry) in entries_vec.iter().enumerate() {
            // Verify individual entry
            if !entry.verify() {
                result.valid = false;
                result.tampered_entries.push((i, entry.id.clone()));
            }

            // Verify chain continuity (skip first entry)
            if i > 0 {
                let prev_entry = &entries_vec[i - 1];
                if let Some(ref prev_hash) = entry.previous_hash {
                    if let Some(ref prev_entry_hash) = prev_entry.entry_hash {
                        if prev_hash != prev_entry_hash {
                            result.valid = false;
                            if result.broken_chain_at.is_none() {
                                result.broken_chain_at = Some(i);
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// 获取防篡改状态
    pub fn is_tamper_protected(&self) -> bool {
        self.tamper_check_enabled
    }

    /// 记录快捷方法
    pub fn log_action(
        &mut self,
        action: AuditAction,
        actor: Actor,
        target: AuditTarget,
        result: AuditResult,
        ip: Option<&str>,
    ) {
        let entry = AuditEntry::new(action, actor, target)
            .with_result(result)
            .with_client_info(ClientInfo {
                ip_address: ip.map(|s| s.to_string()),
                user_agent: None,
                device_id: None,
            });
        self.log(entry);
    }

    /// 获取所有条目
    pub fn get_all(&self) -> Vec<&AuditEntry> {
        self.entries.iter().collect()
    }

    /// 获取过滤后的条目
    pub fn get_filtered(&self, filter: &AuditFilter) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| filter.matches(e)).collect()
    }

    /// 获取最近的N条
    pub fn get_recent(&self, n: usize) -> Vec<&AuditEntry> {
        self.entries.iter().rev().take(n).collect()
    }

    /// 搜索条目
    pub fn search(&self, query: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.actor.username.contains(query)
                    || e.action.description().contains(query)
                    || e.id.contains(query)
            })
            .collect()
    }

    /// 清空日志
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 获取条目数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 生成统计信息
    pub fn generate_stats(&self, filter: Option<&AuditFilter>) -> AuditStats {
        let entries: Vec<_> = match filter {
            Some(f) => self.entries.iter().filter(|e| f.matches(e)).collect(),
            None => self.entries.iter().collect(),
        };

        let mut by_category = std::collections::HashMap::new();
        let mut by_severity = std::collections::HashMap::new();
        let mut unique_actors = std::collections::HashSet::new();
        let mut failed_logins = 0;
        let mut time_start = None::<DateTime<Utc>>;
        let mut time_end = None::<DateTime<Utc>>;

        for entry in &entries {
            *by_category
                .entry(format!("{:?}", entry.action.category()))
                .or_insert(0) += 1;
            *by_severity
                .entry(format!("{:?}", entry.action.severity()))
                .or_insert(0) += 1;
            unique_actors.insert(&entry.actor.user_id);

            if entry.action == AuditAction::LoginFailed {
                failed_logins += 1;
            }

            if time_start.is_none() || Some(entry.timestamp) < time_start {
                time_start = Some(entry.timestamp);
            }
            if time_end.is_none() || Some(entry.timestamp) > time_end {
                time_end = Some(entry.timestamp);
            }
        }

        AuditStats {
            total_entries: entries.len(),
            entries_by_category: by_category,
            entries_by_severity: by_severity,
            failed_logins,
            unique_actors: unique_actors.len(),
            time_range_start: time_start,
            time_range_end: time_end,
        }
    }

    /// 导出为JSON
    pub fn export_json(&self, filter: Option<&AuditFilter>) -> Result<String, LiteError> {
        let entries: Vec<_> = match filter {
            Some(f) => self
                .entries
                .iter()
                .filter(|e| f.matches(e))
                .cloned()
                .collect(),
            None => self.entries.iter().cloned().collect(),
        };

        serde_json::to_string_pretty(&entries)
            .map_err(|e| LiteError::Audit(format!("Export failed: {}", e)))
    }

    /// 导出为CSV
    pub fn export_csv(&self, filter: Option<&AuditFilter>) -> Result<String, LiteError> {
        let mut csv = String::from("timestamp,id,action,actor,team,result,ip\n");

        let entries: Vec<_> = match filter {
            Some(f) => self.entries.iter().filter(|e| f.matches(e)).collect(),
            None => self.entries.iter().collect(),
        };

        for entry in entries {
            csv.push_str(&entry.to_csv());
            csv.push('\n');
        }

        Ok(csv)
    }

    /// 获取用户活动时间线
    pub fn get_user_timeline(&self, user_id: &str, limit: usize) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.actor.user_id == user_id)
            .rev()
            .take(limit)
            .collect()
    }

    /// 获取特定目标的审计记录
    pub fn get_target_history(&self, target_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| match &e.target {
                AuditTarget::User { id }
                | AuditTarget::Server { id, .. }
                | AuditTarget::Team { id }
                | AuditTarget::Session { id }
                | AuditTarget::Key { id } => id == target_id,
                AuditTarget::Config { key } => key == target_id,
                AuditTarget::System => false,
            })
            .collect()
    }

    /// 检测异常活动
    pub fn detect_anomalies(&self, user_id: &str) -> Vec<String> {
        let mut anomalies = Vec::new();
        let user_entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.actor.user_id == user_id)
            .collect();

        // 检查多次登录失败
        let failed_logins = user_entries
            .iter()
            .filter(|e| e.action == AuditAction::LoginFailed)
            .count();

        if failed_logins > 5 {
            anomalies.push(format!("用户 {} 有 {} 次登录失败", user_id, failed_logins));
        }

        // 检查来自不同IP的登录
        let unique_ips: std::collections::HashSet<_> = user_entries
            .iter()
            .filter(|e| matches!(e.action, AuditAction::Login | AuditAction::LoginFailed))
            .filter_map(|e| e.client_info.ip_address.as_ref())
            .collect();

        if unique_ips.len() > 3 {
            anomalies.push(format!(
                "用户 {} 从 {} 个不同IP登录",
                user_id,
                unique_ips.len()
            ));
        }

        anomalies
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// 审计验证结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditVerificationResult {
    pub valid: bool,
    pub total_entries: usize,
    pub tampered_entries: Vec<(usize, String)>, // (index, entry_id)
    pub broken_chain_at: Option<usize>,
    pub error_message: Option<String>,
}

/// 审计报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub generated_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub stats: AuditStats,
    pub top_actors: Vec<ActorSummary>,
    pub top_actions: Vec<ActionSummary>,
    pub anomalies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSummary {
    pub user_id: String,
    pub username: String,
    pub action_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSummary {
    pub action: AuditAction,
    pub count: usize,
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_action_category() {
        assert_eq!(AuditAction::Login.category(), AuditCategory::Authentication);
        assert_eq!(AuditAction::ServerCreate.category(), AuditCategory::Server);
        assert_eq!(AuditAction::TeamCreate.category(), AuditCategory::Team);
    }

    #[test]
    fn test_audit_action_severity() {
        assert_eq!(AuditAction::LoginFailed.severity(), AuditSeverity::Warning);
        assert_eq!(AuditAction::ServerDelete.severity(), AuditSeverity::High);
        assert_eq!(AuditAction::Login.severity(), AuditSeverity::Info);
    }

    #[test]
    fn test_audit_entry_creation() {
        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test User".to_string(),
            team_id: Some("team1".to_string()),
            role: Some("admin".to_string()),
        };

        let target = AuditTarget::Server {
            id: "server1".to_string(),
            host: Some("192.168.1.1".to_string()),
        };

        let entry = AuditEntry::new(AuditAction::ServerConnect, actor, target)
            .with_result(AuditResult::Success)
            .with_client_info(ClientInfo {
                ip_address: Some("10.0.0.1".to_string()),
                user_agent: None,
                device_id: None,
            });

        assert_eq!(entry.action, AuditAction::ServerConnect);
        assert_eq!(entry.result, AuditResult::Success);
        assert_eq!(entry.category(), AuditCategory::Server);
    }

    #[test]
    fn test_audit_logger() {
        let mut logger = AuditLogger::new().with_max_entries(5);

        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        // 添加6条记录，应该只有5条保留
        for i in 0..6 {
            let entry = AuditEntry::new(AuditAction::Login, actor.clone(), AuditTarget::System);
            logger.log(entry);
        }

        assert_eq!(logger.len(), 5);
    }

    #[test]
    fn test_audit_filter() {
        let filter = AuditFilter {
            actions: Some(vec![AuditAction::Login, AuditAction::Logout]),
            result: Some(AuditResult::Success),
            ..Default::default()
        };

        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        let entry = AuditEntry::new(AuditAction::Login, actor.clone(), AuditTarget::System)
            .with_result(AuditResult::Success);

        assert!(filter.matches(&entry));

        let entry2 = AuditEntry::new(AuditAction::ServerCreate, actor, AuditTarget::System)
            .with_result(AuditResult::Success);

        assert!(!filter.matches(&entry2));
    }

    #[test]
    fn test_audit_stats() {
        let mut logger = AuditLogger::new();

        let actor1 = Actor {
            user_id: "user1".to_string(),
            username: "User1".to_string(),
            team_id: None,
            role: None,
        };

        let actor2 = Actor {
            user_id: "user2".to_string(),
            username: "User2".to_string(),
            team_id: None,
            role: None,
        };

        // 添加各种操作
        logger.log(AuditEntry::new(
            AuditAction::Login,
            actor1.clone(),
            AuditTarget::System,
        ));
        logger.log(AuditEntry::new(
            AuditAction::Login,
            actor2.clone(),
            AuditTarget::System,
        ));
        logger.log(AuditEntry::new(
            AuditAction::LoginFailed,
            actor1.clone(),
            AuditTarget::System,
        ));
        logger.log(AuditEntry::new(
            AuditAction::ServerCreate,
            actor1.clone(),
            AuditTarget::System,
        ));

        let stats = logger.generate_stats(None);
        assert_eq!(stats.total_entries, 4);
        assert_eq!(stats.failed_logins, 1);
        assert_eq!(stats.unique_actors, 2);
    }

    #[test]
    fn test_export_csv() {
        let mut logger = AuditLogger::new();

        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: Some("team1".to_string()),
            role: None,
        };

        logger.log(AuditEntry::new(
            AuditAction::Login,
            actor,
            AuditTarget::System,
        ));

        let csv = logger.export_csv(None).unwrap();
        assert!(csv.contains("timestamp,id,action"));
        assert!(csv.contains("用户登录"));
    }

    #[test]
    fn test_detect_anomalies() {
        let mut logger = AuditLogger::new();

        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        // 添加多次登录失败
        for i in 0..6 {
            logger.log(
                AuditEntry::new(AuditAction::LoginFailed, actor.clone(), AuditTarget::System)
                    .with_client_info(ClientInfo {
                        ip_address: Some(format!("10.0.0.{}", i)),
                        user_agent: None,
                        device_id: None,
                    }),
            );
        }

        let anomalies = logger.detect_anomalies("user1");
        assert!(!anomalies.is_empty());
        assert!(anomalies.iter().any(|a| a.contains("登录失败")));
    }

    #[test]
    fn test_audit_target_serialization() {
        let targets = vec![
            AuditTarget::User {
                id: "user1".to_string(),
            },
            AuditTarget::Server {
                id: "srv1".to_string(),
                host: Some("192.168.1.1".to_string()),
            },
            AuditTarget::System,
        ];

        for target in targets {
            let json = serde_json::to_string(&target).unwrap();
            let restored: AuditTarget = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", target), format!("{:?}", restored));
        }
    }

    #[test]
    fn test_user_timeline() {
        let mut logger = AuditLogger::new();

        let actor1 = Actor {
            user_id: "user1".to_string(),
            username: "User1".to_string(),
            team_id: None,
            role: None,
        };

        let actor2 = Actor {
            user_id: "user2".to_string(),
            username: "User2".to_string(),
            team_id: None,
            role: None,
        };

        logger.log(AuditEntry::new(
            AuditAction::Login,
            actor1.clone(),
            AuditTarget::System,
        ));
        logger.log(AuditEntry::new(
            AuditAction::ServerConnect,
            actor1.clone(),
            AuditTarget::System,
        ));
        logger.log(AuditEntry::new(
            AuditAction::Login,
            actor2.clone(),
            AuditTarget::System,
        ));

        let timeline = logger.get_user_timeline("user1", 10);
        assert_eq!(timeline.len(), 2);
        assert!(timeline.iter().all(|e| e.actor.user_id == "user1"));
    }

    #[test]
    fn test_change_record() {
        let change = ChangeRecord {
            field: "name".to_string(),
            old_value: Some("Old Name".to_string()),
            new_value: Some("New Name".to_string()),
        };

        assert_eq!(change.field, "name");
        assert_eq!(change.old_value, Some("Old Name".to_string()));
    }

    // ============ Tamper Protection Tests ============

    #[test]
    fn test_entry_hash_computation() {
        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        let entry = AuditEntry::new(AuditAction::Login, actor, AuditTarget::System);
        let hash = entry.compute_hash();

        // Hash should be 64 hex characters (blake3)
        assert_eq!(hash.len(), 64);
        // Hash should be valid hex
        assert!(hex::decode(&hash).is_ok());
    }

    #[test]
    fn test_entry_sealing() {
        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        let entry = AuditEntry::new(AuditAction::Login, actor, AuditTarget::System);
        let key = b"test_key_for_signing";

        let sealed = entry.seal(None, Some(key));

        // Sealed entry should have hash
        assert!(sealed.entry_hash.is_some());
        // Sealed entry with key should have signature
        assert!(sealed.signature.is_some());
        // Signature should be non-empty
        assert!(!sealed.signature.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_entry_verification_valid() {
        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        let entry = AuditEntry::new(AuditAction::Login, actor, AuditTarget::System);
        let key = b"test_key_for_signing";

        let sealed = entry.seal(None, Some(key));

        // Valid sealed entry should verify (hash check)
        assert!(sealed.verify());
        // Should also verify with key (including signature)
        assert!(sealed.verify_with_key(key));
    }

    #[test]
    fn test_entry_verification_no_hash() {
        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        let entry = AuditEntry::new(AuditAction::Login, actor, AuditTarget::System);

        // Entry without hash should fail verification
        assert!(!entry.verify());
    }

    #[test]
    fn test_tamper_protection_logger() {
        let key = b"test_key_for_protection";
        let mut logger = AuditLogger::new()
            .with_tamper_protection(key)
            .with_max_entries(100);

        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        // Log several entries
        for _ in 0..5 {
            logger.log(AuditEntry::new(
                AuditAction::Login,
                actor.clone(),
                AuditTarget::System,
            ));
        }

        // Logger should be tamper protected
        assert!(logger.is_tamper_protected());

        // All entries should have hashes
        for entry in logger.get_all() {
            assert!(entry.entry_hash.is_some());
        }
    }

    #[test]
    fn test_integrity_verification() {
        let key = b"test_key_for_integrity";
        let mut logger = AuditLogger::new()
            .with_tamper_protection(key)
            .with_max_entries(100);

        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        // Log several entries
        for _ in 0..10 {
            logger.log(AuditEntry::new(
                AuditAction::Login,
                actor.clone(),
                AuditTarget::System,
            ));
        }

        // Verify integrity
        let result = logger.verify_integrity();
        assert!(result.valid);
        assert_eq!(result.total_entries, 10);
        assert!(result.tampered_entries.is_empty());
        assert!(result.broken_chain_at.is_none());
    }

    #[test]
    fn test_integrity_verification_disabled() {
        let mut logger = AuditLogger::new();

        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        logger.log(AuditEntry::new(
            AuditAction::Login,
            actor.clone(),
            AuditTarget::System,
        ));

        // Without tamper protection, verification should report not enabled
        let result = logger.verify_integrity();
        assert!(result.valid); // Returns valid but with warning
        assert!(result.error_message.is_some());
        assert!(result.error_message.unwrap().contains("not enabled"));
    }

    #[test]
    fn test_hash_chain_continuity() {
        let key = b"test_key_for_chain";
        let mut logger = AuditLogger::new()
            .with_tamper_protection(key)
            .with_max_entries(100);

        let actor = Actor {
            user_id: "user1".to_string(),
            username: "Test".to_string(),
            team_id: None,
            role: None,
        };

        // Log several entries
        for _ in 0..5 {
            logger.log(AuditEntry::new(
                AuditAction::Login,
                actor.clone(),
                AuditTarget::System,
            ));
        }

        // Check that entries form a chain (each entry references previous)
        let entries: Vec<_> = logger.get_all();
        for i in 1..entries.len() {
            let current = &entries[i];
            let previous = &entries[i - 1];

            // Current entry's previous_hash should equal previous entry's hash
            assert_eq!(
                current.previous_hash, previous.entry_hash,
                "Hash chain broken at index {}",
                i
            );
        }
    }
}
