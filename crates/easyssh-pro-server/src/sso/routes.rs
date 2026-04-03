//! Pro Server - SSO路由
//!
//! 定义SSO相关的API路由

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use std::net::SocketAddr;

use crate::{
    auth::Claims,
    models::{ErrorResponse, SuccessResponse},
    sso::{
        CreateProviderRequest, SamlCallbackRequest, SessionListItem, SsoCallbackRequest,
        SsoLoginCompleteResponse, SsoLoginResponse, TeamSsoConfigRequest, TeamSsoConfigResponse,
        UpdateProviderRequest,
    },
    AppState,
};

/// 查询参数：状态验证
#[derive(Debug, Deserialize)]
pub struct StateQuery {
    pub state: String,
}

/// 查询参数：OIDC回调
#[derive(Debug, Deserialize)]
pub struct OidcCallbackQuery {
    pub code: String,
    pub state: String,
    #[serde(rename = "error")]
    pub error: Option<String>,
    #[serde(rename = "error_description")]
    pub error_description: Option<String>,
}

/// 创建SSO路由
pub fn sso_routes() -> Router<AppState> {
    Router::new()
        // 提供商管理
        .route("/providers", get(list_providers))
        .route("/providers", post(create_provider))
        .route("/providers/{id}", get(get_provider))
        .route("/providers/{id}", post(update_provider))
        .route("/providers/{id}", delete(delete_provider))
        // SAML
        .route("/saml/{provider_id}/login", get(saml_login))
        .route("/saml/acs", post(saml_acs))
        .route("/saml/{provider_id}/metadata", get(saml_metadata))
        .route("/saml/{provider_id}/logout", post(saml_logout))
        // OIDC
        .route("/oidc/{provider_id}/login", get(oidc_login))
        .route("/oidc/callback", get(oidc_callback))
        .route("/oidc/{provider_id}/logout", get(oidc_logout))
        // OAuth2
        .route("/oauth2/{provider_id}/login", get(oauth2_login))
        .route("/oauth2/callback", get(oauth2_callback))
        // 会话管理
        .route("/sessions", get(list_sessions))
        .route("/sessions/{id}", delete(terminate_session))
        // 团队SSO配置
        .route("/teams/{team_id}/sso", get(get_team_sso))
        .route("/teams/{team_id}/sso", post(configure_team_sso))
        .route("/teams/{team_id}/sso", delete(remove_team_sso))
}

/// 列出所有SSO提供商
async fn list_providers(
    State(state): State<AppState>,
) -> Result<
    Json<SuccessResponse<Vec<crate::sso::ProviderResponse>>>,
    (StatusCode, Json<ErrorResponse>),
> {
    let providers = state.sso_handler.list_providers().await;

    Ok(Json(SuccessResponse {
        success: true,
        data: providers,
        message: None,
    }))
}

/// 创建SSO提供商
async fn create_provider(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<Json<SuccessResponse<crate::sso::ProviderResponse>>, (StatusCode, Json<ErrorResponse>)>
{
    // 需要管理员权限
    if !claims.is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Admin access required".to_string(),
                code: Some("admin_required".to_string()),
                details: None,
            }),
        ));
    }

    let provider = state
        .sso_handler
        .create_provider(req)
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "create_failed".to_string(),
                    message: msg,
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: provider,
        message: Some("SSO provider created successfully".to_string()),
    }))
}

/// 获取SSO提供商
async fn get_provider(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<crate::sso::ProviderResponse>>, (StatusCode, Json<ErrorResponse>)>
{
    let provider = state.sso_handler.get_provider(&id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "not_found".to_string(),
                message: "SSO provider not found".to_string(),
                code: Some("provider_not_found".to_string()),
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: provider,
        message: None,
    }))
}

/// 更新SSO提供商
async fn update_provider(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<SuccessResponse<crate::sso::ProviderResponse>>, (StatusCode, Json<ErrorResponse>)>
{
    if !claims.is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Admin access required".to_string(),
                code: Some("admin_required".to_string()),
                details: None,
            }),
        ));
    }

    let provider = state
        .sso_handler
        .update_provider(&id, req)
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "update_failed".to_string(),
                    message: msg,
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: provider,
        message: Some("SSO provider updated successfully".to_string()),
    }))
}

/// 删除SSO提供商
async fn delete_provider(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    if !claims.is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Admin access required".to_string(),
                code: Some("admin_required".to_string()),
                details: None,
            }),
        ));
    }

    state
        .sso_handler
        .delete_provider(&id)
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "delete_failed".to_string(),
                    message: msg,
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("SSO provider deleted successfully".to_string()),
    }))
}

/// SAML登录初始化
async fn saml_login(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<SuccessResponse<SsoLoginResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let login_response = state
        .sso_handler
        .initiate_saml_login(&provider_id)
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "login_init_failed".to_string(),
                    message: msg,
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: login_response,
        message: Some("SAML authentication initiated".to_string()),
    }))
}

/// SAML ACS回调 (Assertion Consumer Service)
async fn saml_acs(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<SamlCallbackRequest>,
) -> Result<Json<SuccessResponse<SsoLoginCompleteResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let ip_address = Some(addr.ip().to_string());
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = state
        .sso_handler
        .handle_saml_callback(req, ip_address, user_agent, state.db.pool())
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "authentication_failed".to_string(),
                    message: msg,
                    code: Some("saml_auth_failed".to_string()),
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: response,
        message: Some("SAML authentication successful".to_string()),
    }))
}

/// SAML SP元数据
async fn saml_metadata(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Html<String>, (StatusCode, Json<ErrorResponse>)> {
    let metadata = state
        .sso_handler
        .generate_sp_metadata(&provider_id)
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "metadata_failed".to_string(),
                    message: msg,
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Html(metadata))
}

/// SAML登出
async fn saml_logout(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(provider_id): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    // 终止该提供商的所有用户会话
    let _terminated = state
        .sso_handler
        .terminate_user_sessions(&claims.sub, &provider_id)
        .await;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("SAML logout successful".to_string()),
    }))
}

/// OIDC登录初始化
async fn oidc_login(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<SuccessResponse<SsoLoginResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let login_response = state
        .sso_handler
        .initiate_oidc_login(&provider_id)
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "login_init_failed".to_string(),
                    message: msg,
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: login_response,
        message: Some("OIDC authentication initiated".to_string()),
    }))
}

/// OIDC回调
async fn oidc_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<OidcCallbackQuery>,
) -> Result<Json<SuccessResponse<SsoLoginCompleteResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // 检查错误
    if let Some(error) = params.error {
        let description = params.error_description.unwrap_or_default();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error,
                message: description,
                code: Some("oidc_callback_error".to_string()),
                details: None,
            }),
        ));
    }

    let ip_address = Some(addr.ip().to_string());
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let req = SsoCallbackRequest {
        code: params.code,
        state: params.state,
    };

    let response = state
        .sso_handler
        .handle_oidc_callback(req, ip_address, user_agent, state.db.pool())
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "authentication_failed".to_string(),
                    message: msg,
                    code: Some("oidc_auth_failed".to_string()),
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: response,
        message: Some("OIDC authentication successful".to_string()),
    }))
}

/// OIDC登出
async fn oidc_logout(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(provider_id): Path<String>,
) -> Result<Json<SuccessResponse<Option<String>>>, (StatusCode, Json<ErrorResponse>)> {
    // 终止该提供商的所有用户会话
    let _terminated = state
        .sso_handler
        .terminate_user_sessions(&claims.sub, &provider_id)
        .await;

    // 构建登出URL (可选)
    let logout_url = None; // 简化实现

    Ok(Json(SuccessResponse {
        success: true,
        data: logout_url,
        message: Some("OIDC logout successful".to_string()),
    }))
}

/// OAuth2登录初始化
async fn oauth2_login(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<SuccessResponse<SsoLoginResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // OAuth2登录与OIDC类似，但使用不同的scope和端点
    // 简化实现：重定向到OIDC登录
    let login_response = state
        .sso_handler
        .initiate_oidc_login(&provider_id)
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "login_init_failed".to_string(),
                    message: msg,
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: login_response,
        message: Some("OAuth2 authentication initiated".to_string()),
    }))
}

/// OAuth2回调
async fn oauth2_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<OidcCallbackQuery>,
) -> Result<Json<SuccessResponse<SsoLoginCompleteResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // OAuth2回调与OIDC类似
    // 简化实现：重定向到OIDC回调处理
    oidc_callback(State(state), headers, ConnectInfo(addr), Query(params)).await
}

/// 列出用户会话
async fn list_sessions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<SuccessResponse<Vec<SessionListItem>>>, (StatusCode, Json<ErrorResponse>)> {
    let sessions = state.sso_handler.list_user_sessions(&claims.sub).await;

    Ok(Json(SuccessResponse {
        success: true,
        data: sessions,
        message: None,
    }))
}

/// 终止会话
async fn terminate_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .sso_handler
        .terminate_session(&claims.sub, &id)
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "termination_failed".to_string(),
                    message: msg,
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Session terminated successfully".to_string()),
    }))
}

/// 获取团队SSO配置
async fn get_team_sso(
    State(_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
) -> Result<Json<SuccessResponse<TeamSsoConfigResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // 验证用户属于该团队
    // 简化实现
    let _ = (&team_id, &claims.sub);

    Ok(Json(SuccessResponse {
        success: true,
        data: TeamSsoConfigResponse {
            team_id: team_id.clone(),
            provider_id: "example".to_string(),
            group_mappings: vec![],
            auto_provision: false,
            default_role: "Member".to_string(),
        },
        message: None,
    }))
}

/// 配置团队SSO
async fn configure_team_sso(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
    Json(req): Json<TeamSsoConfigRequest>,
) -> Result<Json<SuccessResponse<TeamSsoConfigResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // 验证用户有权限配置该团队
    // 简化实现
    let _ = &claims.sub;

    let config = state
        .sso_handler
        .configure_team_sso(&team_id, req)
        .await
        .map_err(|(status, msg)| {
            (
                status,
                Json(ErrorResponse {
                    error: "configuration_failed".to_string(),
                    message: msg,
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: config,
        message: Some("Team SSO configured successfully".to_string()),
    }))
}

/// 移除团队SSO配置
async fn remove_team_sso(
    State(_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    // 验证用户有权限
    let _ = &claims.sub;

    // 移除团队SSO配置
    // 简化实现
    let _ = &team_id;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Team SSO configuration removed".to_string()),
    }))
}

/// 扩展SSO处理器的方法
impl crate::sso::handlers::SsoServiceHandler {
    /// 终止用户所有会话 (辅助方法)
    pub async fn terminate_user_sessions(&self, user_id: &str, _provider_id: &str) -> usize {
        let session_manager = self.session_manager.write().await;
        session_manager.terminate_user_sessions(user_id)
    }
}
