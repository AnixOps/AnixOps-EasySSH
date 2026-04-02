//! RBAC模块 - 基于角色的访问控制系统
//!
//! 提供细粒度的权限管理功能，包括：
//! - 权限定义和检查
//! - 角色管理和继承
//! - 策略引擎
//! - 资源解析

pub mod checker;
pub mod manager;
pub mod policy;
pub mod types;

// 重新导出主要类型
pub use checker::{CheckContext, CheckResult, PermissionChecker};
pub use manager::{RoleChangeEvent, RoleChangeListener, RoleFilter, RoleManager, RoleManagerStats};
pub use policy::{
    Policy, PolicyCondition, PolicyDecision, PolicyEffect, PolicyEngine, PolicyEngineStats,
};
pub use types::*;

/// 向后兼容类型别名
pub type RbacManager = RoleManager;

use crate::error::LiteError;

/// RBAC错误类型
#[derive(Debug, Clone)]
pub enum RbacError {
    PermissionDenied(String),
    RoleNotFound(String),
    InvalidResource(String),
    PolicyViolation(String),
    CacheError(String),
}

impl std::fmt::Display for RbacError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RbacError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            RbacError::RoleNotFound(id) => write!(f, "Role not found: {}", id),
            RbacError::InvalidResource(msg) => write!(f, "Invalid resource: {}", msg),
            RbacError::PolicyViolation(msg) => write!(f, "Policy violation: {}", msg),
            RbacError::CacheError(msg) => write!(f, "Cache error: {}", msg),
        }
    }
}

impl std::error::Error for RbacError {}

impl From<RbacError> for LiteError {
    fn from(err: RbacError) -> Self {
        LiteError::Rbac(err.to_string())
    }
}

/// RBAC审计日志条目
#[derive(Debug, Clone)]
pub struct RbacAuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: String,
    pub action: String,
    pub resource: Resource,
    pub operation: Operation,
    pub allowed: bool,
    pub reason: Option<String>,
    pub context: Option<String>,
}

/// RBAC配置
#[derive(Debug, Clone)]
pub struct RbacConfig {
    /// 启用审计日志
    pub enable_audit: bool,
    /// 启用缓存
    pub enable_cache: bool,
    /// 缓存过期时间（秒）
    pub cache_ttl: u64,
    /// 超级管理员用户ID列表
    pub super_admins: Vec<String>,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            enable_audit: true,
            enable_cache: true,
            cache_ttl: 300, // 5分钟
            super_admins: Vec::new(),
        }
    }
}

/// 资源解析器trait
pub trait ResourceResolver: Send + Sync {
    /// 解析资源路径
    fn resolve(&self, path: &str) -> Result<Resource, RbacError>;
    /// 获取资源所有者
    fn get_owner(&self, resource: &Resource) -> Option<String>;
    /// 检查资源是否存在
    fn exists(&self, resource: &Resource) -> bool;
}

/// RBAC审计日志trait
pub trait RbacAuditLogger: Send + Sync {
    /// 记录访问尝试
    fn log_access(&self, entry: RbacAuditEntry);
    /// 记录权限变更
    fn log_permission_change(&self, user_id: &str, role_id: &str, action: &str);
}

/// 权限检查快捷宏
#[macro_export]
macro_rules! require_permission {
    ($checker:expr, $ctx:expr, $resource:expr, $operation:expr) => {
        match $checker.check($ctx, $resource, $operation).await {
            Ok(result) if result.allowed => (),
            Ok(result) => {
                return Err($crate::rbac::RbacError::PermissionDenied(
                    result.reason.unwrap_or_else(|| "Access denied".to_string()),
                )
                .into());
            }
            Err(e) => return Err(e.into()),
        }
    };
}

/// 批量权限检查宏
#[macro_export]
macro_rules! require_any_permission {
    ($checker:expr, $ctx:expr, $permissions:expr) => {
        match $checker.check_any($ctx, $permissions).await {
            Ok(result) if result.allowed => (),
            Ok(result) => {
                return Err($crate::rbac::RbacError::PermissionDenied(
                    result.reason.unwrap_or_else(|| "Access denied".to_string()),
                )
                .into());
            }
            Err(e) => return Err(e.into()),
        }
    };
}

/// 初始化默认系统角色
pub fn init_system_roles() -> Vec<RoleDefinition> {
    vec![
        create_super_admin_role(),
        create_team_admin_role(),
        create_team_member_role(),
        create_team_viewer_role(),
        create_personal_role(),
    ]
}

fn create_super_admin_role() -> RoleDefinition {
    let mut role = RoleDefinition::new("super_admin")
        .with_name("超级管理员")
        .with_description("系统超级管理员，拥有所有权限")
        .as_system();

    // 添加所有资源的所有权限
    for resource_type in ResourceType::all() {
        for operation in Operation::all() {
            role.add_permission(Permission::new(Resource::all(resource_type), operation));
        }
    }

    role
}

fn create_team_admin_role() -> RoleDefinition {
    let mut role = RoleDefinition::new("team_admin")
        .with_name("团队管理员")
        .with_description("团队管理员，可以管理团队和成员")
        .as_system();

    // 团队管理权限
    for operation in [
        Operation::Create,
        Operation::Read,
        Operation::Update,
        Operation::Delete,
    ] {
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Team),
            operation,
        ));
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Member),
            operation,
        ));
    }
    // 审计权限
    role.add_permission(Permission::new(
        Resource::all(ResourceType::AuditLog),
        Operation::Read,
    ));

    role
}

fn create_team_member_role() -> RoleDefinition {
    let mut role = RoleDefinition::new("team_member")
        .with_name("团队成员")
        .with_description("团队成员，可以管理服务器")
        .as_system();

    // 服务器相关权限
    for operation in [
        Operation::Create,
        Operation::Read,
        Operation::Update,
        Operation::Delete,
        Operation::Execute,
    ] {
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Server),
            operation,
        ));
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Session),
            operation,
        ));
    }
    // Snippet权限
    for operation in [
        Operation::Create,
        Operation::Read,
        Operation::Update,
        Operation::Delete,
    ] {
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Snippet),
            operation,
        ));
    }
    // Key权限
    for operation in [
        Operation::Create,
        Operation::Read,
        Operation::Update,
        Operation::Delete,
    ] {
        role.add_permission(Permission::new(Resource::all(ResourceType::Key), operation));
    }

    role
}

fn create_team_viewer_role() -> RoleDefinition {
    let mut role = RoleDefinition::new("team_viewer")
        .with_name("团队观察者")
        .with_description("团队观察者，只读访问")
        .as_system();

    // 只读权限
    role.add_permission(Permission::new(
        Resource::all(ResourceType::Server),
        Operation::Read,
    ));
    role.add_permission(Permission::new(
        Resource::all(ResourceType::Session),
        Operation::Read,
    ));
    role.add_permission(Permission::new(
        Resource::all(ResourceType::Snippet),
        Operation::Read,
    ));

    role
}

fn create_personal_role() -> RoleDefinition {
    let mut role = RoleDefinition::new("personal")
        .with_name("个人用户")
        .with_description("个人用户，管理自己的资源")
        .as_system();

    // 服务器相关权限
    for operation in [
        Operation::Create,
        Operation::Read,
        Operation::Update,
        Operation::Delete,
        Operation::Execute,
    ] {
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Server),
            operation,
        ));
        role.add_permission(Permission::new(Resource::all(ResourceType::Key), operation));
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Snippet),
            operation,
        ));
        role.add_permission(Permission::new(
            Resource::all(ResourceType::Layout),
            operation,
        ));
    }

    role
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_roles() {
        let roles = init_system_roles();
        assert_eq!(roles.len(), 5);

        let super_admin = roles.iter().find(|r| r.id == "super_admin").unwrap();
        assert!(super_admin.is_system);
        assert!(super_admin.has_permission(&Permission::new(
            Resource::all(ResourceType::System),
            Operation::Manage,
        )));
    }

    #[test]
    fn test_permission_macros() {
        // These are compile-time tests for the macros
        let ctx = CheckContext::new("user1");
        let resource = Resource::all(ResourceType::Server);
        let operation = Operation::Read;

        // The macro should expand correctly
        let _: CheckContext = ctx;
        let _: Resource = resource;
        let _: Operation = operation;
    }
}
