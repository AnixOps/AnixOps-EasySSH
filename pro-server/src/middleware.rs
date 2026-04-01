use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::{
    auth::decode_token,
    models::ErrorResponse,
    redis_cache::RedisCache,
    AppState,
};

pub mod rate_limit;
pub mod auth;

// Authentication middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, axum::Json<ErrorResponse>)> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            axum::Json(ErrorResponse {
                error: "missing_token".to_string(),
                message: "Authorization header missing".to_string(),
                code: Some("missing_token".to_string()),
                details: None,
            }),
        ))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or((
            StatusCode::UNAUTHORIZED,
            axum::Json(ErrorResponse {
                error: "invalid_token_format".to_string(),
                message: "Authorization header must start with 'Bearer '".to_string(),
                code: Some("invalid_token_format".to_string()),
                details: None,
            }),
        ))?;

    // Try JWT authentication first
    match decode_token(token, &state.config.jwt_secret) {
        Ok(claims) => {
            // Check if token is revoked
            let jti_hash = format!("{:x}", sha256::digest(token));
            if let Ok(Some(_)) = state.redis.get(&format!("revoked:{}", jti_hash)).await {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    axum::Json(ErrorResponse {
                        error: "token_revoked".to_string(),
                        message: "Token has been revoked".to_string(),
                        code: Some("token_revoked".to_string()),
                        details: None,
                    }),
                ));
            }

            // Add claims to request extensions
            request.extensions_mut().insert(claims);
            Ok(next.run(request).await)
        }
        Err(_) => {
            // Try API Key authentication
            let key_hash = format!("{:x}", sha256::digest(token));
            match state.redis.get_api_key_user(&key_hash).await {
                Ok(Some(user_id)) => {
                    // Add user info to request extensions
                    request.extensions_mut().insert(crate::auth::Claims {
                        sub: user_id,
                        email: "".to_string(),
                        exp: i64::MAX,
                        iat: 0,
                        jti: key_hash,
                        scopes: vec!["api".to_string()],
                    });
                    Ok(next.run(request).await)
                }
                _ => Err((
                    StatusCode::UNAUTHORIZED,
                    axum::Json(ErrorResponse {
                        error: "invalid_token".to_string(),
                        message: "Invalid or expired token/API key".to_string(),
                        code: Some("invalid_token".to_string()),
                        details: None,
                    }),
                )),
            }
        }
    }
}

use sha256;

// RBAC middleware factory
pub fn require_permission(resource_type: &'static str, action: &'static str) -> impl axum::middleware::Layer {
    // This would check permissions based on the user's role
    // Implementation depends on your RBAC service
    axum::middleware::from_fn(move |req: Request, next: Next| async move {
        // Check permission logic here
        next.run(req).await
    })
}

// Error handling middleware
pub async fn error_handling_middleware(
    req: Request,
    next: Next,
) -> Response {
    let response = next.run(req).await;

    // Log errors and transform if needed
    if response.status().is_server_error() {
        tracing::error!("Server error: {:?}", response.status());
    } else if response.status().is_client_error() {
        tracing::warn!("Client error: {:?}", response.status());
    }

    response
}

use sha256;
