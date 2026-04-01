//! 基于角色的访问控制模块 (Pro版本)
//! 提供细粒度的权限管理功能

use crate::error::LiteError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

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
        ]
    }
}

/// 资源标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub resource_id: Option<String>, // None 表示所有资源
}

impl Resource {
    /// 创建特定资源
    pub fn specific(resource_type: ResourceType, resource_id: impl Into<String>) -> Self {
        Self {
            resource_type,
            resource_id: Some(resource_id.into()),
        }
    }

    /// 创建通配资源（所有该类型资源）
    pub fn all(resource_type: ResourceType) -> Self {
        Self {
            resource_type,
            resource_id: None,
        }
    }

    /// 检查是否匹配
    pub fn matches(&self, other: &Resource) -> bool {
        if self.resource_type != other.resource_type {
            return false;
        }

        match (&self.resource_id, &other.resource_id) {
            (None, _) => true,        // self 是通配
            (Some(_), None) => false, // self 是特定，other 是通配
            (Some(a), Some(b)) => a == b,
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
}

/// 权限定义
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    pub resource: Resource,
    pub operation: Operation,
}

impl Permission {
    /// 创建新权限
    pub fn new(resource: Resource, operation: Operation) -> Self {
        Self {
            resource,
            operation,
        }
    }

    /// 快捷创建
    pub fn crud(resource_type: ResourceType) -> Vec<Permission> {
        Operation::crud()
            .into_iter()
            .map(|op| Permission::new(Resource::all(resource_type), op))
            .collect()
    }

    /// 完整权限
    pub fn full(resource_type: ResourceType) -> Vec<Permission> {
        Operation::all()
            .into_iter()
            .map(|op| Permission::new(Resource::all(resource_type), op))
            .collect()
    }

    /// 检查是否匹配
    pub fn covers(&self, other: &Permission) -> bool {
        self.resource.matches(&other.resource) && self.operation == other.operation
    }
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
}

impl RoleDefinition {
    /// 创建新角色
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            permissions: HashSet::new(),
            inherits_from: Vec::new(),
            is_system: false,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// 添加权限
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }

    /// 移除权限
    pub fn remove_permission(&mut self, permission: &Permission) {
        self.permissions.remove(permission);
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

    /// 获取所有有效权限（包括继承的）
    pub fn effective_permissions(&self, rbac: &RbacManager) -> HashSet<Permission> {
        let mut result = self.permissions.clone();

        for parent_id in &self.inherits_from {
            if let Some(parent) = rbac.get_role(parent_id) {
                result.extend(parent.effective_permissions(rbac));
            }
        }

        result
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
}

impl UserRoleAssignment {
    /// 创建新的角色分配
    pub fn new(user_id: &str, team_id: Option<&str>, role_id: &str, assigned_by: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            team_id: team_id.map(|s| s.to_string()),
            role_id: role_id.to_string(),
            assigned_by: assigned_by.to_string(),
            assigned_at: chrono::Utc::now(),
            expires_at: None,
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
}

/// 权限检查上下文
#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub user_id: String,
    pub team_id: Option<String>,
    pub ip_address: Option<String>,
    pub is_super_admin: bool,
}

impl PermissionContext {
    /// 创建上下文
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            team_id: None,
            ip_address: None,
            is_super_admin: false,
        }
    }

    /// 设置团队
    pub fn with_team(mut self, team_id: &str) -> Self {
        self.team_id = Some(team_id.to_string());
        self
    }

    /// 设置超级管理员
    pub fn as_super_admin(mut self) -> Self {
        self.is_super_admin = true;
        self
    }
}

/// RBAC管理器
pub struct RbacManager {
    roles: HashMap<String, RoleDefinition>,
    assignments: HashMap<String, Vec<UserRoleAssignment>>, // user_id -> assignments
    policies: Vec<Policy>,
}

/// 访问控制策略
pub struct Policy {
    pub name: String,
    pub condition: Arc<dyn Fn(&PermissionContext, &Permission) -> bool + Send + Sync>,
    pub effect: PolicyEffect,
}

impl std::fmt::Debug for Policy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Policy")
            .field("name", &self.name)
            .field("effect", &self.effect)
            .finish_non_exhaustive()
    }
}

impl Clone for Policy {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            condition: Arc::clone(&self.condition),
            effect: self.effect,
        }
    }
}

/// 策略效果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

impl RbacManager {
    /// 创建新的RBAC管理器
    pub fn new() -> Self {
        let mut manager = Self {
            roles: HashMap::new(),
            assignments: HashMap::new(),
            policies: Vec::new(),
        };

        // 初始化系统内置角色
        manager.init_system_roles();

        manager
    }

    fn init_system_roles(&mut self) {
        // 超级管理员
        let mut super_admin = RoleDefinition::new("Super Admin")
            .with_description("系统超级管理员，拥有所有权限")
            .as_system();

        // 添加所有资源的所有权限
        for resource_type in ResourceType::all() {
            super_admin.add_permissions(Permission::full(resource_type));
        }

        // 团队管理员
        let mut team_admin = RoleDefinition::new("Team Admin")
            .with_description("团队管理员，可以管理团队和成员")
            .as_system()
            .inherits(vec!["team_member"]);
        // 团队管理权限
        let team_admin_perms = Permission::full(ResourceType::Team);
        team_admin.add_permissions(team_admin_perms);
        let team_admin_perms = Permission::full(ResourceType::Member);
        team_admin.add_permissions(team_admin_perms);
        let team_admin_perms = Permission::full(ResourceType::Role);
        team_admin.add_permissions(team_admin_perms);
        let team_admin_perms = Permission::full(ResourceType::AuditLog);
        team_admin.add_permissions(team_admin_perms);

        // 团队成员
        let mut team_member = RoleDefinition::new("Team Member")
            .with_description("团队成员，可以管理服务器")
            .as_system();
        team_member.add_permissions(Permission::full(ResourceType::Server));
        team_member.add_permissions(Permission::full(ResourceType::Session));
        team_member.add_permissions(Permission::full(ResourceType::Snippet));
        team_member.add_permissions(Permission::full(ResourceType::Key));

        // 团队观察者
        let mut team_viewer = RoleDefinition::new("Team Viewer")
            .with_description("团队观察者，只读访问")
            .as_system();
        team_viewer.add_permission(Permission::new(
            Resource::all(ResourceType::Server),
            Operation::Read,
        ));
        team_viewer.add_permission(Permission::new(
            Resource::all(ResourceType::Session),
            Operation::Read,
        ));

        // 个人用户
        let mut personal = RoleDefinition::new("Personal")
            .with_description("个人用户，管理自己的资源")
            .as_system();
        personal.add_permissions(Permission::full(ResourceType::Server));
        personal.add_permissions(Permission::full(ResourceType::Key));
        personal.add_permissions(Permission::full(ResourceType::Snippet));
        personal.add_permissions(Permission::full(ResourceType::Layout));
        personal.add_permission(Permission::new(
            Resource::all(ResourceType::Config),
            Operation::Read,
        ));
        personal.add_permission(Permission::new(
            Resource::all(ResourceType::Config),
            Operation::Update,
        ));

        self.roles.insert("super_admin".to_string(), super_admin);
        self.roles.insert("team_admin".to_string(), team_admin);
        self.roles.insert("team_member".to_string(), team_member);
        self.roles.insert("team_viewer".to_string(), team_viewer);
        self.roles.insert("personal".to_string(), personal);
    }

    /// 创建角色
    pub fn create_role(&mut self, name: &str, description: Option<&str>) -> RoleDefinition {
        let mut role = RoleDefinition::new(name);
        if let Some(desc) = description {
            role = role.with_description(desc);
        }

        let id = role.id.clone();
        self.roles.insert(id.clone(), role.clone());
        role
    }

    /// 获取角色
    pub fn get_role(&self, role_id: &str) -> Option<&RoleDefinition> {
        self.roles.get(role_id)
    }

    /// 获取角色（可变）
    pub fn get_role_mut(&mut self, role_id: &str) -> Option<&mut RoleDefinition> {
        self.roles.get_mut(role_id)
    }

    /// 更新角色
    pub fn update_role(&mut self, role: RoleDefinition) -> Result<(), LiteError> {
        if let Some(existing) = self.roles.get(&role.id) {
            if existing.is_system {
                return Err(LiteError::Rbac("Cannot modify system role".to_string()));
            }
            self.roles.insert(role.id.clone(), role);
            Ok(())
        } else {
            Err(LiteError::Rbac(format!("Role {} not found", role.id)))
        }
    }

    /// 删除角色
    pub fn delete_role(&mut self, role_id: &str) -> Result<(), LiteError> {
        if let Some(role) = self.roles.get(role_id) {
            if role.is_system {
                return Err(LiteError::Rbac("Cannot delete system role".to_string()));
            }
        }

        // 清理相关分配
        for assignments in self.assignments.values_mut() {
            assignments.retain(|a| a.role_id != role_id);
        }

        self.roles.remove(role_id);
        Ok(())
    }

    /// 列出所有角色
    pub fn list_roles(&self) -> Vec<&RoleDefinition> {
        self.roles.values().collect()
    }

    /// 分配角色给用户
    pub fn assign_role(&mut self, assignment: UserRoleAssignment) -> Result<(), LiteError> {
        if !self.roles.contains_key(&assignment.role_id) {
            return Err(LiteError::Rbac(format!(
                "Role {} not found",
                assignment.role_id
            )));
        }

        self.assignments
            .entry(assignment.user_id.clone())
            .or_default()
            .push(assignment);

        Ok(())
    }

    /// 撤销角色
    pub fn revoke_role(
        &mut self,
        user_id: &str,
        role_id: &str,
        team_id: Option<&str>,
    ) -> Result<(), LiteError> {
        if let Some(assignments) = self.assignments.get_mut(user_id) {
            assignments.retain(|a| !(a.role_id == role_id && a.team_id.as_deref() == team_id));
        }
        Ok(())
    }

    /// 获取用户的角色分配
    pub fn get_user_assignments(&self, user_id: &str) -> Vec<&UserRoleAssignment> {
        self.assignments
            .get(user_id)
            .map(|v| v.iter().filter(|a| !a.is_expired()).collect())
            .unwrap_or_default()
    }

    /// 获取用户在特定团队的角色
    pub fn get_user_team_roles(&self, user_id: &str, team_id: &str) -> Vec<&RoleDefinition> {
        self.assignments
            .get(user_id)
            .map(|assignments| {
                assignments
                    .iter()
                    .filter(|a| !a.is_expired())
                    .filter(|a| a.team_id.as_deref() == Some(team_id))
                    .filter_map(|a| self.roles.get(&a.role_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 检查权限
    pub fn check_permission(&self, context: &PermissionContext, permission: &Permission) -> bool {
        // 超级管理员直接通过
        if context.is_super_admin {
            return true;
        }

        // 检查策略（Deny优先）
        for policy in &self.policies {
            if (policy.condition)(context, permission) {
                match policy.effect {
                    PolicyEffect::Deny => return false,
                    PolicyEffect::Allow => return true,
                }
            }
        }

        // 获取用户角色
        let user_assignments = self
            .assignments
            .get(&context.user_id)
            .map(|a| a.iter().filter(|a| !a.is_expired()).collect::<Vec<_>>())
            .unwrap_or_default();

        for assignment in &user_assignments {
            // 检查团队范围
            if let Some(team_id) = &context.team_id {
                if let Some(assignment_team) = &assignment.team_id {
                    if assignment_team != team_id {
                        continue;
                    }
                }
            }

            if let Some(role) = self.roles.get(&assignment.role_id) {
                let effective_perms = role.effective_permissions(self);
                if effective_perms.iter().any(|p| p.covers(permission)) {
                    return true;
                }
            }
        }

        false
    }

    /// 批量检查权限（任意一个）
    pub fn check_any_permission(
        &self,
        context: &PermissionContext,
        permissions: &[Permission],
    ) -> bool {
        permissions
            .iter()
            .any(|p| self.check_permission(context, p))
    }

    /// 批量检查权限（全部）
    pub fn check_all_permissions(
        &self,
        context: &PermissionContext,
        permissions: &[Permission],
    ) -> bool {
        permissions
            .iter()
            .all(|p| self.check_permission(context, p))
    }

    /// 添加策略
    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.push(policy);
    }

    /// 获取用户的所有权限
    pub fn get_user_permissions(
        &self,
        user_id: &str,
        team_id: Option<&str>,
    ) -> HashSet<Permission> {
        let assignments = self
            .assignments
            .get(user_id)
            .map(|a| a.iter().filter(|a| !a.is_expired()).collect::<Vec<_>>())
            .unwrap_or_default();

        let mut result = HashSet::new();

        for assignment in assignments {
            // 过滤团队
            if let Some(tid) = team_id {
                if let Some(atid) = &assignment.team_id {
                    if atid != tid {
                        continue;
                    }
                }
            }

            if let Some(role) = self.roles.get(&assignment.role_id) {
                result.extend(role.effective_permissions(self));
            }
        }

        result
    }

    /// 创建资源级别的权限检查
    pub fn can_access_resource(
        &self,
        context: &PermissionContext,
        resource_type: ResourceType,
        resource_id: &str,
        operation: Operation,
    ) -> bool {
        let permission = Permission::new(Resource::specific(resource_type, resource_id), operation);
        self.check_permission(context, &permission)
    }

    /// 初始化个人用户角色
    pub fn setup_personal_user(&mut self, user_id: &str) -> Result<(), LiteError> {
        let assignment = UserRoleAssignment::new(user_id, None, "personal", "system");
        self.assign_role(assignment)
    }
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 权限检查快捷宏
#[macro_export]
macro_rules! require_permission {
    ($rbac:expr, $ctx:expr, $resource:expr, $operation:expr) => {
        if !$rbac.check_permission($ctx, &$crate::rbac::Permission::new($resource, $operation)) {
            return Err($crate::error::LiteError::Rbac(
                "Permission denied".to_string(),
            ));
        }
    };
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_covers() {
        // 通配权限
        let wildcard = Permission::new(Resource::all(ResourceType::Server), Operation::Read);

        // 特定权限
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
        let mut role = RoleDefinition::new("Test Role");
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Server),
            Operation::Read,
        ));
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Server),
            Operation::Update,
        ));

        assert!(role.has_permission(&Permission::new(
            Resource::specific(ResourceType::Server, "srv1"),
            Operation::Read,
        )));
        assert!(!role.has_permission(&Permission::new(
            Resource::all(ResourceType::Server),
            Operation::Delete,
        )));
    }

    #[test]
    fn test_rbac_manager_system_roles() {
        let rbac = RbacManager::new();

        let super_admin = rbac.get_role("super_admin").unwrap();
        assert!(super_admin.is_system);
        assert!(super_admin.has_permission(&Permission::new(
            Resource::all(ResourceType::System),
            Operation::Manage,
        )));

        let personal = rbac.get_role("personal").unwrap();
        assert!(personal.has_permission(&Permission::new(
            Resource::all(ResourceType::Server),
            Operation::Create,
        )));
    }

    #[test]
    fn test_role_assignment() {
        let mut rbac = RbacManager::new();

        // 分配个人角色
        rbac.setup_personal_user("user1").unwrap();

        let ctx = PermissionContext::new("user1");

        // 应该可以创建服务器
        assert!(rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::Server), Operation::Create,)
        ));

        // 不应该能管理团队
        assert!(!rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::Team), Operation::Manage,)
        ));
    }

    #[test]
    fn test_team_role() {
        let mut rbac = RbacManager::new();

        rbac.setup_personal_user("user1").unwrap();

        // 分配团队管理员角色
        let assignment = UserRoleAssignment::new("user1", Some("team1"), "team_admin", "admin");
        rbac.assign_role(assignment).unwrap();

        let ctx = PermissionContext::new("user1").with_team("team1");

        // 在team1中可以管理成员
        assert!(rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::Member), Operation::Manage,)
        ));

        // 在team2中不行
        let ctx2 = PermissionContext::new("user1").with_team("team2");
        assert!(!rbac.check_permission(
            &ctx2,
            &Permission::new(Resource::all(ResourceType::Member), Operation::Manage,)
        ));
    }

    #[test]
    fn test_super_admin() {
        let rbac = RbacManager::new();

        let ctx = PermissionContext::new("admin").as_super_admin();

        // 超级管理员可以执行任何操作
        assert!(rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::System), Operation::Manage,)
        ));
        assert!(rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::Server), Operation::Delete,)
        ));
    }

    #[test]
    fn test_expired_assignment() {
        let mut rbac = RbacManager::new();

        // 创建已过期1小时的分配
        let assignment = UserRoleAssignment::new("user1", None, "super_admin", "admin")
            .expires(chrono::Utc::now() - chrono::Duration::hours(1));
        rbac.assign_role(assignment).unwrap();

        let ctx = PermissionContext::new("user1");

        // 权限应该已过期
        assert!(!rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::Server), Operation::Manage,)
        ));
    }

    #[test]
    fn test_delete_system_role() {
        let mut rbac = RbacManager::new();

        let result = rbac.delete_role("super_admin");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_custom_role() {
        let mut rbac = RbacManager::new();

        let role = rbac.create_role("Custom Role", Some("自定义角色"));
        assert_eq!(role.name, "Custom Role");
        assert!(!role.is_system);

        // 可以删除
        let result = rbac.delete_role(&role.id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_inheritance() {
        let mut rbac = RbacManager::new();

        // 创建继承personal的角色
        let mut custom = RoleDefinition::new("Custom").inherits(vec!["personal"]);

        // 添加额外权限
        custom.add_permission(Permission::new(
            Resource::all(ResourceType::AuditLog),
            Operation::Read,
        ));

        let id = custom.id.clone();
        rbac.roles.insert(id.clone(), custom);

        let assignment = UserRoleAssignment::new("user1", None, &id, "admin");
        rbac.assign_role(assignment).unwrap();

        let ctx = PermissionContext::new("user1");

        // 应该继承personal的权限
        assert!(rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::Server), Operation::Create,)
        ));

        // 也应有自己的权限
        assert!(rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::AuditLog), Operation::Read,)
        ));
    }

    #[test]
    fn test_permission_crud() {
        let perms = Permission::crud(ResourceType::Server);
        assert_eq!(perms.len(), 4);
        assert!(perms.iter().any(|p| p.operation == Operation::Create));
        assert!(perms.iter().any(|p| p.operation == Operation::Read));
        assert!(perms.iter().any(|p| p.operation == Operation::Update));
        assert!(perms.iter().any(|p| p.operation == Operation::Delete));
    }

    #[test]
    fn test_batch_permissions() {
        let mut rbac = RbacManager::new();
        rbac.setup_personal_user("user1").unwrap();

        let ctx = PermissionContext::new("user1");

        let perms = vec![
            Permission::new(Resource::all(ResourceType::Server), Operation::Read),
            Permission::new(Resource::all(ResourceType::Key), Operation::Read),
        ];

        assert!(rbac.check_all_permissions(&ctx, &perms));
        assert!(rbac.check_any_permission(&ctx, &perms));
    }

    #[test]
    fn test_can_access_resource() {
        let mut rbac = RbacManager::new();
        rbac.setup_personal_user("user1").unwrap();

        let ctx = PermissionContext::new("user1");

        assert!(rbac.can_access_resource(&ctx, ResourceType::Server, "srv1", Operation::Read));
        assert!(!rbac.can_access_resource(&ctx, ResourceType::Team, "team1", Operation::Manage));
    }

    #[test]
    fn test_user_permissions() {
        let mut rbac = RbacManager::new();
        rbac.setup_personal_user("user1").unwrap();

        let perms = rbac.get_user_permissions("user1", None);
        assert!(!perms.is_empty());
        assert!(perms
            .iter()
            .any(|p| p.resource.resource_type == ResourceType::Server));
    }

    #[test]
    fn test_revoke_role() {
        let mut rbac = RbacManager::new();
        rbac.setup_personal_user("user1").unwrap();

        let ctx = PermissionContext::new("user1");
        assert!(rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::Server), Operation::Read,)
        ));

        // 撤销角色
        rbac.revoke_role("user1", "personal", None).unwrap();

        assert!(!rbac.check_permission(
            &ctx,
            &Permission::new(Resource::all(ResourceType::Server), Operation::Read,)
        ));
    }

    #[test]
    fn test_role_serialization() {
        let role = RoleDefinition::new("Test")
            .with_description("Test role")
            .as_system();

        let json = serde_json::to_string(&role).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("system"));
    }
}
