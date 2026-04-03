//! Pro Server - SSO中间件
//!
//! 提供SSO会话验证和安全的中间件

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;

use crate::{auth::Claims, AppState};

/// SSO会话验证中间件
pub async fn sso_session_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    // 从Authorization头提取SSO会话令牌
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    if let Some(_token) = auth_header {
        // 验证SSO会话
        // 简化实现：假设验证通过
    }

    // 继续处理请求
    next.run(request).await
}

/// SSO安全头中间件
pub async fn sso_security_headers_middleware(request: Request, next: Next) -> Response {
    let response = next.run(request).await;

    // 添加安全头
    let security_headers = [
        ("X-SSO-Protected", "true"),
        ("X-Content-Type-Options", "nosniff"),
        ("X-Frame-Options", "DENY"),
    ];

    let mut response = response;

    for (name, value) in security_headers {
        response = response.map(|body| {
            let mut response = Response::new(body);
            response.headers_mut().insert(
                name.parse::<axum::http::header::HeaderName>().unwrap(),
                value.parse().unwrap(),
            );
            response
        });
    }

    response
}

/// 速率限制中间件 (SSO端点)
pub async fn sso_rate_limit_middleware(
    State(_state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    // 检查IP速率限制
    let ip = addr.ip().to_string();
    let _ = ip;

    // 简化实现：假设通过
    // 实际应查询Redis检查该IP的请求频率

    Ok(next.run(request).await)
}

/// CSRF保护中间件
pub async fn csrf_protection_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    // 检查CSRF令牌 (对于POST/PUT/DELETE请求)
    if request.method() != axum::http::Method::GET {
        let csrf_token = headers.get("X-CSRF-Token");

        // 简化实现：假设验证通过
        // 实际应验证CSRF令牌与session关联
        let _ = csrf_token;
    }

    Ok(next.run(request).await)
}

/// 审计日志中间件
pub async fn sso_audit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let start_time = std::time::Instant::now();
    let path = request.uri().path().to_string();
    let method = request.method().to_string();
    let ip = addr.ip().to_string();
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // 执行请求
    let response = next.run(request).await;
    let status = response.status();
    let duration = start_time.elapsed();

    // 记录审计日志 (异步)
    // 简化实现：只记录日志
    if path.starts_with("/api/sso") {
        tracing::info!(
            "SSO Audit: method={}, path={}, ip={}, status={}, duration={}ms, ua={}",
            method,
            path,
            ip,
            status.as_u16(),
            duration.as_millis(),
            user_agent
        );
    }

    // 发送到审计服务
    let _ = state.audit_service;

    response
}

/// 提供方验证中间件
pub async fn provider_validation_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    // 从路径提取provider_id
    let path = request.uri().path();

    // 检查路径是否包含provider_id
    // /api/sso/saml/{provider_id}/...
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() >= 5 && parts[3] == "sso" {
        if let Some(provider_id) = parts.get(4) {
            // 验证提供商存在且启用
            let provider_exists = state
                .sso_handler
                .get_provider(provider_id)
                .await
                .map(|p| p.enabled)
                .unwrap_or(false);

            if !provider_exists {
                return Err((StatusCode::NOT_FOUND, "SSO provider not found or disabled"));
            }
        }
    }

    Ok(next.run(request).await)
}
