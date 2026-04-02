//! RBAC核心类型定义

use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Server,
    Session,
    Team,
    Member,
    Role,
    Key,
    Snippet,
    Layout,
    Config,
    AuditLog,
    System,
    Collaboration,
}

impl ResourceType {
    /// 获取所有资源类型
    pub fn all() -> Vec<ResourceType> {
        vec![
            ResourceType::Server,
            ResourceType::Session,
            ResourceType::Team,
            ResourceType::Member,
            ResourceType::Role,
            ResourceType::Key,
            ResourceType::Snippet,
            ResourceType::Layout,
            ResourceType::Config,
            ResourceType::AuditLog,
            ResourceType::System,
            ResourceType::Collaboration,
        ]
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "server" => Some(ResourceType::Server),
            "session" => Some(ResourceType::Session),
            "team" => Some(ResourceType::Team),
            "member" => Some(ResourceType::Member),
            "role" => Some(ResourceType::Role),
            "key" => Some(ResourceType::Key),
            "snippet" => Some(ResourceType::Snippet),
            "layout" => Some(ResourceType::Layout),
            "config" => Some(ResourceType::Config),
            "audit_log" => Some(ResourceType::AuditLog),
            "system" => Some(ResourceType::System),
            "collaboration" => Some(ResourceType::Collaboration),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Server => "server",
            ResourceType::Session => "session",
            ResourceType::Team => "team",
            ResourceType::Member => "member",
            ResourceType::Role => "role",
            ResourceType::Key => "key",
            ResourceType::Snippet => "snippet",
            ResourceType::Layout => "layout",
            ResourceType::Config => "config",
            ResourceType::AuditLog => "audit_log",
            ResourceType::System => "system",
            ResourceType::Collaboration => "collaboration",
        }
    }
}

/// 资源标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub resource_id: Option<String>, // None 表示所有资源
    pub team_id: Option<String>,     // 可选的团队范围
}

impl Resource {
    /// 创建特定资源
    pub fn specific(resource_type: ResourceType, resource_id: impl Into<String>) -> Self {
        Self {
            resource_type,
            resource_id: Some(resource_id.into()),
            team_id: None,
        }
    }

    /// 创建通配资源（所有该类型资源）
    pub fn all(resource_type: ResourceType) -> Self {
        Self {
            resource_type,
            resource_id: None,
            team_id: None,
        }
    }

    /// 创建团队范围内的资源
    pub fn in_team(mut self, team_id: impl Into<String>) -> Self {
        self.team_id = Some(team_id.into());
        self
    }

    /// 检查是否匹配
    pub fn matches(&self, other: &Resource) -> bool {
        // 检查资源类型
        if self.resource_type != other.resource_type {
            return false;
        }

        // 检查团队范围（如果指定）
        if let (Some(self_team), Some(other_team)) = (&self.team_id, &other.team_id) {
            if self_team != other_team {
                return false;
            }
        }

        // 检查资源ID匹配
        match (&self.resource_id, &other.resource_id) {
            (None, _) => true,        // self 是通配
            (Some(_), None) => false, // self 是特定，other 是通配
            (Some(a), Some(b)) => a == b,
        }
    }

    /// 检查是否包含特定资源
    pub fn contains(&self, resource_id: &str) -> bool {
        match &self.resource_id {
            None => true,
            Some(id) => id == resource_id,
        }
    }
}

/// 操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Create,
    Read,
    Update,
    Delete,
    Execute, // 执行命令、连接服务器等
    Manage,  // 管理权限、分配角色等
}

impl Operation {
    /// 获取CRUD操作
    pub fn crud() -> Vec<Operation> {
        vec![
            Operation::Create,
            Operation::Read,
            Operation::Update,
            Operation::Delete,
        ]
    }

    /// 获取所有操作
    pub fn all() -> Vec<Operation> {
        vec![
            Operation::Create,
            Operation::Read,
            Operation::Update,
            Operation::Delete,
            Operation::Execute,
            Operation::Manage,
        ]
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "create" => Some(Operation::Create),
            "read" => Some(Operation::Read),
            "update" => Some(Operation::Update),
            "delete" => Some(Operation::Delete),
            "execute" => Some(Operation::Execute),
            "manage" => Some(Operation::Manage),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Operation::Create => "create",
            Operation::Read => "read",
            Operation::Update => "update",
            Operation::Delete => "delete",
            Operation::Execute => "execute",
            Operation::Manage => "manage",
        }
    }
}

/// 权限定义
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    pub resource: Resource,
    pub operation: Operation,
    pub conditions: Option<PermissionConditions>,
}

/// 权限条件
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PermissionConditions {
    /// 仅允许在工作时间访问
    pub business_hours_only: bool,
    /// 需要MFA验证
    pub require_mfa: bool,
    /// 允许的IP范围
    pub allowed_ip_ranges: Option<Vec<String>>,
    /// 最大访问频率（每分钟）
    pub rate_limit: Option<u32>,
    /// 自定义条件JSON
    pub custom: Option<serde_json::Value>,
}

impl Default for PermissionConditions {
    fn default() -> Self {
        Self {
            business_hours_only: false,
            require_mfa: false,
            allowed_ip_ranges: None,
            rate_limit: None,
            custom: None,
        }
    }
}

impl Permission {
    /// 创建新权限
    pub fn new(resource: Resource, operation: Operation) -> Self {
        Self {
            resource,
            operation,
            conditions: None,
        }
    }

    /// 带条件的权限
    pub fn with_conditions(mut self, conditions: PermissionConditions) -> Self {
        self.conditions = Some(conditions);
        self
    }

    /// 快捷创建CRUD权限集
    pub fn crud(resource_type: ResourceType) -> Vec<Permission> {
        Operation::crud()
            .into_iter()
            .map(|op| Permission::new(Resource::all(resource_type), op))
            .collect()
    }

    /// 完整权限（所有操作）
    pub fn full(resource_type: ResourceType) -> Vec<Permission> {
        Operation::all()
            .into_iter()
            .map(|op| Permission::new(Resource::all(resource_type), op))
            .collect()
    }

    /// 检查是否覆盖另一个权限
    pub fn covers(&self, other: &Permission) -> bool {
        self.resource.matches(&other.resource) && self.operation == other.operation
    }

    /// 转换为字符串表示
    pub fn to_string(&self) -> String {
        format!(
            "{}:{}",
            self.resource.resource_type.as_str(),
            self.operation.as_str()
        )
    }
}

/// 资源范围
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceScope {
    Global,
    Team(String),
    Server(String),
}

/// 角色定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: HashSet<Permission>,
    pub inherits_from: Vec<String>, // 继承的其他角色ID
    pub is_system: bool,            // 系统内置角色，不可删除
    pub resource_scope: ResourceScope,
    pub metadata: Option<serde_json::Value>,
}

impl RoleDefinition {
    /// 创建新角色
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            description: None,
            permissions: HashSet::new(),
            inherits_from: Vec::new(),
            is_system: false,
            resource_scope: ResourceScope::Global,
            metadata: None,
        }
    }

    /// 设置名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// 设置资源范围
    pub fn with_scope(mut self, scope: ResourceScope) -> Self {
        self.resource_scope = scope;
        self
    }

    /// 添加权限
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }

    /// 移除权限
    pub fn remove_permission(&mut self, permission: &Permission) {
        self.permissions.retain(|p| !p.covers(permission));
    }

    /// 批量添加权限
    pub fn add_permissions(&mut self, permissions: Vec<Permission>) {
        for p in permissions {
            self.permissions.insert(p);
        }
    }

    /// 设置系统角色
    pub fn as_system(mut self) -> Self {
        self.is_system = true;
        self
    }

    /// 设置继承
    pub fn inherits(mut self, role_ids: Vec<&str>) -> Self {
        self.inherits_from = role_ids.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// 检查是否有权限
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.iter().any(|p| p.covers(permission))
    }

    /// 检查是否有特定资源的权限
    pub fn has_resource_permission(
        &self,
        resource_type: ResourceType,
        resource_id: Option<&str>,
        operation: Operation,
    ) -> bool {
        let resource = if let Some(id) = resource_id {
            Resource::specific(resource_type, id)
        } else {
            Resource::all(resource_type)
        };
        self.has_permission(&Permission::new(resource, operation))
    }

    /// 获取所有有效权限（包括继承的）
    pub fn effective_permissions(&self, all_roles: &HashMap<String, RoleDefinition>) -> HashSet<Permission> {
        let mut result = self.permissions.clone();

        for parent_id in &self.inherits_from {
            if let Some(parent) = all_roles.get(parent_id) {
                let parent_perms = parent.effective_permissions(all_roles);
                result.extend(parent_perms);
            }
        }

        result
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 用户角色分配
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleAssignment {
    pub user_id: String,
    pub team_id: Option<String>, // None 表示全局角色
    pub role_id: String,
    pub assigned_by: String,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: Option<serde_json::Value>,
}

impl UserRoleAssignment {
    /// 创建新的角色分配
    pub fn new(
        user_id: impl Into<String>,
        team_id: Option<&str>,
        role_id: impl Into<String>,
        assigned_by: impl Into<String>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            team_id: team_id.map(|s| s.to_string()),
            role_id: role_id.into(),
            assigned_by: assigned_by.into(),
            assigned_at: chrono::Utc::now(),
            expires_at: None,
            metadata: None,
        }
    }

    /// 设置过期时间
    pub fn expires(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|e| chrono::Utc::now() > e)
            .unwrap_or(false)
    }

    /// 检查是否在有效期内
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    /// 获取剩余有效时间（秒）
    pub fn remaining_seconds(&self) -> Option<i64> {
        self.expires_at.map(|e| {
            let now = chrono::Utc::now();
            if e > now {
                (e - now).num_seconds()
            } else {
                0
            }
        })
    }
}

/// 权限检查上下文
#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub user_id: String,
    pub team_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub is_super_admin: bool,
    pub is_mfa_verified: bool,
    pub session_id: Option<String>,
    pub request_time: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

impl PermissionContext {
    /// 创建上下文
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            team_id: None,
            ip_address: None,
            user_agent: None,
            is_super_admin: false,
            is_mfa_verified: false,
            session_id: None,
            request_time: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// 设置团队
    pub fn with_team(mut self, team_id: impl Into<String>) -> Self {
        self.team_id = Some(team_id.into());
        self
    }

    /// 设置IP地址
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    /// 设置超级管理员
    pub fn as_super_admin(mut self) -> Self {
        self.is_super_admin = true;
        self
    }

    /// 设置MFA已验证
    pub fn with_mfa_verified(mut self) -> Self {
        self.is_mfa_verified = true;
        self
    }

    /// 设置会话ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 检查是否在工作时间（示例实现）
    pub fn is_business_hours(&self) -> bool {
        let hour = self.request_time.hour();
        let weekday = self.request_time.weekday();

        // 周一到周五，9点到18点
        weekday.num_days_from_monday() < 5 && hour >= 9 && hour < 18
    }
}

/// 用户RBAC信息
#[derive(Debug, Clone)]
pub struct UserRbacInfo {
    pub user_id: String,
    pub roles: Vec<String>,
    pub team_roles: HashMap<String, Vec<String>>, // team_id -> role_ids
    pub permissions: HashSet<Permission>,
    pub is_super_admin: bool,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl UserRbacInfo {
    /// 创建用户RBAC信息
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            roles: Vec::new(),
            team_roles: HashMap::new(),
            permissions: HashSet::new(),
            is_super_admin: false,
            last_updated: chrono::Utc::now(),
        }
    }

    /// 添加角色
    pub fn add_role(&mut self, role_id: impl Into<String>) {
        self.roles.push(role_id.into());
    }

    /// 添加团队角色
    pub fn add_team_role(&mut self, team_id: impl Into<String>, role_id: impl Into<String>) {
        self.team_roles
            .entry(team_id.into())
            .or_default()
            .push(role_id.into());
    }

    /// 检查是否有权限
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.iter().any(|p| p.covers(permission))
    }

    /// 检查是否需要刷新
    pub fn needs_refresh(&self, ttl_seconds: u64) -> bool {
        let elapsed = (chrono::Utc::now() - self.last_updated).num_seconds();
        elapsed > ttl_seconds as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_covers() {
        let wildcard = Permission::new(Resource::all(ResourceType::Server), Operation::Read);
        let specific = Permission::new(
            Resource::specific(ResourceType::Server, "server1"),
            Operation::Read,
        );

        assert!(wildcard.covers(&specific));
        assert!(!specific.covers(&wildcard));
    }

    #[test]
    fn test_resource_matches() {
        let all_servers = Resource::all(ResourceType::Server);
        let specific = Resource::specific(ResourceType::Server, "srv1");

        assert!(all_servers.matches(&specific));
        assert!(!specific.matches(&all_servers));
    }

    #[test]
    fn test_role_definition() {
        let mut role = RoleDefinition::new("test_role").with_name("Test Role");
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Server),
            Operation::Read,
        ));

        assert!(role.has_permission(&Permission::new(
            Resource::specific(ResourceType::Server, "srv1"),
            Operation::Read,
        )));
    }

    #[test]
    fn test_permission_context() {
        let ctx = PermissionContext::new("user1")
            .with_team("team1")
            .with_ip("192.168.1.1")
            .as_super_admin();

        assert_eq!(ctx.user_id, "user1");
        assert_eq!(ctx.team_id, Some("team1".to_string()));
        assert!(ctx.is_super_admin);
    }

    #[test]
    fn test_user_role_assignment_expiration() {
        let assignment = UserRoleAssignment::new("user1", None, "admin", "system")
            .expires(chrono::Utc::now() - chrono::Duration::hours(1));

        assert!(assignment.is_expired());
    }

    #[test]
    fn test_role_inheritance() {
        let mut roles = HashMap::new();

        let parent = RoleDefinition::new("parent")
            .with_name("Parent")
            .add_permissions(Permission::crud(ResourceType::Server));
        roles.insert("parent".to_string(), parent);

        let child = RoleDefinition::new("child")
            .with_name("Child")
            .inherits(vec!["parent"]);
        roles.insert("child".to_string(), child.clone());

        let effective = child.effective_permissions(&roles);
        assert_eq!(effective.len(), 4); // CRUD
    }

    #[test]
    fn test_resource_scope() {
        let global_scope = ResourceScope::Global;
        let team_scope = ResourceScope::Team("team1".to_string());

        assert_ne!(global_scope, team_scope);
    }
}
