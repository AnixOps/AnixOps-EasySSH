//! 角色管理器 - 角色和分配的CRUD操作

use super::{
    types::*,
    RbacAuditEntry, RbacAuditLogger, RbacError, RbacConfig,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// 角色变更事件
#[derive(Debug, Clone)]
pub enum RoleChangeEvent {
    Created { role_id: String, by_user: String },
    Updated { role_id: String, by_user: String },
    Deleted { role_id: String, by_user: String },
    PermissionAdded { role_id: String, permission: String, by_user: String },
    PermissionRemoved { role_id: String, permission: String, by_user: String },
    Assigned { role_id: String, user_id: String, by_user: String },
    Revoked { role_id: String, user_id: String, by_user: String },
}

/// 角色变更监听器
pub trait RoleChangeListener: Send + Sync {
    fn on_role_change(&self, event: RoleChangeEvent);
}

/// 角色过滤器
#[derive(Debug, Clone, Default)]
pub struct RoleFilter {
    pub name_pattern: Option<String>,
    pub is_system: Option<bool>,
    pub team_id: Option<String>,
    pub has_permission: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 角色管理器
pub struct RoleManager {
    roles: std::sync::RwLock<HashMap<String, RoleDefinition>>,
    assignments: std::sync::RwLock<HashMap<String, Vec<UserRoleAssignment>>>,
    config: RbacConfig,
    audit_logger: Option<Arc<dyn RbacAuditLogger>>,
    change_listeners: Vec<Arc<dyn RoleChangeListener>>,
}

impl RoleManager {
    /// 创建新的角色管理器
    pub fn new() -> Self {
        let mut manager = Self {
            roles: std::sync::RwLock::new(HashMap::new()),
            assignments: std::sync::RwLock::new(HashMap::new()),
            config: RbacConfig::default(),
            audit_logger: None,
            change_listeners: Vec::new(),
        };

        // 初始化系统角色
        manager.init_system_roles();

        manager
    }

    /// 带配置创建
    pub fn with_config(mut self, config: RbacConfig) -> Self {
        self.config = config;
        self
    }

    /// 设置审计日志器
    pub fn with_audit_logger(mut self, logger: Arc<dyn RbacAuditLogger>) -> Self {
        self.audit_logger = Some(logger);
        self
    }

    /// 添加变更监听器
    pub fn add_change_listener(&mut self, listener: Arc<dyn RoleChangeListener>) {
        self.change_listeners.push(listener);
    }

    /// 初始化系统角色
    fn init_system_roles(&self) {
        let system_roles = super::init_system_roles();
        let mut roles = self.roles.write().unwrap();

        for role in system_roles {
            roles.insert(role.id.clone(), role);
        }
    }

    // ============= 角色CRUD操作 =============

    /// 创建角色
    pub fn create_role(
        &self,
        name: impl Into<String>,
        description: Option<&str>,
        by_user: impl Into<String>,
    ) -> Result<RoleDefinition, RbacError> {
        let id = uuid::Uuid::new_v4().to_string();
        let role = RoleDefinition::new(&id)
            .with_name(name)
            .with_description(description.unwrap_or(""));

        let mut roles = self.roles.write().unwrap();
        roles.insert(id.clone(), role.clone());

        self.notify_change(RoleChangeEvent::Created {
            role_id: id,
            by_user: by_user.into(),
        });

        Ok(role)
    }

    /// 创建团队角色
    pub fn create_team_role(
        &self,
        team_id: impl Into<String>,
        name: impl Into<String>,
        description: Option<&str>,
        by_user: impl Into<String>,
    ) -> Result<RoleDefinition, RbacError> {
        let team_id = team_id.into();
        let id = format!("{}-{}", team_id, uuid::Uuid::new_v4());
        let role = RoleDefinition::new(&id)
            .with_name(name)
            .with_description(description.unwrap_or(""))
            .with_scope(ResourceScope::Team(team_id.clone()));

        let mut roles = self.roles.write().unwrap();
        roles.insert(id.clone(), role.clone());

        self.notify_change(RoleChangeEvent::Created {
            role_id: id,
            by_user: by_user.into(),
        });

        Ok(role)
    }

    /// 获取角色
    pub fn get_role(&self, role_id: &str) -> Result<RoleDefinition, RbacError> {
        let roles = self.roles.read().unwrap();
        roles
            .get(role_id)
            .cloned()
            .ok_or_else(|| RbacError::RoleNotFound(role_id.to_string()))
    }

    /// 更新角色
    pub fn update_role(
        &self,
        role_id: impl Into<String>,
        name: Option<impl Into<String>>,
        description: Option<impl Into<String>>,
        by_user: impl Into<String>,
    ) -> Result<RoleDefinition, RbacError> {
        let role_id = role_id.into();
        let mut roles = self.roles.write().unwrap();

        let role = roles
            .get_mut(&role_id)
            .ok_or_else(|| RbacError::RoleNotFound(role_id.clone()))?;

        if role.is_system {
            return Err(RbacError::PermissionDenied(
                "Cannot modify system role".to_string(),
            ));
        }

        if let Some(name) = name {
            role.name = name.into();
        }
        if let Some(desc) = description {
            role.description = Some(desc.into());
        }

        let updated_role = role.clone();

        self.notify_change(RoleChangeEvent::Updated {
            role_id,
            by_user: by_user.into(),
        });

        Ok(updated_role)
    }

    /// 删除角色
    pub fn delete_role(
        &self,
        role_id: impl Into<String>,
        by_user: impl Into<String>,
    ) -> Result<(), RbacError> {
        let role_id = role_id.into();
        let mut roles = self.roles.write().unwrap();

        let role = roles
            .get(&role_id)
            .ok_or_else(|| RbacError::RoleNotFound(role_id.clone()))?;

        if role.is_system {
            return Err(RbacError::PermissionDenied(
                "Cannot delete system role".to_string(),
            ));
        }

        // 清理相关分配
        let mut assignments = self.assignments.write().unwrap();
        for user_assignments in assignments.values_mut() {
            user_assignments.retain(|a| a.role_id != role_id);
        }

        roles.remove(&role_id);

        self.notify_change(RoleChangeEvent::Deleted {
            role_id,
            by_user: by_user.into(),
        });

        Ok(())
    }

    /// 列出所有角色
    pub fn list_roles(&self, filter: Option<RoleFilter>) -> Vec<RoleDefinition> {
        let roles = self.roles.read().unwrap();

        let mut result: Vec<_> = roles.values().cloned().collect();

        if let Some(filter) = filter {
            if let Some(pattern) = filter.name_pattern {
                let pattern_lower = pattern.to_lowercase();
                result.retain(|r| {
                    r.name.to_lowercase().contains(&pattern_lower)
                        || r.id.to_lowercase().contains(&pattern_lower)
                });
            }

            if let Some(is_system) = filter.is_system {
                result.retain(|r| r.is_system == is_system);
            }

            if let Some(team_id) = filter.team_id {
                result.retain(|r| match &r.resource_scope {
                    ResourceScope::Team(tid) => tid == &team_id,
                    _ => false,
                });
            }

            if let Some(perm_str) = filter.has_permission {
                result.retain(|r| r.permissions.iter().any(|p| p.to_string() == perm_str));
            }

            if let Some(offset) = filter.offset {
                if offset < result.len() {
                    result = result.split_off(offset);
                } else {
                    result.clear();
                }
            }

            if let Some(limit) = filter.limit {
                result.truncate(limit);
            }
        }

        result
    }

    /// 搜索角色
    pub fn search_roles(&self, query: &str) -> Vec<RoleDefinition> {
        let query_lower = query.to_lowercase();
        let roles = self.roles.read().unwrap();

        roles
            .values()
            .filter(|r| {
                r.name.to_lowercase().contains(&query_lower)
                    || r.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    // ============= 权限管理 =============

    /// 添加权限到角色
    pub fn add_permission_to_role(
        &self,
        role_id: impl Into<String>,
        permission: Permission,
        by_user: impl Into<String>,
    ) -> Result<(), RbacError> {
        let role_id = role_id.into();
        let mut roles = self.roles.write().unwrap();

        let role = roles
            .get_mut(&role_id)
            .ok_or_else(|| RbacError::RoleNotFound(role_id.clone()))?;

        if role.is_system && !role.permissions.is_empty() {
            return Err(RbacError::PermissionDenied(
                "Cannot modify permissions of system role".to_string(),
            ));
        }

        role.add_permission(permission.clone());

        self.notify_change(RoleChangeEvent::PermissionAdded {
            role_id,
            permission: permission.to_string(),
            by_user: by_user.into(),
        });

        Ok(())
    }

    /// 从角色移除权限
    pub fn remove_permission_from_role(
        &self,
        role_id: impl Into<String>,
        permission: &Permission,
        by_user: impl Into<String>,
    ) -> Result<(), RbacError> {
        let role_id = role_id.into();
        let mut roles = self.roles.write().unwrap();

        let role = roles
            .get_mut(&role_id)
            .ok_or_else(|| RbacError::RoleNotFound(role_id.clone()))?;

        if role.is_system {
            return Err(RbacError::PermissionDenied(
                "Cannot modify permissions of system role".to_string(),
            ));
        }

        role.remove_permission(permission);

        self.notify_change(RoleChangeEvent::PermissionRemoved {
            role_id,
            permission: permission.to_string(),
            by_user: by_user.into(),
        });

        Ok(())
    }

    /// 设置角色权限（替换所有）
    pub fn set_role_permissions(
        &self,
        role_id: impl Into<String>,
        permissions: Vec<Permission>,
        by_user: impl Into<String>,
    ) -> Result<(), RbacError> {
        let role_id = role_id.into();
        let mut roles = self.roles.write().unwrap();

        let role = roles
            .get_mut(&role_id)
            .ok_or_else(|| RbacError::RoleNotFound(role_id.clone()))?;

        if role.is_system {
            return Err(RbacError::PermissionDenied(
                "Cannot modify permissions of system role".to_string(),
            ));
        }

        role.permissions = permissions.into_iter().collect();

        self.notify_change(RoleChangeEvent::Updated {
            role_id,
            by_user: by_user.into(),
        });

        Ok(())
    }

    /// 获取角色的有效权限（包含继承）
    pub fn get_role_effective_permissions(&self, role_id: &str) -> Result<HashSet<Permission>, RbacError> {
        let roles = self.roles.read().unwrap();
        let role = roles
            .get(role_id)
            .ok_or_else(|| RbacError::RoleNotFound(role_id.to_string()))?;

        Ok(role.effective_permissions(&*roles))
    }

    // ============= 角色分配管理 =============

    /// 分配角色给用户
    pub fn assign_role(
        &self,
        user_id: impl Into<String>,
        role_id: impl Into<String>,
        team_id: Option<&str>,
        by_user: impl Into<String>,
    ) -> Result<UserRoleAssignment, RbacError> {
        let user_id = user_id.into();
        let role_id = role_id.into();
        let by_user = by_user.into();

        // 验证角色存在
        let roles = self.roles.read().unwrap();
        if !roles.contains_key(&role_id) {
            return Err(RbacError::RoleNotFound(role_id.clone()));
        }
        drop(roles);

        let assignment = UserRoleAssignment::new(&user_id, team_id, &role_id, &by_user);

        let mut assignments = self.assignments.write().unwrap();
        assignments
            .entry(user_id.clone())
            .or_default()
            .push(assignment.clone());

        self.notify_change(RoleChangeEvent::Assigned {
            role_id,
            user_id,
            by_user,
        });

        Ok(assignment)
    }

    /// 撤销角色
    pub fn revoke_role(
        &self,
        user_id: impl Into<String>,
        role_id: Option<&str>,
        team_id: Option<&str>,
        by_user: impl Into<String>,
    ) -> Result<(), RbacError> {
        let user_id = user_id.into();
        let by_user = by_user.into();

        let mut assignments = self.assignments.write().unwrap();

        if let Some(user_assignments) = assignments.get_mut(&user_id) {
            let before_count = user_assignments.len();

            user_assignments.retain(|a| {
                let role_match = role_id.map(|r| r != a.role_id).unwrap_or(false);
                let team_match = match (team_id, &a.team_id) {
                    (Some(t1), Some(t2)) => t1 != t2,
                    (None, _) => false, // 如果未指定team_id，不匹配任何
                    _ => true,
                };
                role_match || team_match
            });

            let after_count = user_assignments.len();

            if after_count < before_count {
                if let Some(role_id) = role_id {
                    self.notify_change(RoleChangeEvent::Revoked {
                        role_id: role_id.to_string(),
                        user_id,
                        by_user,
                    });
                }
            }
        }

        Ok(())
    }

    /// 获取用户的角色分配
    pub fn get_user_assignments(&self, user_id: &str) -> Vec<UserRoleAssignment> {
        let assignments = self.assignments.read().unwrap();
        assignments
            .get(user_id)
            .map(|a| a.iter().filter(|a| a.is_valid()).cloned().collect())
            .unwrap_or_default()
    }

    /// 获取用户的有效角色
    pub fn get_user_roles(&self, user_id: &str) -> Vec<RoleDefinition> {
        let assignments = self.get_user_assignments(user_id);
        let roles = self.roles.read().unwrap();

        assignments
            .iter()
            .filter_map(|a| roles.get(&a.role_id).cloned())
            .collect()
    }

    /// 获取用户在特定团队的角色
    pub fn get_user_team_roles(&self, user_id: &str, team_id: &str) -> Vec<RoleDefinition> {
        let assignments = self.assignments.read().unwrap();
        let roles = self.roles.read().unwrap();

        assignments
            .get(user_id)
            .map(|user_assignments| {
                user_assignments
                    .iter()
                    .filter(|a| a.is_valid())
                    .filter(|a| a.team_id.as_deref() == Some(team_id))
                    .filter_map(|a| roles.get(&a.role_id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 检查用户是否有特定角色
    pub fn user_has_role(&self, user_id: &str, role_id: &str) -> bool {
        let assignments = self.assignments.read().unwrap();

        assignments
            .get(user_id)
            .map(|user_assignments| {
                user_assignments
                    .iter()
                    .filter(|a| a.is_valid())
                    .any(|a| a.role_id == role_id)
            })
            .unwrap_or(false)
    }

    /// 批量分配角色
    pub fn batch_assign_roles(
        &self,
        user_ids: &[String],
        role_id: &str,
        team_id: Option<&str>,
        by_user: &str,
    ) -> Result<Vec<UserRoleAssignment>, RbacError> {
        let mut results = Vec::new();

        for user_id in user_ids {
            match self.assign_role(user_id, role_id, team_id, by_user) {
                Ok(assignment) => results.push(assignment),
                Err(e) => {
                    // 记录错误但继续处理
                    eprintln!("Failed to assign role to {}: {}", user_id, e);
                }
            }
        }

        Ok(results)
    }

    // ============= 清理操作 =============

    /// 清理过期分配
    pub fn cleanup_expired_assignments(&self) -> usize {
        let mut assignments = self.assignments.write().unwrap();
        let mut removed_count = 0;

        for user_assignments in assignments.values_mut() {
            let before = user_assignments.len();
            user_assignments.retain(|a| !a.is_expired());
            removed_count += before - user_assignments.len();
        }

        removed_count
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> RoleManagerStats {
        let roles = self.roles.read().unwrap();
        let assignments = self.assignments.read().unwrap();

        RoleManagerStats {
            total_roles: roles.len(),
            system_roles: roles.values().filter(|r| r.is_system).count(),
            custom_roles: roles.values().filter(|r| !r.is_system).count(),
            total_assignments: assignments.values().map(|v| v.len()).sum(),
            active_assignments: assignments
                .values()
                .map(|v| v.iter().filter(|a| a.is_valid()).count())
                .sum(),
            expired_assignments: assignments
                .values()
                .map(|v| v.iter().filter(|a| a.is_expired()).count())
                .sum(),
        }
    }

    // ============= 内部方法 =============

    fn notify_change(&self, event: RoleChangeEvent) {
        for listener in &self.change_listeners {
            listener.on_role_change(event.clone());
        }

        if let Some(logger) = &self.audit_logger {
            match &event {
                RoleChangeEvent::Assigned { role_id, user_id, by_user } => {
                    logger.log_permission_change(user_id, role_id, "assigned");
                }
                RoleChangeEvent::Revoked { role_id, user_id, by_user } => {
                    logger.log_permission_change(user_id, role_id, "revoked");
                }
                _ => {}
            }
        }
    }
}

impl Default for RoleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 角色管理器统计
#[derive(Debug, Clone)]
pub struct RoleManagerStats {
    pub total_roles: usize,
    pub system_roles: usize,
    pub custom_roles: usize,
    pub total_assignments: usize,
    pub active_assignments: usize,
    pub expired_assignments: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> RoleManager {
        RoleManager::new()
    }

    #[test]
    fn test_create_role() {
        let manager = create_test_manager();
        let role = manager.create_role("Test Role", Some("A test role"), "admin").unwrap();

        assert_eq!(role.name, "Test Role");
        assert!(!role.is_system);
    }

    #[test]
    fn test_delete_system_role() {
        let manager = create_test_manager();
        let result = manager.delete_role("super_admin", "admin");

        assert!(result.is_err());
    }

    #[test]
    fn test_assign_role() {
        let manager = create_test_manager();
        let assignment = manager.assign_role("user1", "super_admin", None, "admin").unwrap();

        assert_eq!(assignment.user_id, "user1");
        assert_eq!(assignment.role_id, "super_admin");
    }

    #[test]
    fn test_get_user_roles() {
        let manager = create_test_manager();
        manager.assign_role("user1", "super_admin", None, "admin").unwrap();

        let roles = manager.get_user_roles("user1");
        assert!(!roles.is_empty());
        assert!(roles.iter().any(|r| r.id == "super_admin"));
    }

    #[test]
    fn test_role_filter() {
        let manager = create_test_manager();

        let filter = RoleFilter {
            is_system: Some(true),
            ..Default::default()
        };

        let roles = manager.list_roles(Some(filter));
        assert!(roles.iter().all(|r| r.is_system));
    }

    #[test]
    fn test_search_roles() {
        let manager = create_test_manager();
        manager.create_role("Search Test", Some("For searching"), "admin").unwrap();

        let results = manager.search_roles("search");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_add_permission_to_role() {
        let manager = create_test_manager();
        let role = manager.create_role("Test", None, "admin").unwrap();

        let permission = Permission::new(
            Resource::all(ResourceType::Server),
            Operation::Read,
        );

        manager.add_permission_to_role(&role.id, permission.clone(), "admin").unwrap();

        let updated_role = manager.get_role(&role.id).unwrap();
        assert!(updated_role.has_permission(&permission));
    }

    #[test]
    fn test_revoke_role() {
        let manager = create_test_manager();
        manager.assign_role("user1", "super_admin", None, "admin").unwrap();

        assert!(manager.user_has_role("user1", "super_admin"));

        manager.revoke_role("user1", Some("super_admin"), None, "admin").unwrap();

        assert!(!manager.user_has_role("user1", "super_admin"));
    }

    #[test]
    fn test_get_stats() {
        let manager = create_test_manager();
        let stats = manager.get_stats();

        assert!(stats.total_roles > 0);
        assert!(stats.system_roles > 0);
    }

    #[test]
    fn test_effective_permissions() {
        let manager = create_test_manager();

        // 创建继承关系的角色
        let parent = manager.create_role("Parent", None, "admin").unwrap();
        let perm = Permission::new(Resource::all(ResourceType::Server), Operation::Read);
        manager.add_permission_to_role(&parent.id, perm.clone(), "admin").unwrap();

        let child = RoleDefinition::new("child").inherits(vec![&parent.id]);
        {
            let mut roles = manager.roles.write().unwrap();
            roles.insert("child".to_string(), child);
        }

        let effective = manager.get_role_effective_permissions("child").unwrap();
        assert!(!effective.is_empty());
    }
}
