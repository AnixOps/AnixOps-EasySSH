//! Pro服务器RBAC模块
//!
//! 提供服务器端的RBAC实现，包括：
//! - 资源解析器
//! - 权限缓存
//! - HTTP中间件
//! - 数据库集成

pub mod cache;
pub mod middleware;
pub mod resolver;

pub use cache::{PermissionCache, CacheConfig};
pub use middleware::{RbacMiddlewareConfig, rbac_middleware, require_permission, require_role, require_team_permission, RbacRequestExt};
pub use resolver::{DatabaseResourceResolver, ServerResourceResolver};

use axum::extract::Request;
use std::collections::HashMap;

/// 请求RBAC上下文
#[derive(Debug, Clone)]
pub struct RequestRbacContext {
    pub user_id: String,
    pub team_id: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub is_admin: bool,
    pub is_mfa_verified: bool,
}

impl RequestRbacContext {
    /// 创建新的请求上下文
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            team_id: None,
            roles: Vec::new(),
            permissions: Vec::new(),
            ip_address: "127.0.0.1".to_string(),
            user_agent: None,
            is_admin: false,
            is_mfa_verified: false,
        }
    }

    /// 设置团队ID
    pub fn with_team(mut self, team_id: impl Into<String>) -> Self {
        self.team_id = Some(team_id.into());
        self
    }

    /// 设置IP地址
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = ip.into();
        self
    }

    /// 设置管理员标志
    pub fn as_admin(mut self) -> Self {
        self.is_admin = true;
        self
    }

    /// 检查是否有特定权限
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    /// 检查是否有特定角色
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// 转换为core RBAC上下文
    pub fn to_core_context(&self) -> easyssh_core::rbac::PermissionContext {
        let mut ctx = easyssh_core::rbac::PermissionContext::new(&self.user_id);

        if let Some(team_id) = &self.team_id {
            ctx = ctx.with_team(team_id);
        }

        ctx = ctx.with_ip(&self.ip_address);

        if self.is_admin {
            ctx = ctx.as_super_admin();
        }

        if self.is_mfa_verified {
            ctx = ctx.with_mfa_verified();
        }

        ctx
    }
}

/// 从请求中提取RBAC上下文
pub fn extract_rbac_context(req: &Request) -> RequestRbacContext {
    // 从请求头或扩展中提取用户信息
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let team_id = req
        .headers()
        .get("x-team-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let ip_address = req
        .headers()
        .get("x-forwarded-for")
        .or_else(|| req.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    RequestRbacContext {
        user_id,
        team_id,
        ip_address,
        user_agent,
        roles: Vec::new(),
        permissions: Vec::new(),
        is_admin: false,
        is_mfa_verified: false,
    }
}

/// 资源路径解析
pub fn parse_resource_path(path: &str) -> Option<(String, Option<String>)> {
    // 解析类似 /api/v1/servers/{id} 的路径
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if parts.len() < 2 {
        return None;
    }

    // 确定资源类型
    let resource_type = parts[1].to_string();

    // 尝试获取资源ID
    let resource_id = parts.get(2).map(|s| s.to_string());

    Some((resource_type, resource_id))
}

/// HTTP方法到操作的映射
pub fn method_to_operation(method: &axum::http::Method) -> &'static str {
    match *method {
        axum::http::Method::GET => "read",
        axum::http::Method::POST => "create",
        axum::http::Method::PUT | axum::http::Method::PATCH => "update",
        axum::http::Method::DELETE => "delete",
        _ => "execute",
    }
}

/// 资源类型映射表
pub fn get_resource_type_mapping() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert("servers", "server");
    map.insert("teams", "team");
    map.insert("members", "member");
    map.insert("snippets", "snippet");
    map.insert("keys", "key");
    map.insert("sessions", "session");
    map.insert("audit", "audit_log");
    map.insert("rbac", "rbac");
    map.insert("resources", "server");
    map.insert("collaboration", "collaboration");
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_resource_path() {
        assert_eq!(
            parse_resource_path("/api/v1/servers"),
            Some(("api".to_string(), Some("v1".to_string())))
        );
        assert_eq!(
            parse_resource_path("/servers/server-123"),
            Some(("servers".to_string(), Some("server-123".to_string())))
        );
    }

    #[test]
    fn test_method_to_operation() {
        assert_eq!(method_to_operation(&axum::http::Method::GET), "read");
        assert_eq!(method_to_operation(&axum::http::Method::POST), "create");
        assert_eq!(method_to_operation(&axum::http::Method::DELETE), "delete");
    }

    #[test]
    fn test_request_rbac_context() {
        let ctx = RequestRbacContext::new("user1")
            .with_team("team1")
            .with_ip("192.168.1.1")
            .as_admin();

        assert_eq!(ctx.user_id, "user1");
        assert_eq!(ctx.team_id, Some("team1".to_string()));
        assert_eq!(ctx.ip_address, "192.168.1.1");
        assert!(ctx.is_admin);
    }
}
