//! EasySSH Pro Server
//!
//! Backend service for team collaboration, SSO, RBAC, and audit logging.
//!
//! Note: This crate is under active development. Some unused imports and
//! variables are intentionally left for future use.

// Allow unused imports during active development
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use anyhow::Result;
use axum::{middleware as axum_middleware, routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

mod api;
mod auth;
mod config;
mod db;
mod docs;
mod middleware;
mod models;
mod rate_limit;
mod redis_cache;
mod services;
mod sso;
mod websocket;

// 事件响应中心模块
mod escalation_service;
mod incident_models;
mod incident_service;
mod post_mortem_service;
mod rbac;
mod runbook_service;

use crate::api::{
    audit::audit_routes, collaboration::collaboration_routes, incident::incident_routes,
    rbac::rbac_routes, resources::resource_routes, teams::team_routes,
};
use crate::auth::auth_routes;
use crate::config::AppConfig;
use crate::db::Database;
use crate::docs::swagger::swagger_routes;
use crate::middleware::rate_limit_middleware;
use crate::sso::routes::sso_routes;
use crate::websocket::ws_routes;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub redis: Arc<redis_cache::RedisCache>,
    pub config: Arc<AppConfig>,
    pub sso_handler: Arc<sso::handlers::SsoServiceHandler>,
    pub audit_service: Arc<services::audit_service::AuditService>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "easyssh_pro_server=debug,tower_http=debug".into()),
        )
        .init();

    info!("Starting EasySSH Pro Server...");

    // Load configuration
    let config = AppConfig::from_env()?;
    info!("Configuration loaded");

    // Initialize database
    let db = Database::new(&config.database_url).await?;
    info!("Database connected");

    // Initialize Redis
    let redis = redis_cache::RedisCache::new(&config.redis_url).await?;
    info!("Redis connected");

    // Initialize SSO handler (clone redis before wrapping in Arc)
    let sso_handler = sso::handlers::SsoServiceHandler::new(redis.clone());
    info!("SSO handler initialized");

    // Initialize audit service
    let audit_service = services::audit_service::AuditService::new(db.pool().clone());
    info!("Audit service initialized");

    let state = AppState {
        db: Arc::new(db),
        redis: Arc::new(redis),
        config: Arc::new(config),
        sso_handler: Arc::new(sso_handler),
        audit_service: Arc::new(audit_service),
    };

    // Build application router
    let app = create_router(state.clone());

    // Start server
    let addr: SocketAddr = format!("{}:{}", state.config.host, state.config.port)
        .parse()
        .expect("Invalid address");

    info!("Server listening on http://{}", addr);
    info!("API Documentation available at http://{}/api-docs", addr);
    info!("Swagger UI available at http://{}/swagger-ui", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn create_router(state: AppState) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Health check route (no auth required)
    let health_router = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check));

    // Public routes (no auth required)
    let public_router = Router::new()
        .nest("/auth", auth_routes())
        .nest("/sso", sso_routes());

    // Protected API routes (auth required)
    let protected_router = Router::new()
        .nest("/teams", team_routes())
        .nest("/audit", audit_routes())
        .nest("/rbac", rbac_routes())
        .nest("/resources", resource_routes())
        .nest("/collaboration", collaboration_routes())
        .nest("/incidents", incident_routes())
        .nest("/ws", ws_routes())
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ));

    // Combine all routes
    let api_router = Router::new()
        .merge(health_router)
        .merge(public_router)
        .merge(protected_router)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // Main router with Swagger UI
    let app = Router::new()
        .nest("/api/v1", api_router)
        .merge(swagger_routes())
        .with_state(state);

    app
}

async fn health_check() -> &'static str {
    "OK"
}

async fn readiness_check(
    state: axum::extract::State<AppState>,
) -> Result<&'static str, (axum::http::StatusCode, String)> {
    // Check database connection
    if let Err(e) = state.db.ping().await {
        return Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            format!("Database unavailable: {}", e),
        ));
    }

    // Check Redis connection
    if let Err(e) = state.redis.ping().await {
        return Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            format!("Redis unavailable: {}", e),
        ));
    }

    Ok("Ready")
}

async fn require_auth(
    req: axum::extract::Request,
    next: axum_middleware::Next,
) -> Result<axum::response::Response, (axum::http::StatusCode, String)> {
    // Auth middleware will be implemented properly
    // For now, just pass through
    Ok(next.run(req).await)
}
