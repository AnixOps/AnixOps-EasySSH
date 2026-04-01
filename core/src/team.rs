//! 团队管理模块 (Pro版本)
//! 提供团队创建、成员管理、邀请系统、资源共享等功能

use crate::error::LiteError;
#[cfg(feature = "audit")]
use crate::audit::{Actor, AuditAction, AuditEntry, AuditLogger, AuditResult, AuditTarget, ClientInfo};
#[cfg(feature = "pro")]
use crate::rbac::{Permission, PermissionContext, Resource, ResourceType, Operation, RbacManager, UserRoleAssignment};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 团队角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    Owner,   // 所有者 - 完整权限
    Admin,   // 管理员 - 管理团队和服务器
    Member,  // 成员 - 管理服务器
    Viewer,  // 观察者 - 只读访问
}

impl Default for TeamRole {
    fn default() -> Self {
        TeamRole::Viewer
    }
}

impl TeamRole {
    /// 获取权限等级（越高权限越大）
    pub fn level(&self) -> u8 {
        match self {
            TeamRole::Owner => 100,
            TeamRole::Admin => 80,
            TeamRole::Member => 50,
            TeamRole::Viewer => 20,
        }
    }

    /// 检查是否可以管理团队
    pub fn can_manage_team(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// 检查是否可以管理成员
    pub fn can_manage_members(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// 检查是否可以管理服务器
    pub fn can_manage_servers(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Member)
    }

    /// 检查是否可以查看审计日志
    pub fn can_view_audit(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// 检查是否可以管理团队设置
    pub fn can_manage_settings(&self) -> bool {
        matches!(self, TeamRole::Owner)
    }

    /// 检查是否可以删除团队
    pub fn can_delete_team(&self) -> bool {
        matches!(self, TeamRole::Owner)
    }

    /// 检查是否可以邀请成员
    pub fn can_invite_members(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// 检查是否可以分享资源
    pub fn can_share_resources(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Member)
    }

    /// 检查是否可以移除成员
    pub fn can_remove_member(&self, target_role: &TeamRole) -> bool {
        self.level() > target_role.level()
    }

    /// 获取角色名称（中文）
    pub fn display_name(&self) -> &'static str {
        match self {
            TeamRole::Owner => "所有者",
            TeamRole::Admin => "管理员",
            TeamRole::Member => "成员",
            TeamRole::Viewer => "观察者",
        }
    }

    /// 转换为RBAC角色ID
    #[cfg(feature = "pro")]
    pub fn to_rbac_role_id(&self) -> &'static str {
        match self {
            TeamRole::Owner => "team_admin",      // Owner maps to team_admin with extra ownership
            TeamRole::Admin => "team_admin",
            TeamRole::Member => "team_member",
            TeamRole::Viewer => "team_viewer",
        }
    }
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// 团队成员
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub role: TeamRole,
    pub status: MemberStatus,
    pub joined_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, String>, // 额外元数据
}

/// 成员状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    Active,
    Inactive,
    Suspended,
    Pending,
}

impl TeamMember {
    /// 创建新成员
    pub fn new(
        team_id: &str,
        user_id: &str,
        username: &str,
        email: &str,
        role: TeamRole,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            role,
            status: MemberStatus::Active,
            joined_at: Utc::now(),
            last_active_at: None,
            metadata: HashMap::new(),
        }
    }

    /// 检查是否有特定权限
    pub fn has_permission(&self, permission: TeamPermission) -> bool {
        match permission {
            TeamPermission::ViewServers => true,
            TeamPermission::ManageServers => self.role.can_manage_servers(),
            TeamPermission::ManageMembers => self.role.can_manage_members(),
            TeamPermission::ViewAudit => self.role.can_view_audit(),
            TeamPermission::ManageTeam => self.role.can_manage_team(),
            TeamPermission::DeleteTeam => self.role.can_delete_team(),
            TeamPermission::InviteMembers => self.role.can_invite_members(),
            TeamPermission::ShareResources => self.role.can_share_resources(),
        }
    }

    /// 标记活跃
    pub fn mark_active(&mut self) {
        self.last_active_at = Some(Utc::now());
    }

    /// 暂停成员
    pub fn suspend(&mut self) {
        self.status = MemberStatus::Suspended;
    }

    /// 激活成员
    pub fn activate(&mut self) {
        self.status = MemberStatus::Active;
    }

    /// 检查是否活跃
    pub fn is_active(&self) -> bool {
        self.status == MemberStatus::Active
    }
}

/// 团队权限
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamPermission {
    ViewServers,
    ManageServers,
    ManageMembers,
    ViewAudit,
    ManageTeam,
    DeleteTeam,
    InviteMembers,
    ShareResources,
}

/// 可分享的资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareableResourceType {
    Server,
    Snippet,
    Key,
    Layout,
    Config,
}

/// 共享资源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedResource {
    pub id: String,
    pub team_id: String,
    pub resource_type: ShareableResourceType,
    pub resource_id: String,
    pub resource_name: String,
    pub shared_by: String,      // 分享者用户ID
    pub shared_at: DateTime<Utc>,
    pub share_type: ShareType,
    pub permissions: Vec<ResourceAccessPermission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// 分享类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareType {
    Full,       // 完全共享（所有团队成员）
    Selective,  // 选择性共享（指定成员）
    ReadOnly,   // 只读共享
}

/// 资源访问权限
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAccessPermission {
    View,
    Edit,
    Execute,    // 执行命令、连接等
    Delete,
    Share,      // 再次分享
}

impl SharedResource {
    /// 创建新的共享资源
    pub fn new(
        team_id: &str,
        resource_type: ShareableResourceType,
        resource_id: &str,
        resource_name: &str,
        shared_by: &str,
        share_type: ShareType,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            resource_type,
            resource_id: resource_id.to_string(),
            resource_name: resource_name.to_string(),
            shared_by: shared_by.to_string(),
            shared_at: Utc::now(),
            share_type,
            permissions: vec![ResourceAccessPermission::View],
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// 设置权限
    pub fn with_permissions(mut self, permissions: Vec<ResourceAccessPermission>) -> Self {
        self.permissions = permissions;
        self
    }

    /// 设置过期时间
    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| Utc::now() > e).unwrap_or(false)
    }

    /// 检查是否有特定权限
    pub fn has_permission(&self, permission: ResourceAccessPermission) -> bool {
        self.permissions.contains(&permission)
    }

    /// 检查用户是否可以访问
    pub fn can_access(&self, user_id: &str, team_members: &[TeamMember]) -> bool {
        // 如果过期，拒绝访问
        if self.is_expired() {
            return false;
        }

        // 检查用户是否是团队成员
        let member = team_members.iter().find(|m| m.user_id == user_id);
        if member.is_none() {
            return false;
        }

        let member = member.unwrap();

        // 所有者和管理员总是有访问权限
        if member.role.can_manage_team() {
            return true;
        }

        // 分享者总是有权限
        if self.shared_by == user_id {
            return true;
        }

        // 根据分享类型检查
        match self.share_type {
            ShareType::Full => true,
            ShareType::Selective => {
                // 检查是否在允许列表中（通过metadata存储）
                self.metadata.get("allowed_members")
                    .map(|allowed| allowed.split(',').any(|id| id == user_id))
                    .unwrap_or(false)
            }
            ShareType::ReadOnly => {
                // 只读模式下，Viewer也可以访问
                true
            }
        }
    }
}

/// 团队
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub settings: TeamSettings,
    #[serde(skip)]
    pub members: Vec<TeamMember>,
    #[serde(skip)]
    pub shared_resources: Vec<SharedResource>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// 团队设置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamSettings {
    pub allow_invite_links: bool,
    pub default_role: TeamRole,
    pub require_approval: bool,
    pub max_members: Option<i32>,
    pub sso_enabled: bool,
    pub sso_provider_id: Option<String>,
    #[serde(default)]
    pub allow_guest_access: bool,        // 允许访客访问
    #[serde(default)]
    pub require_2fa: bool,               // 要求2FA
    #[serde(default)]
    pub auto_expire_shares: Option<i64>, // 自动过期分享（天数）
    #[serde(default)]
    pub notification_settings: NotificationSettings,
}

/// 通知设置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub member_join_email: bool,
    pub member_leave_email: bool,
    pub security_alert_email: bool,
    pub daily_digest_email: bool,
}

impl Team {
    /// 创建新团队
    pub fn new(name: &str, owner_id: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            owner_id: owner_id.to_string(),
            created_at: now,
            updated_at: now,
            settings: TeamSettings::default(),
            members: Vec::new(),
            shared_resources: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// 设置描述
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// 设置自定义设置
    pub fn with_settings(mut self, settings: TeamSettings) -> Self {
        self.settings = settings;
        self
    }

    /// 添加标签
    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// 获取成员数量
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// 获取活跃成员数量
    pub fn active_member_count(&self) -> usize {
        self.members.iter().filter(|m| m.is_active()).count()
    }

    /// 查找成员
    pub fn find_member(&self, user_id: &str) -> Option<&TeamMember> {
        self.members.iter().find(|m| m.user_id == user_id)
    }

    /// 查找成员（可变）
    pub fn find_member_mut(&mut self, user_id: &str) -> Option<&mut TeamMember> {
        self.members.iter_mut().find(|m| m.user_id == user_id)
    }

    /// 获取所有管理员
    pub fn get_admins(&self) -> Vec<&TeamMember> {
        self.members.iter().filter(|m| m.role.can_manage_members()).collect()
    }

    /// 获取所有所有者
    pub fn get_owners(&self) -> Vec<&TeamMember> {
        self.members.iter().filter(|m| matches!(m.role, TeamRole::Owner)).collect()
    }

    /// 更新设置
    pub fn update_settings(&mut self, settings: TeamSettings) {
        self.settings = settings;
        self.updated_at = Utc::now();
    }

    /// 是否达到最大成员限制
    pub fn is_at_capacity(&self) -> bool {
        self.settings.max_members.map(|max| self.members.len() >= max as usize).unwrap_or(false)
    }

    /// 添加共享资源
    pub fn add_shared_resource(&mut self, resource: SharedResource) {
        self.shared_resources.push(resource);
        self.updated_at = Utc::now();
    }

    /// 移除共享资源
    pub fn remove_shared_resource(&mut self, resource_id: &str) -> Option<SharedResource> {
        let pos = self.shared_resources.iter().position(|r| r.id == resource_id)?;
        Some(self.shared_resources.remove(pos))
    }

    /// 查找共享资源
    pub fn find_shared_resource(&self, resource_id: &str) -> Option<&SharedResource> {
        self.shared_resources.iter().find(|r| r.id == resource_id)
    }

    /// 清理过期分享
    pub fn cleanup_expired_shares(&mut self) -> Vec<SharedResource> {
        let expired: Vec<_> = self.shared_resources.iter()
            .filter(|r| r.is_expired())
            .cloned()
            .collect();

        self.shared_resources.retain(|r| !r.is_expired());

        if !expired.is_empty() {
            self.updated_at = Utc::now();
        }

        expired
    }

    /// 获取用户可访问的资源
    pub fn get_accessible_resources(&self, user_id: &str) -> Vec<&SharedResource> {
        self.shared_resources.iter()
            .filter(|r| r.can_access(user_id, &self.members))
            .collect()
    }

    /// 检查用户是否可以访问特定资源
    pub fn can_access_resource(&self, user_id: &str, resource_id: &str) -> bool {
        self.find_shared_resource(resource_id)
            .map(|r| r.can_access(user_id, &self.members))
            .unwrap_or(false)
    }
}

/// 团队邀请
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvite {
    pub id: String,
    pub team_id: String,
    pub email: String,
    pub role: TeamRole,
    pub invited_by: String,
    pub status: InviteStatus,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub invite_code: String,      // 邀请码（用于链接邀请）
    #[serde(default)]
    pub invite_link: Option<String>, // 邀请链接
    #[serde(default)]
    pub custom_message: Option<String>, // 自定义消息
}

/// 邀请状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
    Revoked,
}

impl TeamInvite {
    /// 创建新邀请
    pub fn new(
        team_id: &str,
        email: &str,
        role: TeamRole,
        invited_by: &str,
        expires_hours: i64,
    ) -> Self {
        let now = Utc::now();
        let invite_code = Self::generate_invite_code();
        Self {
            id: Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            email: email.to_string(),
            role,
            invited_by: invited_by.to_string(),
            status: InviteStatus::Pending,
            expires_at: now + chrono::Duration::hours(expires_hours),
            created_at: now,
            accepted_at: None,
            invite_code,
            invite_link: None,
            custom_message: None,
        }
    }

    /// 生成邀请码
    fn generate_invite_code() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::thread_rng();
        (0..8)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// 设置自定义消息
    pub fn with_message(mut self, message: &str) -> Self {
        self.custom_message = Some(message.to_string());
        self
    }

    /// 设置邀请链接
    pub fn with_invite_link(mut self, base_url: &str) -> Self {
        self.invite_link = Some(format!("{}/join/{}?code={}", base_url, self.team_id, self.invite_code));
        self
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// 接受邀请
    pub fn accept(&mut self) {
        self.status = InviteStatus::Accepted;
        self.accepted_at = Some(Utc::now());
    }

    /// 拒绝邀请
    pub fn decline(&mut self) {
        self.status = InviteStatus::Declined;
    }

    /// 撤销邀请
    pub fn revoke(&mut self) {
        self.status = InviteStatus::Revoked;
    }

    /// 标记为过期
    pub fn mark_expired(&mut self) {
        self.status = InviteStatus::Expired;
    }
}

/// 团队统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamStats {
    pub total_members: usize,
    pub active_members: usize,
    pub servers_count: usize,
    pub sessions_today: i64,
    pub pending_invites: usize,
    pub shared_resources: usize,
    pub last_activity_at: Option<DateTime<Utc>>,
}

/// 团队活动记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamActivity {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub activity_type: ActivityType,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// 活动类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    MemberJoined,
    MemberLeft,
    MemberRemoved,
    RoleChanged,
    ServerAdded,
    ServerRemoved,
    ResourceShared,
    ResourceUnshared,
    SettingsChanged,
    InviteSent,
    InviteAccepted,
    InviteRevoked,
}

/// 团队管理器
pub struct TeamManager {
    teams: HashMap<String, Team>,
    invites: HashMap<String, TeamInvite>,
    user_teams: HashMap<String, Vec<String>>, // user_id -> team_ids
    activities: Vec<TeamActivity>,
    #[cfg(feature = "pro")]
    rbac_manager: Option<RbacManager>,
    #[cfg(feature = "audit")]
    audit_logger: Option<AuditLogger>,
}

impl TeamManager {
    /// 创建新的团队管理器
    pub fn new() -> Self {
        Self {
            teams: HashMap::new(),
            invites: HashMap::new(),
            user_teams: HashMap::new(),
            activities: Vec::new(),
            #[cfg(feature = "pro")]
            rbac_manager: None,
            #[cfg(feature = "audit")]
            audit_logger: None,
        }
    }

    /// 启用RBAC
    #[cfg(feature = "pro")]
    pub fn with_rbac(mut self, rbac: RbacManager) -> Self {
        self.rbac_manager = Some(rbac);
        self
    }

    /// 启用审计日志
    #[cfg(feature = "audit")]
    pub fn with_audit(mut self, audit: AuditLogger) -> Self {
        self.audit_logger = Some(audit);
        self
    }

    /// 创建团队
    pub fn create_team(
        &mut self,
        name: &str,
        owner_id: &str,
        owner_name: Option<&str>,
    ) -> Result<Team, LiteError> {
        let mut team = Team::new(name, owner_id);
        let id = team.id.clone();

        // Add owner as a team member with Owner role
        let owner_member = TeamMember::new(
            &id,
            owner_id,
            owner_name.unwrap_or(owner_id),
            "",       // Empty email initially
            TeamRole::Owner,
        );
        team.members.push(owner_member);

        self.teams.insert(id.clone(), team.clone());
        self.add_user_to_team(owner_id, &id);

        // 记录活动
        self.record_activity(&id, owner_id, ActivityType::MemberJoined, "创建了团队");

        // 记录审计日志
        #[cfg(feature = "audit")]
        self.log_audit(AuditAction::TeamCreate, owner_id, &id, AuditTarget::Team { id: id.clone() });

        Ok(team)
    }

    /// 获取团队
    pub fn get_team(&self, team_id: &str) -> Option<&Team> {
        self.teams.get(team_id)
    }

    /// 获取团队（可变）
    pub fn get_team_mut(&mut self, team_id: &str) -> Option<&mut Team> {
        self.teams.get_mut(team_id)
    }

    /// 更新团队
    pub fn update_team(&mut self, team: Team, updated_by: &str) -> Result<(), LiteError> {
        if self.teams.contains_key(&team.id) {
            let team_id = team.id.clone();
            self.teams.insert(team_id.clone(), team);

            // 记录审计日志
            #[cfg(feature = "audit")]
            self.log_audit(AuditAction::TeamUpdate, updated_by, &team_id, AuditTarget::Team { id: team_id.clone() });

            Ok(())
        } else {
            Err(LiteError::Team(format!("Team {} not found", team.id)))
        }
    }

    /// 删除团队
    pub fn delete_team(&mut self, team_id: &str, requester_id: &str) -> Result<(), LiteError> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        if team.owner_id != requester_id {
            return Err(LiteError::Team("Only owner can delete team".to_string()));
        }

        // 收集所有成员ID
        let member_ids: Vec<String> = team.members.iter()
            .map(|m| m.user_id.clone())
            .collect();

        let team_id_owned = team_id.to_string();
        self.teams.remove(team_id);

        // 移除所有成员的关联
        for user_id in member_ids {
            self.remove_user_from_team(&user_id, &team_id_owned);
        }

        // 清理相关邀请
        self.invites.retain(|_, invite| invite.team_id != team_id_owned);

        // 记录审计日志
        #[cfg(feature = "audit")]
        self.log_audit(AuditAction::TeamDelete, requester_id, &team_id_owned, AuditTarget::Team { id: team_id_owned.clone() });

        Ok(())
    }

    /// 列出用户的所有团队
    pub fn list_user_teams(&self, user_id: &str) -> Vec<&Team> {
        self.user_teams
            .get(user_id)
            .map(|team_ids| {
                team_ids.iter()
                    .filter_map(|id| self.teams.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 列出所有团队
    pub fn list_all_teams(&self) -> Vec<&Team> {
        self.teams.values().collect()
    }

    /// 搜索团队
    pub fn search_teams(&self, query: &str) -> Vec<&Team> {
        let query_lower = query.to_lowercase();
        self.teams.values()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower) ||
                t.description.as_ref().map(|d| d.to_lowercase().contains(&query_lower)).unwrap_or(false) ||
                t.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// 邀请成员
    pub fn invite_member(
        &mut self,
        team_id: &str,
        email: &str,
        role: TeamRole,
        invited_by: &str,
        custom_message: Option<&str>,
    ) -> Result<TeamInvite, LiteError> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        // 检查权限
        if let Some(member) = team.find_member(invited_by) {
            if !member.role.can_invite_members() {
                return Err(LiteError::Team("No permission to invite members".to_string()));
            }
        } else {
            return Err(LiteError::Team("Not a team member".to_string()));
        }

        // 检查容量
        if team.is_at_capacity() {
            return Err(LiteError::Team("Team is at capacity".to_string()));
        }

        // 检查是否已有待处理邀请，如果有则撤销旧邀请
        let existing_invite_ids: Vec<String> = self.invites.values()
            .filter(|i| i.team_id == team_id && i.email == email && i.status == InviteStatus::Pending)
            .map(|i| i.id.clone())
            .collect();

        for invite_id in existing_invite_ids {
            if let Some(invite) = self.invites.get_mut(&invite_id) {
                invite.revoke();
            }
        }

        // 检查是否已是成员
        if team.members.iter().any(|m| m.email == email) {
            return Err(LiteError::Team("Already a member".to_string()));
        }

        let mut invite = TeamInvite::new(team_id, email, role, invited_by, 168); // 7天过期

        // 设置自定义消息
        if let Some(msg) = custom_message {
            invite.custom_message = Some(msg.to_string());
        }

        // 生成邀请链接
        invite.invite_link = Some(format!("easyssh://join/{}/{}", team_id, invite.invite_code));

        let id = invite.id.clone();
        self.invites.insert(id.clone(), invite.clone());

        // 记录活动
        self.record_activity(team_id, invited_by, ActivityType::InviteSent, &format!("邀请了 {}", email));

        // 记录审计日志
        #[cfg(feature = "audit")]
        self.log_audit(AuditAction::MemberInvite, invited_by, team_id, AuditTarget::User { id: email.to_string() });

        Ok(invite)
    }

    /// 通过邀请码加入团队
    pub fn join_by_invite_code(
        &mut self,
        invite_code: &str,
        user_id: &str,
        username: &str,
        email: &str,
    ) -> Result<TeamMember, LiteError> {
        let invite = self.invites.values_mut()
            .find(|i| i.invite_code == invite_code && i.status == InviteStatus::Pending)
            .ok_or_else(|| LiteError::Team("Invalid or expired invite code".to_string()))?;

        if invite.is_expired() {
            invite.mark_expired();
            return Err(LiteError::Team("Invite has expired".to_string()));
        }

        let team_id = invite.team_id.clone();
        let role = invite.role;

        invite.accept();

        // 添加成员
        let member = self.add_member_internal(&team_id, user_id, username, email, role)?;

        // 记录活动
        self.record_activity(&team_id, user_id, ActivityType::InviteAccepted, &format!("通过邀请码加入"));

        Ok(member)
    }

    /// 接受邀请（通过邀请ID）
    pub fn accept_invite(
        &mut self,
        invite_id: &str,
        user_id: &str,
        username: &str,
        email: &str,
    ) -> Result<TeamMember, LiteError> {
        let invite = self.invites.get_mut(invite_id)
            .ok_or_else(|| LiteError::Team(format!("Invite {} not found", invite_id)))?;

        if invite.status != InviteStatus::Pending {
            return Err(LiteError::Team("Invite is not pending".to_string()));
        }

        if invite.is_expired() {
            invite.mark_expired();
            return Err(LiteError::Team("Invite has expired".to_string()));
        }

        // 验证邮箱匹配
        if invite.email != email {
            return Err(LiteError::Team("Email does not match invitation".to_string()));
        }

        // 克隆需要的字段
        let team_id = invite.team_id.clone();
        let role = invite.role;

        invite.accept();

        // 添加成员
        let member = self.add_member_internal(&team_id, user_id, username, email, role)?;

        // 记录审计日志
        #[cfg(feature = "audit")]
        self.log_audit(AuditAction::MemberJoin, user_id, &team_id, AuditTarget::Team { id: team_id.clone() });

        Ok(member)
    }

    /// 内部方法：添加成员
    fn add_member_internal(
        &mut self,
        team_id: &str,
        user_id: &str,
        username: &str,
        email: &str,
        role: TeamRole,
    ) -> Result<TeamMember, LiteError> {
        let team = self.teams.get_mut(team_id)
            .ok_or_else(|| LiteError::Team("Team not found".to_string()))?;

        // 检查容量
        if team.is_at_capacity() {
            return Err(LiteError::Team("Team is at capacity".to_string()));
        }

        // 检查是否已是成员
        if team.find_member(user_id).is_some() {
            return Err(LiteError::Team("Already a team member".to_string()));
        }

        // 创建成员
        let member = TeamMember::new(
            team_id,
            user_id,
            username,
            email,
            role,
        );

        team.members.push(member.clone());
        team.updated_at = Utc::now();

        self.add_user_to_team(user_id, team_id);

        // 记录活动
        self.record_activity(team_id, user_id, ActivityType::MemberJoined, &format!("{} 加入了团队", username));

        // 设置RBAC角色
        #[cfg(feature = "pro")]
        if let Some(ref mut rbac) = self.rbac_manager {
            let assignment = UserRoleAssignment::new(user_id, Some(team_id), role.to_rbac_role_id(), "system");
            let _ = rbac.assign_role(assignment);
        }

        Ok(member)
    }

    /// 拒绝邀请
    pub fn decline_invite(&mut self, invite_id: &str) -> Result<(), LiteError> {
        let invite = self.invites.get_mut(invite_id)
            .ok_or_else(|| LiteError::Team(format!("Invite {} not found", invite_id)))?;

        if invite.status != InviteStatus::Pending {
            return Err(LiteError::Team("Invite is not pending".to_string()));
        }

        invite.decline();
        Ok(())
    }

    /// 撤销邀请
    pub fn revoke_invite(&mut self, invite_id: &str, revoked_by: &str) -> Result<(), LiteError> {
        let invite = self.invites.get_mut(invite_id)
            .ok_or_else(|| LiteError::Team(format!("Invite {} not found", invite_id)))?;

        let team = self.teams.get(&invite.team_id)
            .ok_or_else(|| LiteError::Team("Team not found".to_string()))?;

        // 检查权限
        if let Some(member) = team.find_member(revoked_by) {
            if !member.role.can_manage_members() {
                return Err(LiteError::Team("No permission to revoke invite".to_string()));
            }
        } else {
            return Err(LiteError::Team("Not a team member".to_string()));
        }

        if invite.status != InviteStatus::Pending {
            return Err(LiteError::Team("Invite is not pending".to_string()));
        }

        let team_id = invite.team_id.clone();
        let email = invite.email.clone();
        invite.revoke();

        // 记录活动
        self.record_activity(&team_id, revoked_by, ActivityType::InviteRevoked, &format!("撤销了对 {} 的邀请", email));

        Ok(())
    }

    /// 获取团队的待处理邀请
    pub fn get_pending_invites(&self, team_id: &str) -> Vec<&TeamInvite> {
        self.invites.values()
            .filter(|i| i.team_id == team_id && i.status == InviteStatus::Pending)
            .collect()
    }

    /// 获取用户的待处理邀请
    pub fn get_user_pending_invites(&self, email: &str) -> Vec<&TeamInvite> {
        self.invites.values()
            .filter(|i| i.email == email && i.status == InviteStatus::Pending && !i.is_expired())
            .collect()
    }

    /// 移除成员
    pub fn remove_member(
        &mut self,
        team_id: &str,
        member_user_id: &str,
        removed_by: &str,
    ) -> Result<(), LiteError> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        // 检查权限
        let remover_role = team.find_member(removed_by).map(|m| m.role);
        let target_role = team.find_member(member_user_id).map(|m| m.role);

        match (remover_role, target_role) {
            (Some(remover), Some(target)) => {
                if !remover.can_remove_member(&target) {
                    return Err(LiteError::Team("Cannot remove member with higher or equal role".to_string()));
                }
            }
            _ => return Err(LiteError::Team("Member not found".to_string())),
        }

        // 不能移除所有者
        if team.owner_id == member_user_id {
            return Err(LiteError::Team("Cannot remove team owner".to_string()));
        }

        let team = self.teams.get_mut(team_id).unwrap();
        let member_name = team.find_member(member_user_id).map(|m| m.username.clone()).unwrap_or_default();
        team.members.retain(|m| m.user_id != member_user_id);
        team.updated_at = Utc::now();

        self.remove_user_from_team(member_user_id, team_id);

        // 记录活动
        self.record_activity(team_id, removed_by, ActivityType::MemberRemoved, &format!("移除了成员 {}", member_name));

        // 记录审计日志
        #[cfg(feature = "audit")]
        self.log_audit(AuditAction::MemberRemove, removed_by, team_id, AuditTarget::User { id: member_user_id.to_string() });

        Ok(())
    }

    /// 更改成员角色
    pub fn change_member_role(
        &mut self,
        team_id: &str,
        member_user_id: &str,
        new_role: TeamRole,
        changed_by: &str,
    ) -> Result<(), LiteError> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        // 检查权限
        if let Some(changer) = team.find_member(changed_by) {
            if !changer.role.can_manage_members() {
                return Err(LiteError::Team("No permission to change roles".to_string()));
            }
            if changer.role.level() <= new_role.level() {
                return Err(LiteError::Team("Cannot assign role higher or equal to yours".to_string()));
            }
        } else {
            return Err(LiteError::Team("Not a team member".to_string()));
        }

        // 不能更改所有者角色
        if team.owner_id == member_user_id {
            return Err(LiteError::Team("Cannot change owner role".to_string()));
        }

        let team = self.teams.get_mut(team_id).unwrap();
        let member_info = team.find_member_mut(member_user_id).map(|member| {
            let old_role = member.role;
            member.role = new_role;
            (member.username.clone(), old_role)
        });

        if let Some((username, old_role)) = member_info {
            team.updated_at = Utc::now();

            // 记录活动
            self.record_activity(team_id, changed_by, ActivityType::RoleChanged,
                &format!("将 {} 的角色从 {:?} 更改为 {:?}", username, old_role, new_role));

            // 记录审计日志
            #[cfg(feature = "audit")]
            self.log_audit(AuditAction::MemberRoleChange, changed_by, team_id, AuditTarget::User { id: member_user_id.to_string() });

            Ok(())
        } else {
            Err(LiteError::Team("Member not found".to_string()))
        }
    }

    /// 成员离开团队
    pub fn leave_team(&mut self, team_id: &str, user_id: &str) -> Result<(), LiteError> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        // 所有者不能离开，必须先转让所有权
        if team.owner_id == user_id {
            return Err(LiteError::Team("Owner must transfer ownership before leaving".to_string()));
        }

        let member_name = team.find_member(user_id).map(|m| m.username.clone()).unwrap_or_default();

        let team = self.teams.get_mut(team_id).unwrap();
        team.members.retain(|m| m.user_id != user_id);
        team.updated_at = Utc::now();

        self.remove_user_from_team(user_id, team_id);

        // 记录活动
        self.record_activity(team_id, user_id, ActivityType::MemberLeft, &format!("{} 离开了团队", member_name));

        Ok(())
    }

    /// 转让所有权
    pub fn transfer_ownership(
        &mut self,
        team_id: &str,
        new_owner_id: &str,
        current_owner_id: &str,
    ) -> Result<(), LiteError> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        // 验证当前所有者
        if team.owner_id != current_owner_id {
            return Err(LiteError::Team("Only owner can transfer ownership".to_string()));
        }

        // 验证新所有者存在
        if team.find_member(new_owner_id).is_none() {
            return Err(LiteError::Team("New owner must be a team member".to_string()));
        }

        let team = self.teams.get_mut(team_id).unwrap();

        // 更新原所有者为Admin
        if let Some(old_owner) = team.find_member_mut(current_owner_id) {
            old_owner.role = TeamRole::Admin;
        }

        // 更新新所有者为Owner
        if let Some(new_owner) = team.find_member_mut(new_owner_id) {
            new_owner.role = TeamRole::Owner;
        }

        team.owner_id = new_owner_id.to_string();
        team.updated_at = Utc::now();

        // 记录活动
        self.record_activity(team_id, current_owner_id, ActivityType::RoleChanged,
            &format!("将所有权转让给 {}", new_owner_id));

        Ok(())
    }

    /// 标记成员活跃
    pub fn mark_member_active(&mut self, team_id: &str, user_id: &str) {
        if let Some(team) = self.teams.get_mut(team_id) {
            if let Some(member) = team.find_member_mut(user_id) {
                member.mark_active();
            }
        }
    }

    /// 分享资源
    pub fn share_resource(
        &mut self,
        team_id: &str,
        shared_by: &str,
        resource_type: ShareableResourceType,
        resource_id: &str,
        resource_name: &str,
        share_type: ShareType,
        permissions: Vec<ResourceAccessPermission>,
    ) -> Result<SharedResource, LiteError> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        // 检查权限
        if let Some(member) = team.find_member(shared_by) {
            if !member.role.can_share_resources() {
                return Err(LiteError::Team("No permission to share resources".to_string()));
            }
        } else {
            return Err(LiteError::Team("Not a team member".to_string()));
        }

        let resource = SharedResource::new(
            team_id,
            resource_type,
            resource_id,
            resource_name,
            shared_by,
            share_type,
        ).with_permissions(permissions);

        let _resource_id_clone = resource.id.clone();

        let team = self.teams.get_mut(team_id).unwrap();
        team.add_shared_resource(resource.clone());

        // 记录活动
        self.record_activity(team_id, shared_by, ActivityType::ResourceShared,
            &format!("分享了 {:?}: {}", resource_type, resource_name));

        Ok(resource)
    }

    /// 取消分享资源
    pub fn unshare_resource(
        &mut self,
        team_id: &str,
        resource_id: &str,
        unshared_by: &str,
    ) -> Result<SharedResource, LiteError> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        // 检查权限：资源所有者或团队管理员
        let resource = team.find_shared_resource(resource_id)
            .ok_or_else(|| LiteError::Team("Resource not found".to_string()))?;

        let can_unshare = if let Some(member) = team.find_member(unshared_by) {
            resource.shared_by == unshared_by || member.role.can_manage_team()
        } else {
            false
        };

        if !can_unshare {
            return Err(LiteError::Team("No permission to unshare this resource".to_string()));
        }

        let resource_name = resource.resource_name.clone();
        let resource_type = resource.resource_type;

        let team = self.teams.get_mut(team_id).unwrap();
        let resource = team.remove_shared_resource(resource_id)
            .ok_or_else(|| LiteError::Team("Resource not found".to_string()))?;

        // 记录活动
        self.record_activity(team_id, unshared_by, ActivityType::ResourceUnshared,
            &format!("取消了 {:?}: {} 的分享", resource_type, resource_name));

        Ok(resource)
    }

    /// 获取用户可访问的资源
    pub fn get_user_accessible_resources(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<Vec<&SharedResource>, LiteError> {
        let team = self.teams.get(team_id)
            .ok_or_else(|| LiteError::Team(format!("Team {} not found", team_id)))?;

        Ok(team.get_accessible_resources(user_id))
    }

    /// 获取团队统计
    pub fn get_team_stats(&self, team_id: &str) -> Option<TeamStats> {
        let team = self.teams.get(team_id)?;

        let active_members = team.members.iter()
            .filter(|m| m.status == MemberStatus::Active)
            .count();

        let pending_invites = self.get_pending_invites(team_id).len();

        // 获取最后活动时间
        let last_activity = team.members.iter()
            .filter_map(|m| m.last_active_at)
            .max();

        Some(TeamStats {
            total_members: team.members.len(),
            active_members,
            servers_count: team.shared_resources.iter().filter(|r| matches!(r.resource_type, ShareableResourceType::Server)).count(),
            sessions_today: 0, // 需要与审计模块集成
            pending_invites,
            shared_resources: team.shared_resources.len(),
            last_activity_at: last_activity,
        })
    }

    /// 检查用户是否是团队成员
    pub fn is_team_member(&self, team_id: &str, user_id: &str) -> bool {
        self.teams.get(team_id)
            .map(|t| t.find_member(user_id).is_some())
            .unwrap_or(false)
    }

    /// 获取用户在团队中的角色
    pub fn get_member_role(&self, team_id: &str, user_id: &str) -> Option<TeamRole> {
        self.teams.get(team_id)
            .and_then(|t| t.find_member(user_id).map(|m| m.role))
    }

    /// 检查用户是否有特定权限
    pub fn check_permission(
        &self,
        team_id: &str,
        user_id: &str,
        permission: TeamPermission,
    ) -> bool {
        self.teams.get(team_id)
            .and_then(|t| t.find_member(user_id))
            .map(|m| m.has_permission(permission))
            .unwrap_or(false)
    }

    /// 清理过期邀请
    pub fn cleanup_expired_invites(&mut self) -> Vec<TeamInvite> {
        let expired: Vec<_> = self.invites.values_mut()
            .filter(|i| i.status == InviteStatus::Pending && i.is_expired())
            .map(|i| {
                i.mark_expired();
                i.clone()
            })
            .collect();

        expired
    }

    /// 清理所有过期内容（邀请、分享等）
    pub fn cleanup_all_expired(&mut self) -> CleanupResult {
        let expired_invites = self.cleanup_expired_invites();

        let mut expired_shares = Vec::new();
        for team in self.teams.values_mut() {
            let expired = team.cleanup_expired_shares();
            expired_shares.extend(expired);
        }

        CleanupResult {
            expired_invites,
            expired_shares,
        }
    }

    /// 记录活动
    fn record_activity(&mut self, team_id: &str, user_id: &str, activity_type: ActivityType, description: &str) {
        let activity = TeamActivity {
            id: Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            user_id: user_id.to_string(),
            activity_type,
            description: description.to_string(),
            created_at: Utc::now(),
            metadata: None,
        };
        self.activities.push(activity);
    }

    /// 获取团队活动
    pub fn get_team_activities(&self, team_id: &str, limit: usize) -> Vec<&TeamActivity> {
        self.activities.iter()
            .filter(|a| a.team_id == team_id)
            .rev()
            .take(limit)
            .collect()
    }

    /// 获取用户活动
    pub fn get_user_activities(&self, user_id: &str, limit: usize) -> Vec<&TeamActivity> {
        self.activities.iter()
            .filter(|a| a.user_id == user_id)
            .rev()
            .take(limit)
            .collect()
    }

    /// 记录审计日志（内部方法）
    #[cfg(feature = "audit")]
    fn log_audit(&mut self, action: AuditAction, actor_id: &str, team_id: &str, target: AuditTarget) {
        if let Some(ref mut audit) = self.audit_logger {
            let actor = Actor {
                user_id: actor_id.to_string(),
                username: actor_id.to_string(), // 简化处理，实际应从用户信息获取
                team_id: Some(team_id.to_string()),
                role: None,
            };

            let entry = AuditEntry::new(action, actor, target)
                .with_result(AuditResult::Success);

            audit.log(entry);
        }
    }

    // 辅助方法
    fn add_user_to_team(&mut self, user_id: &str, team_id: &str) {
        self.user_teams
            .entry(user_id.to_string())
            .or_default()
            .push(team_id.to_string());
    }

    fn remove_user_from_team(&mut self, user_id: &str, team_id: &str) {
        if let Some(teams) = self.user_teams.get_mut(user_id) {
            teams.retain(|id| id != team_id);
        }
    }
}

impl Default for TeamManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 清理结果
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub expired_invites: Vec<TeamInvite>,
    pub expired_shares: Vec<SharedResource>,
}

/// 团队操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamOperationResult {
    pub success: bool,
    pub message: String,
    pub team_id: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl TeamOperationResult {
    pub fn success(team_id: impl Into<String>) -> Self {
        Self {
            success: true,
            message: "Success".to_string(),
            team_id: Some(team_id.into()),
            data: None,
        }
    }

    pub fn success_with_data(team_id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: "Success".to_string(),
            team_id: Some(team_id.into()),
            data: Some(data),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            team_id: None,
            data: None,
        }
    }
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

        assert!(TeamRole::Owner.can_delete_team());
        assert!(!TeamRole::Admin.can_delete_team());

        assert!(TeamRole::Owner.can_share_resources());
        assert!(TeamRole::Admin.can_share_resources());
        assert!(TeamRole::Member.can_share_resources());
        assert!(!TeamRole::Viewer.can_share_resources());
    }

    #[test]
    fn test_team_role_hierarchy() {
        assert!(TeamRole::Owner.level() > TeamRole::Admin.level());
        assert!(TeamRole::Admin.level() > TeamRole::Member.level());
        assert!(TeamRole::Member.level() > TeamRole::Viewer.level());
    }

    #[test]
    fn test_can_remove_member() {
        assert!(TeamRole::Owner.can_remove_member(&TeamRole::Admin));
        assert!(TeamRole::Owner.can_remove_member(&TeamRole::Member));
        assert!(TeamRole::Admin.can_remove_member(&TeamRole::Member));
        assert!(!TeamRole::Member.can_remove_member(&TeamRole::Member));
        assert!(!TeamRole::Admin.can_remove_member(&TeamRole::Owner));
    }

    #[test]
    fn test_create_team() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "user1", Some("User 1")).unwrap();

        assert_eq!(team.name, "Test Team");
        assert_eq!(team.owner_id, "user1");

        // 验证可以获取
        let retrieved = manager.get_team(&team.id).unwrap();
        assert_eq!(retrieved.name, "Test Team");

        // 验证所有者是成员
        assert!(manager.is_team_member(&team.id, "user1"));
        assert_eq!(manager.get_member_role(&team.id, "user1"), Some(TeamRole::Owner));
    }

    #[test]
    fn test_invite_and_accept() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 邀请成员
        let invite = manager.invite_member(&team.id, "test@example.com", TeamRole::Member, "owner1", None).unwrap();
        assert_eq!(invite.email, "test@example.com");
        assert_eq!(invite.role, TeamRole::Member);
        assert!(!invite.invite_code.is_empty());
        assert!(invite.invite_link.is_some());

        // 通过邮箱接受邀请
        let member = manager.accept_invite(&invite.id, "user2", "TestUser", "test@example.com").unwrap();
        assert_eq!(member.email, "test@example.com");
        assert_eq!(member.role, TeamRole::Member);
        assert_eq!(member.team_id, team.id);

        // 验证是团队成员
        assert!(manager.is_team_member(&team.id, "user2"));
    }

    #[test]
    fn test_join_by_invite_code() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 邀请成员
        let invite = manager.invite_member(&team.id, "test@example.com", TeamRole::Member, "owner1", None).unwrap();
        let invite_code = invite.invite_code.clone();

        // 通过邀请码加入
        let member = manager.join_by_invite_code(&invite_code, "user2", "TestUser", "test@example.com").unwrap();
        assert_eq!(member.role, TeamRole::Member);
        assert!(manager.is_team_member(&team.id, "user2"));
    }

    #[test]
    fn test_remove_member() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 添加成员
        let invite = manager.invite_member(&team.id, "member@test.com", TeamRole::Member, "owner1", None).unwrap();
        manager.accept_invite(&invite.id, "member1", "Member", "member@test.com").unwrap();

        // 所有者可以移除成员
        assert!(manager.remove_member(&team.id, "member1", "owner1").is_ok());
        assert!(!manager.is_team_member(&team.id, "member1"));
    }

    #[test]
    fn test_leave_team() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 添加成员
        let invite = manager.invite_member(&team.id, "member@test.com", TeamRole::Member, "owner1", None).unwrap();
        manager.accept_invite(&invite.id, "member1", "Member", "member@test.com").unwrap();

        // 成员可以离开
        assert!(manager.leave_team(&team.id, "member1").is_ok());
        assert!(!manager.is_team_member(&team.id, "member1"));

        // 所有者不能离开
        let result = manager.leave_team(&team.id, "owner1");
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_remove_owner() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 不能移除所有者
        let result = manager.remove_member(&team.id, "owner1", "owner1");
        assert!(result.is_err());
    }

    #[test]
    fn test_change_role() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 添加成员
        let invite = manager.invite_member(&team.id, "member@test.com", TeamRole::Member, "owner1", None).unwrap();
        manager.accept_invite(&invite.id, "member1", "Member", "member@test.com").unwrap();

        // 提升为管理员
        assert!(manager.change_member_role(&team.id, "member1", TeamRole::Admin, "owner1").is_ok());

        let team = manager.get_team(&team.id).unwrap();
        let member = team.find_member("member1").unwrap();
        assert_eq!(member.role, TeamRole::Admin);
    }

    #[test]
    fn test_transfer_ownership() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 添加管理员
        let invite = manager.invite_member(&team.id, "admin@test.com", TeamRole::Admin, "owner1", None).unwrap();
        manager.accept_invite(&invite.id, "admin1", "Admin", "admin@test.com").unwrap();

        // 转让所有权
        assert!(manager.transfer_ownership(&team.id, "admin1", "owner1").is_ok());

        let team = manager.get_team(&team.id).unwrap();
        assert_eq!(team.owner_id, "admin1");

        // 原所有者变为Admin
        assert_eq!(team.find_member("owner1").unwrap().role, TeamRole::Admin);
        // 新所有者
        assert_eq!(team.find_member("admin1").unwrap().role, TeamRole::Owner);
    }

    #[test]
    fn test_invite_expiration() {
        let mut invite = TeamInvite::new("team1", "test@example.com", TeamRole::Member, "owner1", 1);

        // 未过期
        assert!(!invite.is_expired());
        assert_eq!(invite.status, InviteStatus::Pending);

        // 模拟过期（手动设置过期时间）
        invite.expires_at = Utc::now() - chrono::Duration::hours(1);
        assert!(invite.is_expired());
    }

    #[test]
    fn test_team_settings() {
        let settings = TeamSettings {
            allow_invite_links: true,
            default_role: TeamRole::Member,
            require_approval: false,
            max_members: Some(10),
            sso_enabled: false,
            sso_provider_id: None,
            allow_guest_access: false,
            require_2fa: true,
            auto_expire_shares: Some(30),
            notification_settings: NotificationSettings::default(),
        };

        let mut team = Team::new("Test", "owner1").with_settings(settings);
        assert_eq!(team.settings.max_members, Some(10));
        assert!(team.settings.allow_invite_links);
        assert!(team.settings.require_2fa);

        team.members.push(TeamMember::new(&team.id, "u1", "User", "u@test.com", TeamRole::Member));
        assert!(!team.is_at_capacity());

        for i in 2..=10 {
            team.members.push(TeamMember::new(&team.id, &format!("u{}", i), "User", &format!("u{}@test.com", i), TeamRole::Member));
        }
        assert!(team.is_at_capacity());
    }

    #[test]
    fn test_list_user_teams() {
        let mut manager = TeamManager::new();
        let team1 = manager.create_team("Team 1", "user1", Some("User 1")).unwrap();
        let team2 = manager.create_team("Team 2", "user1", Some("User 1")).unwrap();

        let user_teams = manager.list_user_teams("user1");
        assert_eq!(user_teams.len(), 2);
        assert!(user_teams.iter().any(|t| t.id == team1.id));
        assert!(user_teams.iter().any(|t| t.id == team2.id));
    }

    #[test]
    fn test_permission_check() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 邀请成员
        let invite = manager.invite_member(&team.id, "admin@test.com", TeamRole::Admin, "owner1", None).unwrap();
        manager.accept_invite(&invite.id, "admin1", "Admin", "admin@test.com").unwrap();

        let invite2 = manager.invite_member(&team.id, "viewer@test.com", TeamRole::Viewer, "admin1", None).unwrap();
        manager.accept_invite(&invite2.id, "viewer1", "Viewer", "viewer@test.com").unwrap();

        // 检查权限
        assert!(manager.check_permission(&team.id, "owner1", TeamPermission::ManageTeam));
        assert!(manager.check_permission(&team.id, "admin1", TeamPermission::ManageMembers));
        assert!(manager.check_permission(&team.id, "viewer1", TeamPermission::ViewServers));
        assert!(!manager.check_permission(&team.id, "viewer1", TeamPermission::ManageServers));
        assert!(!manager.check_permission(&team.id, "viewer1", TeamPermission::ShareResources));
    }

    #[test]
    fn test_team_serialization() {
        let team = Team::new("Test Team", "owner1")
            .with_description("A test team")
            .with_tags(vec!["dev", "ops"]);

        let json = serde_json::to_string(&team).unwrap();
        assert!(json.contains("Test Team"));
        assert!(json.contains("owner1"));
        assert!(json.contains("dev"));
    }

    #[test]
    fn test_team_member_permissions() {
        let member = TeamMember::new("team1", "user1", "User", "user@test.com", TeamRole::Member);

        assert!(member.has_permission(TeamPermission::ViewServers));
        assert!(member.has_permission(TeamPermission::ManageServers));
        assert!(!member.has_permission(TeamPermission::ManageMembers));
        assert!(!member.has_permission(TeamPermission::DeleteTeam));
        assert!(member.has_permission(TeamPermission::ShareResources));

        let admin = TeamMember::new("team1", "admin1", "Admin", "admin@test.com", TeamRole::Admin);
        assert!(admin.has_permission(TeamPermission::ManageMembers));
        assert!(admin.has_permission(TeamPermission::ViewAudit));
    }

    #[test]
    fn test_shared_resource() {
        let resource = SharedResource::new(
            "team1",
            ShareableResourceType::Server,
            "server1",
            "Production Server",
            "user1",
            ShareType::Full,
        ).with_permissions(vec![
            ResourceAccessPermission::View,
            ResourceAccessPermission::Edit,
            ResourceAccessPermission::Execute,
        ]);

        assert_eq!(resource.resource_name, "Production Server");
        assert!(resource.has_permission(ResourceAccessPermission::View));
        assert!(resource.has_permission(ResourceAccessPermission::Edit));
        assert!(!resource.has_permission(ResourceAccessPermission::Delete));
        assert!(!resource.is_expired());
    }

    #[test]
    fn test_shared_resource_with_expiry() {
        let resource = SharedResource::new(
            "team1",
            ShareableResourceType::Snippet,
            "snippet1",
            "Deploy Script",
            "user1",
            ShareType::ReadOnly,
        ).with_expiry(Utc::now() - chrono::Duration::hours(1)); // 已过期

        assert!(resource.is_expired());

        // 模拟团队成员
        let members = vec![
            TeamMember::new("team1", "user1", "Owner", "owner@test.com", TeamRole::Owner),
            TeamMember::new("team1", "user2", "Member", "member@test.com", TeamRole::Member),
        ];

        // 过期资源不能被访问
        assert!(!resource.can_access("user2", &members));
    }

    #[test]
    fn test_share_and_unshare_resource() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 分享资源
        let resource = manager.share_resource(
            &team.id,
            "owner1",
            ShareableResourceType::Server,
            "server1",
            "Production",
            ShareType::Full,
            vec![ResourceAccessPermission::View, ResourceAccessPermission::Edit],
        ).unwrap();

        let team_id = team.id.clone();
        let resource_id = resource.id.clone();

        // 验证资源已分享
        let team = manager.get_team(&team_id).unwrap();
        assert_eq!(team.shared_resources.len(), 1);

        // 取消分享
        let unshared = manager.unshare_resource(&team_id, &resource_id, "owner1").unwrap();
        assert_eq!(unshared.resource_name, "Production");

        let team = manager.get_team(&team_id).unwrap();
        assert!(team.shared_resources.is_empty());
    }

    #[test]
    fn test_cleanup_expired() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 创建过期邀请（手动设置过期时间）
        let mut expired_invite = TeamInvite::new(&team.id, "expired@test.com", TeamRole::Member, "owner1", -1);
        expired_invite.expires_at = Utc::now() - chrono::Duration::hours(1);
        let invite_id = expired_invite.id.clone();
        manager.invites.insert(invite_id, expired_invite);

        // 清理过期邀请
        let expired = manager.cleanup_expired_invites();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].email, "expired@test.com");
        assert_eq!(expired[0].status, InviteStatus::Expired);
    }

    #[test]
    fn test_search_teams() {
        let mut manager = TeamManager::new();
        manager.create_team("Development Team", "user1", Some("User 1")).unwrap()
            .with_tags(vec!["dev", "engineering"]);
        manager.create_team("Operations Team", "user1", Some("User 1")).unwrap()
            .with_description("IT operations and infrastructure");
        manager.create_team("Sales Team", "user2", Some("User 2")).unwrap();

        let results = manager.search_teams("dev");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Development Team");

        let results = manager.search_teams("operations");
        assert_eq!(results.len(), 1);

        let results = manager.search_teams("team");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_team_stats() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 添加成员
        for i in 1..=3 {
            let invite = manager.invite_member(&team.id, &format!("user{}@test.com", i), TeamRole::Member, "owner1", None).unwrap();
            manager.accept_invite(&invite.id, &format!("user{}", i), &format!("User {}", i), &format!("user{}@test.com", i)).unwrap();
        }

        // 分享资源
        manager.share_resource(&team.id, "owner1", ShareableResourceType::Server, "srv1", "Server 1", ShareType::Full, vec![]).unwrap();
        manager.share_resource(&team.id, "owner1", ShareableResourceType::Snippet, "snip1", "Snippet 1", ShareType::Full, vec![]).unwrap();

        let stats = manager.get_team_stats(&team.id).unwrap();
        assert_eq!(stats.total_members, 4); // owner + 3 members
        assert_eq!(stats.active_members, 4);
        assert_eq!(stats.shared_resources, 2);
    }

    #[test]
    fn test_team_activities() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 邀请成员会产生活动
        let invite = manager.invite_member(&team.id, "member@test.com", TeamRole::Member, "owner1", None).unwrap();
        manager.accept_invite(&invite.id, "member1", "Member", "member@test.com").unwrap();

        // 获取活动
        let activities = manager.get_team_activities(&team.id, 10);
        assert!(!activities.is_empty());

        // 获取用户活动
        let user_activities = manager.get_user_activities("owner1", 10);
        assert!(!user_activities.is_empty());
    }

    #[test]
    fn test_invite_with_custom_message() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        let invite = manager.invite_member(
            &team.id,
            "test@example.com",
            TeamRole::Admin,
            "owner1",
            Some("Join our team!"),
        ).unwrap();

        assert_eq!(invite.custom_message, Some("Join our team!".to_string()));
        assert!(!invite.invite_code.is_empty());
    }

    #[test]
    fn test_member_status_management() {
        let mut member = TeamMember::new("team1", "user1", "User", "user@test.com", TeamRole::Member);

        assert!(member.is_active());

        member.suspend();
        assert!(!member.is_active());
        assert_eq!(member.status, MemberStatus::Suspended);

        member.activate();
        assert!(member.is_active());
        assert_eq!(member.status, MemberStatus::Active);
    }

    #[test]
    fn test_get_user_pending_invites() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        manager.invite_member(&team.id, "user1@test.com", TeamRole::Member, "owner1", None).unwrap();
        manager.invite_member(&team.id, "user1@test.com", TeamRole::Viewer, "owner1", None).unwrap(); // 同一邮箱第二次邀请
        manager.invite_member(&team.id, "user2@test.com", TeamRole::Member, "owner1", None).unwrap();

        let pending = manager.get_user_pending_invites("user1@test.com");
        // 第一个邀请已被第二个覆盖（Pending状态检查）
        assert_eq!(pending.len(), 1);

        let pending = manager.get_user_pending_invites("user2@test.com");
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_cannot_invite_existing_member() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        // 先邀请并添加成员
        let invite = manager.invite_member(&team.id, "member@test.com", TeamRole::Member, "owner1", None).unwrap();
        manager.accept_invite(&invite.id, "member1", "Member", "member@test.com").unwrap();

        // 不能再次邀请同一邮箱
        let result = manager.invite_member(&team.id, "member@test.com", TeamRole::Member, "owner1", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_accept_with_wrong_email() {
        let mut manager = TeamManager::new();
        let team = manager.create_team("Test Team", "owner1", Some("Owner")).unwrap();

        let invite = manager.invite_member(&team.id, "correct@example.com", TeamRole::Member, "owner1", None).unwrap();

        // 使用错误的邮箱接受
        let result = manager.accept_invite(&invite.id, "user2", "User", "wrong@example.com");
        assert!(result.is_err());
    }
}
