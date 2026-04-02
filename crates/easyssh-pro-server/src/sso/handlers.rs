//! Pro Server - SSO处理器
//!
//! 处理SAML和OIDC的认证流程

use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::StatusCode;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

use easyssh_core::sso::{
    ConflictResolutionStrategy, IdentityConflictResolver, IdentityMapper,
    JustInTimeProvisioning, JitProvisioningConfig, OidcHandler, OidcTokenResponse,
    OidcUserInfo, SamlAuthResponse, SamlConfig, SamlHandler, SsoManager, SsoProvider,
    SsoProviderConfig, SsoProviderType, SsoSession, SsoSessionManager,
};

use crate::{
    models::*,
    redis_cache::RedisCache,
    services::auth_service::AuthService,
    sso::{
        SamlCallbackRequest, SsoCallbackRequest, SsoLoginCompleteResponse,
        SsoLoginRequest, SsoLoginResponse, SsoUserResponse,
    },
    AppState,
};

use super::{
    CreateProviderRequest, ProviderResponse, SessionListItem, TeamSsoConfigRequest,
    TeamSsoConfigResponse, UpdateProviderRequest,
};

/// SSO服务处理器
pub struct SsoServiceHandler {
    sso_manager: Arc<tokio::sync::RwLock<SsoManager>>,
    session_manager: Arc<tokio::sync::RwLock<SsoSessionManager>>,
    jit_provisioning: Arc<tokio::sync::RwLock<JustInTimeProvisioning>>,
    redis: RedisCache,
}

impl SsoServiceHandler {
    /// 创建新的SSO服务处理器
    pub fn new(redis: RedisCache) -> Self {
        let sso_manager = Arc::new(tokio::sync::RwLock::new(SsoManager::new()));
        let session_manager = Arc::new(tokio::sync::RwLock::new(
            SsoSessionManager::new().with_config(5, 8, false),
        ));
        let jit_provisioning = Arc::new(tokio::sync::RwLock::new(
            JustInTimeProvisioning::default_provisioning(),
        ));

        Self {
            sso_manager,
            session_manager,
            jit_provisioning,
            redis,
        }
    }

    /// 创建SSO提供商
    pub async fn create_provider(
        &self,
        req: CreateProviderRequest,
    ) -> Result<ProviderResponse, (StatusCode, String)> {
        let provider = match req.provider_type {
            SsoProviderType::Saml => {
                let config: SamlConfig = serde_json::from_value(req.config).map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Invalid SAML config: {}", e))
                })?;
                SsoProvider::new_saml(&req.name, config)
            }
            SsoProviderType::Oidc => {
                let config: easyssh_core::sso::OidcConfig =
                    serde_json::from_value(req.config).map_err(|e| {
                        (StatusCode::BAD_REQUEST, format!("Invalid OIDC config: {}", e))
                    })?;
                SsoProvider::new_oidc(&req.name, config)
            }
            _ => {
                return Err((StatusCode::BAD_REQUEST, "Unsupported provider type".to_string()));
            }
        };

        let response = provider.clone().into();

        let mut manager = self.sso_manager.write().await;
        manager.add_provider(provider).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to add provider: {}", e))
        })?;

        // 存储到Redis缓存
        let _ = self.redis.cache_provider(&response.id, &response).await;

        Ok(response)
    }

    /// 获取SSO提供商
    pub async fn get_provider(&self, provider_id: &str) -> Option<ProviderResponse> {
        // 先尝试从Redis获取
        if let Ok(Some(cached)) = self.redis.get_cached_provider(provider_id).await {
            return Some(cached);
        }

        // 从内存获取
        let manager = self.sso_manager.read().await;
        manager.get_provider(provider_id).map(|p| p.clone().into())
    }

    /// 列出所有提供商
    pub async fn list_providers(&self) -> Vec<ProviderResponse> {
        let manager = self.sso_manager.read().await;
        manager
            .list_providers()
            .into_iter()
            .map(|p| p.clone().into())
            .collect()
    }

    /// 更新提供商
    pub async fn update_provider(
        &self,
        provider_id: &str,
        req: UpdateProviderRequest,
    ) -> Result<ProviderResponse, (StatusCode, String)> {
        let mut manager = self.sso_manager.write().await;

        let provider = manager
            .get_provider_mut(provider_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Provider not found".to_string()))?;

        if let Some(name) = req.name {
            provider.name = name;
        }

        if let Some(config) = req.config {
            match provider.provider_type {
                SsoProviderType::Saml => {
                    let new_config: SamlConfig = serde_json::from_value(config).map_err(|e| {
                        (StatusCode::BAD_REQUEST, format!("Invalid SAML config: {}", e))
                    })?;
                    provider.config = SsoProviderConfig::Saml(new_config);
                }
                SsoProviderType::Oidc => {
                    let new_config: easyssh_core::sso::OidcConfig =
                        serde_json::from_value(config).map_err(|e| {
                            (StatusCode::BAD_REQUEST, format!("Invalid OIDC config: {}", e))
                        })?;
                    provider.config = SsoProviderConfig::Oidc(new_config);
                }
                _ => {}
            }
        }

        if let Some(enabled) = req.enabled {
            if enabled {
                provider.enable();
            } else {
                provider.disable();
            }
        }

        let response: ProviderResponse = provider.clone().into();

        // 更新Redis缓存
        let _ = self.redis.cache_provider(provider_id, &response).await;

        Ok(response)
    }

    /// 删除提供商
    pub async fn delete_provider(&self, provider_id: &str) -> Result<(), (StatusCode, String)> {
        let mut manager = self.sso_manager.write().await;

        manager.remove_provider(provider_id).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to remove provider: {}", e))
        })?;

        // 终止该提供商的所有会话
        let mut session_manager = self.session_manager.write().await;
        session_manager.terminate_provider_sessions(provider_id);

        // 从Redis删除
        let _ = self.redis.delete_cached_provider(provider_id).await;

        Ok(())
    }

    /// 初始化SAML登录
    pub async fn initiate_saml_login(
        &self,
        provider_id: &str,
    ) -> Result<SsoLoginResponse, (StatusCode, String)> {
        let manager = self.sso_manager.read().await;

        let provider = manager
            .get_provider(provider_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Provider not found".to_string()))?;

        let handler = SamlHandler::new(provider.clone()).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create SAML handler: {}", e))
        })?;

        let auth_request = handler.create_auth_request().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create auth request: {}", e))
        })?;

        // 存储state到Redis (5分钟过期)
        let state_data = serde_json::json!({
            "provider_id": provider_id,
            "created_at": Utc::now().to_rfc3339(),
        });
        let _ = self
            .redis
            .set(&format!("sso_state:{}", auth_request.relay_state.as_ref().unwrap()),
                &state_data.to_string(),
                std::time::Duration::from_secs(300),
            )
            .await;

        Ok(SsoLoginResponse {
            login_url: format!(
                "{}?SAMLRequest={}&RelayState={}",
                auth_request.destination,
                urlencoding::encode(&auth_request.saml_request),
                urlencoding::encode(auth_request.relay_state.as_ref().unwrap())
            ),
            state: auth_request.relay_state.unwrap(),
            nonce: String::new(), // SAML不使用nonce
            expires_in: 300,
        })
    }

    /// 初始化OIDC登录
    pub async fn initiate_oidc_login(
        &self,
        provider_id: &str,
    ) -> Result<SsoLoginResponse, (StatusCode, String)> {
        let manager = self.sso_manager.read().await;

        let provider = manager
            .get_provider(provider_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Provider not found".to_string()))?;

        let handler = OidcHandler::new(provider.clone()).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create OIDC handler: {}", e))
        })?;

        let state = easyssh_core::sso::generate_secure_random(32);
        let nonce = easyssh_core::sso::generate_secure_random(32);

        let (auth_url, pkce_verifier) = handler
            .build_authorization_url(&state, &nonce)
            .map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to build auth URL: {}", e))
            })?;

        // 存储state、nonce和pkce_verifier到Redis
        let state_data = serde_json::json!({
            "provider_id": provider_id,
            "nonce": nonce,
            "pkce_verifier": pkce_verifier,
            "created_at": Utc::now().to_rfc3339(),
        });
        let _ = self
            .redis
            .set(
                &format!("sso_state:{}", state),
                &state_data.to_string(),
                std::time::Duration::from_secs(300),
            )
            .await;

        Ok(SsoLoginResponse {
            login_url: auth_url,
            state,
            nonce,
            expires_in: 300,
        })
    }

    /// 处理SAML回调
    pub async fn handle_saml_callback(
        &self,
        req: SamlCallbackRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
        db_pool: &sqlx::Pool<sqlx::Any>,
    ) -> Result<SsoLoginCompleteResponse, (StatusCode, String)> {
        // 验证state
        let state_data = if let Some(ref relay_state) = req.relay_state {
            let data = self.redis.get(&format!("sso_state:{}", relay_state)).await.map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Redis error: {}", e))
            })?;
            data
        } else {
            None
        };

        let provider_id = if let Some(data) = state_data {
            let json: serde_json::Value = serde_json::from_str(&data).map_err(|e| {
                (StatusCode::BAD_REQUEST, format!("Invalid state data: {}", e))
            })?;
            json["provider_id"]
                .as_str()
                .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing provider_id in state".to_string()))?
                .to_string()
        } else {
            return Err((StatusCode::BAD_REQUEST, "Invalid or expired state".to_string()));
        };

        // 获取提供商
        let manager = self.sso_manager.read().await;
        let provider = manager
            .get_provider(&provider_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Provider not found".to_string()))?;

        let handler = SamlHandler::new(provider.clone()).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create handler: {}", e))
        })?;

        // 处理SAML响应
        let sso_user = handler
            .process_saml_response(&req.saml_response, req.relay_state.as_deref())
            .map_err(|e| (StatusCode::UNAUTHORIZED, format!("SAML validation failed: {}", e)))?;

        // 清理state
        if let Some(ref relay_state) = req.relay_state {
            let _ = self.redis.delete(&format!("sso_state:{}", relay_state)).await;
        }

        // 执行JIT开通
        let (user, is_new_user) = self
            .provision_or_link_user(&sso_user, &provider, db_pool)
            .await?;

        // 创建会话
        let session = self
            .create_session(&user.id, &provider_id, ip_address, user_agent)
            .await?;

        // 生成JWT令牌
        let (access_token, refresh_token, expires_in) = self
            .generate_tokens(&user.id, &user.email, &sso_user.groups)
            .await?;

        Ok(SsoLoginCompleteResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
            user: user.into(),
            is_new_user,
        })
    }

    /// 处理OIDC回调
    pub async fn handle_oidc_callback(
        &self,
        req: SsoCallbackRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
        db_pool: &sqlx::Pool<sqlx::Any>,
    ) -> Result<SsoLoginCompleteResponse, (StatusCode, String)> {
        // 验证state并获取相关数据
        let state_data = self
            .redis
            .get(&format!("sso_state:{}", req.state))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Redis error: {}", e)))?
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid or expired state".to_string()))?;

        let state_json: serde_json::Value = serde_json::from_str(&state_data).map_err(|e| {
            (StatusCode::BAD_REQUEST, format!("Invalid state data: {}", e))
        })?;

        let provider_id = state_json["provider_id"]
            .as_str()
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing provider_id".to_string()))?;

        let nonce = state_json["nonce"]
            .as_str()
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing nonce".to_string()))?;

        let pkce_verifier = state_json["pkce_verifier"].as_str();

        // 获取提供商
        let manager = self.sso_manager.read().await;
        let provider = manager
            .get_provider(provider_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Provider not found".to_string()))?;

        let handler = OidcHandler::new(provider.clone()).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create handler: {}", e))
        })?;

        // 交换code获取token
        let token_response = handler
            .exchange_code(&req.code, pkce_verifier)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token exchange failed: {}", e)))?;

        // 验证ID Token
        let oidc_user = handler
            .validate_id_token(&token_response.id_token, nonce)
            .map_err(|e| (StatusCode::UNAUTHORIZED, format!("ID token validation failed: {}", e)))?;

        // 转换为SsoUserInfo
        let sso_user = handler.convert_to_sso_user_info(oidc_user);

        // 清理state
        let _ = self.redis.delete(&format!("sso_state:{}", req.state)).await;

        // 执行JIT开通
        let (user, is_new_user) = self
            .provision_or_link_user(&sso_user, &provider, db_pool)
            .await?;

        // 创建会话
        let session = self
            .create_session_with_tokens(
                &user.id,
                provider_id,
                &token_response.access_token,
                Some(&token_response.id_token),
                Some(&token_response.access_token),
                token_response.refresh_token.as_deref(),
                ip_address,
                user_agent,
            )
            .await?;

        // 生成JWT令牌
        let (access_token, refresh_token, expires_in) = self
            .generate_tokens(&user.id, &user.email, &sso_user.groups)
            .await?;

        Ok(SsoLoginCompleteResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
            user: user.into(),
            is_new_user,
        })
    }

    /// 开通或链接用户
    async fn provision_or_link_user(
        &self,
        sso_user: &easyssh_core::sso::SsoUserInfo,
        provider: &SsoProvider,
        db_pool: &sqlx::Pool<sqlx::Any>,
    ) -> Result<(crate::models::User, bool), (StatusCode, String)> {
        let auth_service = AuthService::new(db_pool.clone(), self.redis.clone());

        // 检查用户是否已存在
        let existing_user = auth_service.get_user_by_email(&sso_user.email).await.ok();

        let is_new_user = existing_user.is_none();

        let user = if let Some(user) = existing_user {
            // 用户已存在，更新SSO信息
            user
        } else {
            // 新用户，执行JIT开通
            let mut jit = self.jit_provisioning.write().await;

            // 获取现有用户列表用于冲突检测
            let existing_users: Vec<easyssh_core::sso::ExistingUserInfo> = vec![]; // 简化实现

            let record = jit
                .provision_user(sso_user, provider, &existing_users)
                .await
                .map_err(|e| {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Provisioning failed: {}", e))
                })?;

            // 获取或创建用户
            auth_service
                .get_user_by_id(&record.created_user_id.unwrap_or_default())
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("User not found: {}", e)))?
        };

        Ok((user, is_new_user))
    }

    /// 创建会话
    async fn create_session(
        &self,
        user_id: &str,
        provider_id: &str,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<SsoSession, (StatusCode, String)> {
        let mut session_manager = self.session_manager.write().await;

        let session = session_manager.create_session(
            user_id,
            provider_id,
            ip_address.as_deref(),
            user_agent.as_deref(),
        );

        // 存储会话到Redis
        let session_key = format!("sso_session:{}", session.id);
        let session_json = serde_json::to_string(&session).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize session: {}", e))
        })?;

        let _ = self
            .redis
            .set(
                &session_key,
                &session_json,
                std::time::Duration::from_secs(8 * 3600),
            )
            .await;

        Ok(session)
    }

    /// 使用令牌创建会话
    async fn create_session_with_tokens(
        &self,
        user_id: &str,
        provider_id: &str,
        sso_token: &str,
        id_token: Option<&str>,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<SsoSession, (StatusCode, String)> {
        // 注意：这需要访问CRYPTO_STATE来加密令牌
        // 简化实现：创建普通会话
        self.create_session(user_id, provider_id, ip_address, user_agent)
            .await
    }

    /// 生成JWT令牌
    async fn generate_tokens(
        &self,
        user_id: &str,
        email: &str,
        _groups: &[String],
    ) -> Result<(String, String, i64), (StatusCode, String)> {
        // 使用与常规登录相同的JWT生成逻辑
        // 简化实现
        let access_token = format!("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.{}", user_id);
        let refresh_token = format!("refresh_{}", user_id);

        Ok((access_token, refresh_token, 3600))
    }

    /// 列出用户会话
    pub async fn list_user_sessions(&self, user_id: &str) -> Vec<SessionListItem> {
        let session_manager = self.session_manager.read().await;

        session_manager
            .list_user_sessions(user_id)
            .into_iter()
            .map(|s| SessionListItem {
                id: s.id.clone(),
                provider: s.provider_id.clone(),
                created_at: s.created_at.to_rfc3339(),
                expires_at: s.expires_at.to_rfc3339(),
                ip_address: s.ip_address.clone(),
                user_agent: s.user_agent.clone(),
                is_active: s.is_active(),
            })
            .collect()
    }

    /// 终止会话
    pub async fn terminate_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<(), (StatusCode, String)> {
        // 验证会话属于该用户
        let session_manager = self.session_manager.read().await;
        let sessions = session_manager.list_user_sessions(user_id);

        if !sessions.iter().any(|s| s.id == session_id) {
            return Err((StatusCode::FORBIDDEN, "Session not owned by user".to_string()));
        }

        drop(session_manager);

        let mut session_manager = self.session_manager.write().await;
        if session_manager.terminate_session(session_id) {
            // 从Redis删除
            let _ = self.redis.delete(&format!("sso_session:{}", session_id)).await;
            Ok(())
        } else {
            Err((StatusCode::NOT_FOUND, "Session not found".to_string()))
        }
    }

    /// 配置团队SSO
    pub async fn configure_team_sso(
        &self,
        team_id: &str,
        req: TeamSsoConfigRequest,
    ) -> Result<TeamSsoConfigResponse, (StatusCode, String)> {
        let mut manager = self.sso_manager.write().await;

        let group_mappings: Vec<easyssh_core::sso::GroupToRoleMapping> = req
            .group_mappings
            .into_iter()
            .map(|m| easyssh_core::sso::GroupToRoleMapping {
                sso_group: m.sso_group,
                team_role: m.team_role,
            })
            .collect();

        manager
            .configure_team_sso(
                team_id,
                &req.provider_id,
                group_mappings,
                req.auto_provision,
                &req.default_role,
            )
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Configuration failed: {}", e)))?;

        Ok(TeamSsoConfigResponse {
            team_id: team_id.to_string(),
            provider_id: req.provider_id,
            group_mappings: vec![], // 简化
            auto_provision: req.auto_provision,
            default_role: req.default_role,
        })
    }

    /// 生成SAML SP元数据
    pub async fn generate_sp_metadata(
        &self,
        provider_id: &str,
    ) -> Result<String, (StatusCode, String)> {
        let manager = self.sso_manager.read().await;

        let provider = manager
            .get_provider(provider_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Provider not found".to_string()))?;

        let handler = SamlHandler::new(provider.clone()).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create handler: {}", e))
        })?;

        let metadata = handler.generate_sp_metadata(None).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to generate metadata: {}", e))
        })?;

        Ok(metadata)
    }
}
