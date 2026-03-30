//! Pro版本后端 - 团队管理与审计

use crate::error::LiteError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============ 团队相关类型 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub role: TeamRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl TeamRole {
    pub fn can_manage_members(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    pub fn can_manage_servers(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Member)
    }

    pub fn can_view_audit(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }
}

// ============ 邀请相关类型 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvite {
    pub id: String,
    pub team_id: String,
    pub email: String,
    pub role: TeamRole,
    pub invited_by: String,
    pub expires_at: DateTime<Utc>,
    pub accepted: bool,
    pub created_at: DateTime<Utc>,
}

// ============ 审计日志类型 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub username: String,
    pub action: AuditAction,
    pub target_type: String,
    pub target_id: String,
    pub details: String,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // 用户操作
    UserLogin,
    UserLogout,
    UserInvite,
    UserRemove,
    UserRoleChange,

    // 服务器操作
    ServerAdd,
    ServerUpdate,
    ServerDelete,
    ServerConnect,
    ServerDisconnect,

    // 团队操作
    TeamCreate,
    TeamUpdate,
    TeamDelete,

    // 权限操作
    PermissionGrant,
    PermissionRevoke,
}

impl AuditAction {
    pub fn description(&self) -> &'static str {
        match self {
            AuditAction::UserLogin => "用户登录",
            AuditAction::UserLogout => "用户登出",
            AuditAction::UserInvite => "邀请用户",
            AuditAction::UserRemove => "移除用户",
            AuditAction::UserRoleChange => "角色变更",
            AuditAction::ServerAdd => "添加服务器",
            AuditAction::ServerUpdate => "更新服务器",
            AuditAction::ServerDelete => "删除服务器",
            AuditAction::ServerConnect => "连接服务器",
            AuditAction::ServerDisconnect => "断开连接",
            AuditAction::TeamCreate => "创建团队",
            AuditAction::TeamUpdate => "更新团队",
            AuditAction::TeamDelete => "删除团队",
            AuditAction::PermissionGrant => "授予权限",
            AuditAction::PermissionRevoke => "撤销权限",
        }
    }
}

// ============ 共享服务器类型 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedServer {
    pub id: String,
    pub team_id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub group_id: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============ Pro版本数据库操作 ============

/// Pro版本数据库管理器
pub struct ProDatabase {
    endpoint: Option<String>,
    api_key: Option<String>,
    last_check: std::sync::Mutex<Option<std::time::Instant>>,
}

impl ProDatabase {
    pub fn new() -> Self {
        Self {
            endpoint: std::env::var("EASYSSH_PRO_ENDPOINT").ok(),
            api_key: std::env::var("EASYSSH_PRO_API_KEY").ok(),
            last_check: std::sync::Mutex::new(None),
        }
    }

    /// 检查是否配置了Pro后端
    pub fn is_configured(&self) -> bool {
        self.endpoint.is_some() && self.api_key.is_some()
    }

    /// 检查是否连接到Pro后端
    /// 实际连接检查需要网络请求，这里返回配置状态
    pub fn is_connected(&self) -> bool {
        // 检查最近是否已经验证过连接
        let last = self.last_check.lock().unwrap();
        if let Some(time) = *last {
            // 5分钟内不再重复检查
            if time.elapsed() < std::time::Duration::from_secs(300) {
                return self.is_configured();
            }
        }
        self.is_configured()
    }

    /// 获取Pro后端端点
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    /// 刷新连接状态（异步环境调用）
    pub async fn refresh_connection(&self) -> Result<bool, LiteError> {
        if !self.is_configured() {
            return Ok(false);
        }

        // 更新最后检查时间
        let mut last = self.last_check.lock().unwrap();
        *last = Some(std::time::Instant::now());

        // 实际实现中这里会发送ping请求到Pro后端
        // 目前返回配置状态
        Ok(true)
    }
}

impl Default for ProDatabase {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 团队操作函数 ============

/// 创建团队
pub fn create_team(name: &str, owner_id: &str) -> Result<Team, LiteError> {
    let now = Utc::now();
    Ok(Team {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        owner_id: owner_id.to_string(),
        created_at: now,
        updated_at: now,
    })
}

/// 创建团队成员
pub fn create_team_member(
    team_id: &str,
    user_id: &str,
    username: &str,
    email: &str,
    role: TeamRole,
) -> Result<TeamMember, LiteError> {
    Ok(TeamMember {
        id: Uuid::new_v4().to_string(),
        team_id: team_id.to_string(),
        user_id: user_id.to_string(),
        username: username.to_string(),
        email: email.to_string(),
        role,
        joined_at: Utc::now(),
    })
}

/// 创建审计日志
#[allow(clippy::too_many_arguments)]
pub fn create_audit_log(
    team_id: &str,
    user_id: &str,
    username: &str,
    action: AuditAction,
    target_type: &str,
    target_id: &str,
    details: &str,
    ip_address: Option<&str>,
) -> AuditLog {
    AuditLog {
        id: Uuid::new_v4().to_string(),
        team_id: team_id.to_string(),
        user_id: user_id.to_string(),
        username: username.to_string(),
        action,
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        details: details.to_string(),
        ip_address: ip_address.map(|s| s.to_string()),
        created_at: Utc::now(),
    }
}

// ============ SSO相关类型 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProvider {
    pub id: String,
    pub provider_type: SsoProviderType,
    pub client_id: String,
    pub client_secret: String,
    pub issuer_url: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SsoProviderType {
    Saml,
    Oidc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoSession {
    pub id: String,
    pub user_id: String,
    pub provider: SsoProviderType,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_role_permissions() {
        assert!(TeamRole::Owner.can_manage_members());
        assert!(TeamRole::Admin.can_manage_members());
        assert!(!TeamRole::Member.can_manage_members());
        assert!(!TeamRole::Viewer.can_manage_members());

        assert!(TeamRole::Owner.can_manage_servers());
        assert!(TeamRole::Admin.can_manage_servers());
        assert!(TeamRole::Member.can_manage_servers());
        assert!(!TeamRole::Viewer.can_manage_servers());
    }

    #[test]
    fn test_create_team() {
        let team = create_team("Test Team", "user123").unwrap();
        assert_eq!(team.name, "Test Team");
        assert_eq!(team.owner_id, "user123");
        assert!(!team.id.is_empty());
    }

    #[test]
    fn test_create_audit_log() {
        let log = create_audit_log(
            "team1",
            "user1",
            "testuser",
            AuditAction::ServerConnect,
            "server",
            "srv123",
            "Connected to server",
            Some("192.168.1.1"),
        );
        assert_eq!(log.team_id, "team1");
        assert!(matches!(log.action, AuditAction::ServerConnect));
    }

    #[test]
    fn test_audit_action_description() {
        assert_eq!(AuditAction::UserLogin.description(), "用户登录");
        assert_eq!(AuditAction::ServerAdd.description(), "添加服务器");
    }

    #[test]
    fn test_pro_database_new_without_config() {
        // 确保环境变量未设置
        std::env::remove_var("EASYSSH_PRO_ENDPOINT");
        std::env::remove_var("EASYSSH_PRO_API_KEY");

        let db = ProDatabase::new();
        assert!(!db.is_configured());
        assert!(!db.is_connected());
        assert!(db.endpoint().is_none());
    }

    #[test]
    fn test_pro_database_default() {
        let db = ProDatabase::default();
        assert!(!db.is_configured());
    }
}
