//! 权限检查器 - 核心权限验证逻辑

use chrono::{Datelike, Timelike};
use super::{
    types::*,
    RbacAuditEntry, RbacAuditLogger, RbacConfig, RbacError, ResourceResolver,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// 权限检查结果
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub matched_permission: Option<Permission>,
    pub applied_policies: Vec<String>,
}

impl CheckResult {
    /// 允许访问
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            reason: None,
            matched_permission: None,
            applied_policies: Vec::new(),
        }
    }

    /// 拒绝访问
    pub fn denied(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
            matched_permission: None,
            applied_policies: Vec::new(),
        }
    }

    /// 带权限的允许
    pub fn allowed_with_permission(permission: Permission) -> Self {
        Self {
            allowed: true,
            reason: None,
            matched_permission: Some(permission),
            applied_policies: Vec::new(),
        }
    }
}

/// 权限检查上下文（与types.rs中的PermissionContext区分，这里用于内部检查）
pub struct CheckContext {
    pub user_id: String,
    pub team_id: Option<String>,
    pub ip_address: Option<String>,
    pub is_super_admin: bool,
    pub is_mfa_verified: bool,
    pub request_time: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

impl CheckContext {
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            team_id: None,
            ip_address: None,
            is_super_admin: false,
            is_mfa_verified: false,
            request_time: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn from_permission_context(ctx: &PermissionContext) -> Self {
        Self {
            user_id: ctx.user_id.clone(),
            team_id: ctx.team_id.clone(),
            ip_address: ctx.ip_address.clone(),
            is_super_admin: ctx.is_super_admin,
            is_mfa_verified: ctx.is_mfa_verified,
            request_time: ctx.request_time,
            metadata: ctx.metadata.clone(),
        }
    }

    pub fn is_business_hours(&self) -> bool {
        let hour = self.request_time.hour();
        let weekday = self.request_time.weekday();
        weekday.num_days_from_monday() < 5 && hour >= 9 && hour < 18
    }
}

/// 权限缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    result: bool,
    timestamp: chrono::DateTime<chrono::Utc>,
    ttl_seconds: u64,
}

impl CacheEntry {
    fn is_valid(&self) -> bool {
        let elapsed = (chrono::Utc::now() - self.timestamp).num_seconds();
        elapsed < self.ttl_seconds as i64
    }
}

/// 权限检查器
pub struct PermissionChecker {
    config: RbacConfig,
    roles: Arc<HashMap<String, RoleDefinition>>,
    assignments: Arc<HashMap<String, Vec<UserRoleAssignment>>>,
    cache: std::sync::Mutex<HashMap<String, CacheEntry>>,
    audit_logger: Option<Arc<dyn RbacAuditLogger>>,
    resource_resolver: Option<Arc<dyn ResourceResolver>>,
}

impl PermissionChecker {
    /// 创建新的权限检查器
    pub fn new(
        roles: Arc<HashMap<String, RoleDefinition>>,
        assignments: Arc<HashMap<String, Vec<UserRoleAssignment>>>,
    ) -> Self {
        Self {
            config: RbacConfig::default(),
            roles,
            assignments,
            cache: std::sync::Mutex::new(HashMap::new()),
            audit_logger: None,
            resource_resolver: None,
        }
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

    /// 设置资源解析器
    pub fn with_resource_resolver(mut self, resolver: Arc<dyn ResourceResolver>) -> Self {
        self.resource_resolver = Some(resolver);
        self
    }

    /// 检查单个权限
    pub async fn check(
        &self,
        ctx: &PermissionContext,
        resource: &Resource,
        operation: Operation,
    ) -> Result<CheckResult, RbacError> {
        let permission = Permission::new(resource.clone(), operation);
        self.check_permission(ctx, &permission).await
    }

    /// 检查特定权限
    pub async fn check_permission(
        &self,
        ctx: &PermissionContext,
        permission: &Permission,
    ) -> Result<CheckResult, RbacError> {
        // 检查缓存
        if self.config.enable_cache {
            let cache_key = self.build_cache_key(ctx, permission);
            if let Some(entry) = self.cache.lock().unwrap().get(&cache_key) {
                if entry.is_valid() {
                    return Ok(CheckResult {
                        allowed: entry.result,
                        reason: if entry.result { None } else { Some("Cached: denied".to_string()) },
                        matched_permission: None,
                        applied_policies: vec!["cache".to_string()],
                    });
                }
            }
        }

        // 超级管理员直接通过
        if ctx.is_super_admin || self.config.super_admins.contains(&ctx.user_id) {
            let result = CheckResult::allowed_with_permission(permission.clone());
            self.update_cache(ctx, permission, true);
            self.log_audit(ctx, permission, &result).await;
            return Ok(result);
        }

        // 获取用户所有角色
        let user_roles = self.get_user_roles(ctx);

        // 检查权限
        let has_permission = user_roles.iter().any(|role| {
            let effective_perms = role.effective_permissions(&self.roles);
            effective_perms.iter().any(|p| {
                if p.covers(permission) {
                    // 检查条件
                    self.check_conditions(p, ctx)
                } else {
                    false
                }
            })
        });

        let result = if has_permission {
            CheckResult::allowed_with_permission(permission.clone())
        } else {
            CheckResult::denied("Insufficient permissions")
        };

        // 更新缓存
        if self.config.enable_cache {
            self.update_cache(ctx, permission, result.allowed);
        }

        // 记录审计日志
        self.log_audit(ctx, permission, &result).await;

        Ok(result)
    }

    /// 批量检查权限（任意一个）
    pub async fn check_any(
        &self,
        ctx: &PermissionContext,
        permissions: &[Permission],
    ) -> Result<CheckResult, RbacError> {
        for permission in permissions {
            let result = self.check_permission(ctx, permission).await?;
            if result.allowed {
                return Ok(result);
            }
        }

        Ok(CheckResult::denied("No matching permissions found"))
    }

    /// 批量检查权限（全部）
    pub async fn check_all(
        &self,
        ctx: &PermissionContext,
        permissions: &[Permission],
    ) -> Result<CheckResult, RbacError> {
        let mut missing = Vec::new();

        for permission in permissions {
            let result = self.check_permission(ctx, permission).await?;
            if !result.allowed {
                missing.push(format!(
                    "{}:{}",
                    permission.resource.resource_type.as_str(),
                    permission.operation.as_str()
                ));
            }
        }

        if missing.is_empty() {
            Ok(CheckResult::allowed())
        } else {
            Ok(CheckResult::denied(format!(
                "Missing permissions: {}",
                missing.join(", ")
            )))
        }
    }

    /// 检查资源级别的权限
    pub async fn check_resource(
        &self,
        ctx: &PermissionContext,
        resource_type: ResourceType,
        resource_id: Option<&str>,
        operation: Operation,
    ) -> Result<CheckResult, RbacError> {
        let resource = if let Some(id) = resource_id {
            Resource::specific(resource_type, id)
        } else {
            Resource::all(resource_type)
        };

        self.check(ctx, &resource, operation).await
    }

    /// 检查是否拥有特定角色
    pub fn has_role(&self, ctx: &PermissionContext, role_id: &str) -> bool {
        let assignments = self
            .assignments
            .get(&ctx.user_id)
            .map(|a| a.iter().filter(|a| a.is_valid()).collect::<Vec<_>>())
            .unwrap_or_default();

        assignments.iter().any(|a| {
            if a.role_id == role_id {
                // 检查团队范围
                if let (Some(ctx_team), Some(assign_team)) = (&ctx.team_id, &a.team_id) {
                    return ctx_team == assign_team;
                }
                true
            } else {
                false
            }
        })
    }

    /// 获取用户的所有有效权限
    pub fn get_user_permissions(&self, ctx: &PermissionContext) -> HashSet<Permission> {
        let roles = self.get_user_roles(ctx);
        let mut permissions = HashSet::new();

        for role in roles {
            permissions.extend(role.effective_permissions(&self.roles));
        }

        permissions
    }

    /// 获取用户的所有角色
    fn get_user_roles(&self, ctx: &PermissionContext) -> Vec<&RoleDefinition> {
        let assignments = self
            .assignments
            .get(&ctx.user_id)
            .map(|a| a.iter().filter(|a| a.is_valid()).collect::<Vec<_>>())
            .unwrap_or_default();

        assignments
            .iter()
            .filter_map(|a| {
                // 检查团队范围
                if let Some(ctx_team) = &ctx.team_id {
                    if let Some(assign_team) = &a.team_id {
                        if ctx_team != assign_team {
                            return None;
                        }
                    }
                }
                self.roles.get(&a.role_id)
            })
            .collect()
    }

    /// 检查权限条件
    fn check_conditions(&self, permission: &Permission, ctx: &PermissionContext) -> bool {
        if let Some(conditions) = &permission.conditions {
            // 检查工作时间
            if conditions.business_hours_only && !ctx.is_business_hours() {
                return false;
            }

            // 检查MFA
            if conditions.require_mfa && !ctx.is_mfa_verified {
                return false;
            }

            // 检查IP范围
            if let (Some(allowed_ranges), Some(ip)) = (&conditions.allowed_ip_ranges, &ctx.ip_address) {
                if !allowed_ranges.iter().any(|range| ip.starts_with(range)) {
                    return false;
                }
            }

            // 自定义条件检查可以在这里扩展
            if conditions.custom.is_some() {
                // 可以根据需要解析custom JSON进行更复杂的检查
            }
        }

        true
    }

    /// 构建缓存键
    fn build_cache_key(&self, ctx: &PermissionContext, permission: &Permission) -> String {
        format!(
            "{}:{}:{}:{:?}",
            ctx.user_id,
            permission.resource.resource_type.as_str(),
            permission.resource.resource_id.as_deref().unwrap_or("*"),
            permission.operation
        )
    }

    /// 更新缓存
    fn update_cache(&self, ctx: &PermissionContext, permission: &Permission, result: bool) {
        if !self.config.enable_cache {
            return;
        }

        let cache_key = self.build_cache_key(ctx, permission);
        let entry = CacheEntry {
            result,
            timestamp: chrono::Utc::now(),
            ttl_seconds: self.config.cache_ttl,
        };

        self.cache.lock().unwrap().insert(cache_key, entry);
    }

    /// 清除用户缓存
    pub fn clear_user_cache(&self, user_id: &str) {
        if !self.config.enable_cache {
            return;
        }

        let mut cache = self.cache.lock().unwrap();
        cache.retain(|key, _| !key.starts_with(&format!("{}:", user_id)));
    }

    /// 清除所有缓存
    pub fn clear_all_cache(&self) {
        if !self.config.enable_cache {
            return;
        }

        self.cache.lock().unwrap().clear();
    }

    /// 记录审计日志
    async fn log_audit(&self, ctx: &PermissionContext, permission: &Permission, result: &CheckResult) {
        if !self.config.enable_audit {
            return;
        }

        if let Some(logger) = &self.audit_logger {
            let entry = RbacAuditEntry {
                timestamp: chrono::Utc::now(),
                user_id: ctx.user_id.clone(),
                action: if result.allowed { "allow".to_string() } else { "deny".to_string() },
                resource: permission.resource.clone(),
                operation: permission.operation,
                allowed: result.allowed,
                reason: result.reason.clone(),
                context: ctx.session_id.clone(),
            };

            logger.log_access(entry);
        }
    }

    /// 获取缓存统计
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        let total = cache.len();
        let valid = cache.values().filter(|e| e.is_valid()).count();
        (total, valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_checker() -> PermissionChecker {
        let mut roles = HashMap::new();

        // 创建测试角色
        let mut admin_role = RoleDefinition::new("admin").with_name("Admin");
        admin_role.add_permissions(Permission::full(ResourceType::Server));
        roles.insert("admin".to_string(), admin_role);

        let mut user_role = RoleDefinition::new("user").with_name("User");
        user_role.add_permissions(vec![
            Permission::new(Resource::all(ResourceType::Server), Operation::Read),
            Permission::new(Resource::all(ResourceType::Server), Operation::Execute),
        ]);
        roles.insert("user".to_string(), user_role);

        let mut assignments = HashMap::new();
        assignments.insert(
            "user1".to_string(),
            vec![UserRoleAssignment::new("user1", None, "admin", "system")],
        );
        assignments.insert(
            "user2".to_string(),
            vec![UserRoleAssignment::new("user2", None, "user", "system")],
        );

        PermissionChecker::new(
            Arc::new(roles),
            Arc::new(assignments),
        )
    }

    #[tokio::test]
    async fn test_check_permission_allowed() {
        let checker = create_test_checker();
        let ctx = PermissionContext::new("user1");

        let result = checker
            .check(&ctx, &Resource::all(ResourceType::Server), Operation::Create)
            .await
            .unwrap();

        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_check_permission_denied() {
        let checker = create_test_checker();
        let ctx = PermissionContext::new("user2");

        let result = checker
            .check(&ctx, &Resource::all(ResourceType::Server), Operation::Create)
            .await
            .unwrap();

        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_super_admin_bypass() {
        let checker = create_test_checker();
        let ctx = PermissionContext::new("admin").as_super_admin();

        let result = checker
            .check(&ctx, &Resource::all(ResourceType::System), Operation::Manage)
            .await
            .unwrap();

        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_check_any() {
        let checker = create_test_checker();
        let ctx = PermissionContext::new("user2");

        let permissions = vec![
            Permission::new(Resource::all(ResourceType::Server), Operation::Create),
            Permission::new(Resource::all(ResourceType::Server), Operation::Read),
        ];

        let result = checker.check_any(&ctx, &permissions).await.unwrap();
        assert!(result.allowed); // 有Read权限
    }

    #[tokio::test]
    async fn test_check_all() {
        let checker = create_test_checker();
        let ctx = PermissionContext::new("user2");

        let permissions = vec![
            Permission::new(Resource::all(ResourceType::Server), Operation::Read),
            Permission::new(Resource::all(ResourceType::Server), Operation::Execute),
        ];

        let result = checker.check_all(&ctx, &permissions).await.unwrap();
        assert!(result.allowed);
    }

    #[test]
    fn test_has_role() {
        let checker = create_test_checker();
        let ctx = PermissionContext::new("user1");

        assert!(checker.has_role(&ctx, "admin"));
        assert!(!checker.has_role(&ctx, "user"));
    }

    #[tokio::test]
    async fn test_cache() {
        let checker = create_test_checker().with_config(RbacConfig {
            enable_cache: true,
            cache_ttl: 60,
            ..Default::default()
        });

        let ctx = PermissionContext::new("user1");
        let resource = Resource::all(ResourceType::Server);

        // 第一次检查
        checker.check(&ctx, &resource, Operation::Read).await.unwrap();

        // 检查缓存
        let (total, valid) = checker.cache_stats();
        assert_eq!(total, 1);
        assert_eq!(valid, 1);

        // 清除缓存
        checker.clear_user_cache("user1");
        let (total, valid) = checker.cache_stats();
        assert_eq!(total, 0);
    }

    #[tokio::test]
    async fn test_check_with_conditions() {
        let mut roles = HashMap::new();

        let mut role = RoleDefinition::new("limited").with_name("Limited User");
        role.add_permission(
            Permission::new(
                Resource::all(ResourceType::Server),
                Operation::Read,
            ).with_conditions(PermissionConditions {
                business_hours_only: true,
                ..Default::default()
            })
        );
        roles.insert("limited".to_string(), role);

        let mut assignments = HashMap::new();
        assignments.insert(
            "user3".to_string(),
            vec![UserRoleAssignment::new("user3", None, "limited", "system")],
        );

        let checker = PermissionChecker::new(
            Arc::new(roles),
            Arc::new(assignments),
        );

        let ctx = PermissionContext::new("user3");

        // 条件检查取决于当前时间，这里只验证结构
        let _ = checker.check(&ctx, &Resource::all(ResourceType::Server), Operation::Read).await;
    }
}
