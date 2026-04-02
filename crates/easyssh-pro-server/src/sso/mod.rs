//! EasySSH Pro Server - SSO模块
//!
//! 提供SAML 2.0、OIDC和OAuth 2.0协议的SSO服务端API实现

pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod service;

pub use handlers::*;
pub use middleware::*;
pub use routes::*;
pub use service::*;

use easyssh_core::sso::{
    OidcConfig, SamlConfig, SsoProvider, SsoProviderType, SsoUserInfo,
};
use serde::{Deserialize, Serialize};

/// SSO提供商创建请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_type: SsoProviderType,
    pub config: serde_json::Value,
    pub enabled: Option<bool>,
}

/// SSO提供商更新请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

/// SSO提供商响应
#[derive(Debug, Clone, Serialize)]
pub struct ProviderResponse {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_url: Option<String>,
    pub acs_url: Option<String>,
}

impl From<SsoProvider> for ProviderResponse {
    fn from(provider: SsoProvider) -> Self {
        let metadata = provider.get_metadata().ok();

        Self {
            id: provider.id,
            name: provider.name,
            provider_type: provider.provider_type.to_string(),
            enabled: provider.enabled,
            created_at: provider.created_at.to_rfc3339(),
            updated_at: provider.updated_at.to_rfc3339(),
            metadata_url: metadata.as_ref().map(|m| m.entity_id.clone()),
            acs_url: metadata.as_ref().map(|m| m.acs_url.clone()),
        }
    }
}

/// SSO登录请求
#[derive(Debug, Clone, Deserialize)]
pub struct SsoLoginRequest {
    pub provider_id: String,
    pub redirect_url: Option<String>,
}

/// SSO登录响应
#[derive(Debug, Clone, Serialize)]
pub struct SsoLoginResponse {
    pub login_url: String,
    pub state: String,
    pub nonce: String,
    pub expires_in: i64,
}

/// SSO回调请求 (OIDC)
#[derive(Debug, Clone, Deserialize)]
pub struct SsoCallbackRequest {
    pub code: String,
    pub state: String,
}

/// SSO回调请求 (SAML)
#[derive(Debug, Clone, Deserialize)]
pub struct SamlCallbackRequest {
    pub saml_response: String,
    pub relay_state: Option<String>,
}

/// SSO登录完成响应
#[derive(Debug, Clone, Serialize)]
pub struct SsoLoginCompleteResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: SsoUserResponse,
    pub is_new_user: bool,
}

/// SSO用户信息响应
#[derive(Debug, Clone, Serialize)]
pub struct SsoUserResponse {
    pub id: String,
    pub email: String,
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub roles: Vec<String>,
    pub provider: String,
}

impl From<SsoUserInfo> for SsoUserResponse {
    fn from(user: SsoUserInfo) -> Self {
        Self {
            id: user.user_id,
            email: user.email,
            username: user.username,
            first_name: user.first_name,
            last_name: user.last_name,
            roles: user.groups,
            provider: user.provider_id,
        }
    }
}

/// 团队SSO配置请求
#[derive(Debug, Clone, Deserialize)]
pub struct TeamSsoConfigRequest {
    pub provider_id: String,
    pub group_mappings: Vec<GroupMappingRequest>,
    pub auto_provision: bool,
    pub default_role: String,
}

/// 组映射请求
#[derive(Debug, Clone, Deserialize)]
pub struct GroupMappingRequest {
    pub sso_group: String,
    pub team_role: String,
}

/// 团队SSO配置响应
#[derive(Debug, Clone, Serialize)]
pub struct TeamSsoConfigResponse {
    pub team_id: String,
    pub provider_id: String,
    pub group_mappings: Vec<GroupMappingResponse>,
    pub auto_provision: bool,
    pub default_role: String,
}

/// 组映射响应
#[derive(Debug, Clone, Serialize)]
pub struct GroupMappingResponse {
    pub sso_group: String,
    pub team_role: String,
}

/// SSO会话列表项
#[derive(Debug, Clone, Serialize)]
pub struct SessionListItem {
    pub id: String,
    pub provider: String,
    pub created_at: String,
    pub expires_at: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub is_active: bool,
}

/// 验证状态参数请求
#[derive(Debug, Clone, Deserialize)]
pub struct ValidateStateRequest {
    pub state: String,
}

/// 验证状态响应
#[derive(Debug, Clone, Serialize)]
pub struct ValidateStateResponse {
    pub valid: bool,
    pub provider_id: Option<String>,
    pub expires_in: Option<i64>,
}
