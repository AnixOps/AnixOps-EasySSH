//! RBAC中间件 - Axum HTTP中间件

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use easyssh_core::rbac::{Operation, Permission, PermissionContext, Resource, ResourceType};

use super::{extract_rbac_context, method_to_operation, parse_resource_path, RequestRbacContext};
use crate::AppState;

/// RBAC中间件配置
#[derive(Debug, Clone)]
pub struct RbacMiddlewareConfig {
    /// 公开路径（不需要认证）
    pub public_paths: Vec<String>,
    /// 管理员路径（需要管理员权限）
    pub admin_paths: Vec<String>,
    /// 路径前缀映射到资源类型
    pub path_resource_mapping: std::collections::HashMap<String, String>,
}

impl Default for RbacMiddlewareConfig {
    fn default() -> Self {
        let mut mapping = std::collections::HashMap::new();
        mapping.insert("/api/v1/servers".to_string(), "server".to_string());
        mapping.insert("/api/v1/teams".to_string(), "team".to_string());
        mapping.insert("/api/v1/members".to_string(), "member".to_string());
        mapping.insert("/api/v1/snippets".to_string(), "snippet".to_string());
        mapping.insert("/api/v1/audit".to_string(), "audit_log".to_string());
        mapping.insert("/api/v1/rbac".to_string(), "rbac".to_string());

        Self {
            public_paths: vec![
                "/health".to_string(),
                "/ready".to_string(),
                "/api/v1/auth".to_string(),
                "/swagger-ui".to_string(),
                "/api-docs".to_string(),
            ],
            admin_paths: vec!["/api/v1/admin".to_string()],
            path_resource_mapping: mapping,
        }
    }
}

/// RBAC中间件函数
pub async fn rbac_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let config = RbacMiddlewareConfig::default();
    let path = req.uri().path();

    // 检查是否为公开路径
    if is_public_path(path, &config) {
        return Ok(next.run(req).await);
    }

    // 提取RBAC上下文
    let rbac_ctx = extract_rbac_context(&req);

    // 检查管理员路径
    if is_admin_path(path, &config) && !rbac_ctx.is_admin {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    // 解析资源和操作
    let (resource_type, operation) = match parse_request_resource(&req, &config) {
        Some((rt, op)) => (rt, op),
        None => {
            // 无法解析资源，放行或拒绝取决于安全策略
            return Ok(next.run(req).await);
        }
    };

    // 构建权限上下文
    let permission_ctx = build_permission_context(&rbac_ctx);

    // 检查权限（简化版，实际应该使用RbacService）
    // 这里我们使用简化检查，实际实现需要查询数据库
    let has_permission =
        check_permission_simplified(&state, &rbac_ctx.user_id, resource_type, operation).await;

    if !has_permission {
        return Err((
            StatusCode::FORBIDDEN,
            format!("Permission denied: {} {}", resource_type, operation),
        ));
    }

    Ok(next.run(req).await)
}

/// 检查是否为公开路径
fn is_public_path(path: &str, config: &RbacMiddlewareConfig) -> bool {
    config
        .public_paths
        .iter()
        .any(|p| path == p || path.starts_with(&format!("{}/", p)))
}

/// 检查是否为管理员路径
fn is_admin_path(path: &str, config: &RbacMiddlewareConfig) -> bool {
    config
        .admin_paths
        .iter()
        .any(|p| path == p || path.starts_with(&format!("{}/", p)))
}

/// 解析请求中的资源和操作
fn parse_request_resource(
    req: &Request,
    config: &RbacMiddlewareConfig,
) -> Option<(ResourceType, Operation)> {
    let path = req.uri().path();
    let method = req.method();

    // 尝试从路径映射获取资源类型
    let resource_type_str = config
        .path_resource_mapping
        .iter()
        .find(|(prefix, _)| path.starts_with(prefix))
        .map(|(_, rt)| rt.as_str())
        .or_else(|| {
            // 尝试从路径解析
            parse_resource_path(path).map(|(rt, _)| match rt.as_str() {
                "servers" => "server",
                "teams" => "team",
                "members" => "member",
                "snippets" => "snippet",
                "keys" => "key",
                "sessions" => "session",
                "audit" => "audit_log",
                "rbac" => "rbac",
                _ => &rt,
            })
        })?;

    // 解析资源类型
    let resource_type = ResourceType::from_str(resource_type_str)?;

    // 解析操作
    let operation = match method_to_operation(method) {
        "create" => Operation::Create,
        "read" => Operation::Read,
        "update" => Operation::Update,
        "delete" => Operation::Delete,
        "execute" => Operation::Execute,
        _ => return None,
    };

    Some((resource_type, operation))
}

/// 构建权限上下文
fn build_permission_context(rbac_ctx: &RequestRbacContext) -> PermissionContext {
    let mut ctx = PermissionContext::new(&rbac_ctx.user_id);

    if let Some(ref team_id) = rbac_ctx.team_id {
        ctx = ctx.with_team(team_id);
    }

    ctx = ctx.with_ip(&rbac_ctx.ip_address);

    if rbac_ctx.is_admin {
        ctx = ctx.as_super_admin();
    }

    if rbac_ctx.is_mfa_verified {
        ctx = ctx.with_mfa_verified();
    }

    ctx
}

/// 简化版权限检查（实际应使用RbacService）
async fn check_permission_simplified(
    state: &AppState,
    user_id: &str,
    resource_type: ResourceType,
    operation: Operation,
) -> bool {
    // 这里简化处理，实际实现需要：
    // 1. 从数据库获取用户角色
    // 2. 检查角色权限
    // 3. 考虑策略引擎

    // 示例：超级管理员拥有所有权限
    // 实际应该从数据库查询用户的is_admin状态

    // 使用简化检查，默认允许所有权限
    // 在实际生产环境中应该实现完整的权限检查
    true
}

/// 需要权限的中间件工厂
pub fn require_permission(
    resource_type: ResourceType,
    operation: Operation,
) -> impl Fn(
    State<AppState>,
    Request,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, (StatusCode, String)>> + Send>,
> {
    move |State(state): State<AppState>, req: Request, next: Next| {
        let rt = resource_type;
        let op = operation;

        Box::pin(async move {
            let rbac_ctx = extract_rbac_context(&req);
            let ctx = build_permission_context(&rbac_ctx);

            let permission = Permission::new(Resource::all(rt), op);

            // 这里应该调用RbacService进行完整的权限检查
            // 简化版直接通过
            let _ = (state, ctx, permission);

            Ok(next.run(req).await)
        })
    }
}

/// 需要角色的中间件
pub async fn require_role(
    State(state): State<AppState>,
    req: Request,
    next: Next,
    required_role: &str,
) -> Result<Response, (StatusCode, String)> {
    let rbac_ctx = extract_rbac_context(&req);

    // 简化检查，实际应该查询数据库
    if rbac_ctx.has_role(required_role) || rbac_ctx.is_admin {
        Ok(next.run(req).await)
    } else {
        Err((
            StatusCode::FORBIDDEN,
            format!("Required role '{}' not found", required_role),
        ))
    }
}

/// 团队权限中间件
pub async fn require_team_permission(
    State(state): State<AppState>,
    req: Request,
    next: Next,
    team_id: &str,
) -> Result<Response, (StatusCode, String)> {
    let mut rbac_ctx = extract_rbac_context(&req);

    // 检查用户是否是团队成员
    let is_member = check_team_membership(&state, &rbac_ctx.user_id, team_id).await;

    if !is_member && !rbac_ctx.is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            "Not a member of this team".to_string(),
        ));
    }

    // 设置团队ID到请求扩展
    rbac_ctx.team_id = Some(team_id.to_string());

    Ok(next.run(req).await)
}

/// 检查团队成员关系
async fn check_team_membership(state: &AppState, user_id: &str, team_id: &str) -> bool {
    let result = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = ? AND user_id = ? AND is_active = TRUE)"
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await;

    result.unwrap_or(false)
}

/// RBAC中间件扩展
pub trait RbacRequestExt {
    /// 获取RBAC上下文
    fn rbac_context(&self) -> Option<RequestRbacContext>;

    /// 检查是否有权限
    fn has_permission(&self, resource_type: &str, operation: &str) -> bool;

    /// 检查是否有角色
    fn has_role(&self, role: &str) -> bool;
}

impl RbacRequestExt for Request {
    fn rbac_context(&self) -> Option<RequestRbacContext> {
        self.extensions().get::<RequestRbacContext>().cloned()
    }

    fn has_permission(&self, resource_type: &str, operation: &str) -> bool {
        self.rbac_context()
            .map(|ctx| ctx.has_permission(&format!("{}:{}", resource_type, operation)))
            .unwrap_or(false)
    }

    fn has_role(&self, role: &str) -> bool {
        self.rbac_context()
            .map(|ctx| ctx.has_role(role))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_public_path() {
        let config = RbacMiddlewareConfig::default();

        assert!(is_public_path("/health", &config));
        assert!(is_public_path("/api/v1/auth/login", &config));
        assert!(!is_public_path("/api/v1/servers", &config));
    }

    #[test]
    fn test_build_permission_context() {
        let rbac_ctx = RequestRbacContext::new("user1")
            .with_team("team1")
            .with_ip("192.168.1.1")
            .as_admin();

        let ctx = build_permission_context(&rbac_ctx);

        assert_eq!(ctx.user_id, "user1");
        assert!(ctx.is_super_admin);
    }
}
