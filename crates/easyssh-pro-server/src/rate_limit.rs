use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::{models::ErrorResponse, AppState};

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
        }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(_state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, axum::Json<ErrorResponse>)> {
    // TODO: Implement actual rate limiting with Redis
    // For now, allow all requests
    Ok(next.run(request).await)
}
