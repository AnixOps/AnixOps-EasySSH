//! SSO (Single Sign-On) 核心模块 (Pro版本)
//!
//! 支持 SAML 2.0、OIDC 和 OAuth 2.0 协议，具有增强的安全特性：
//! - PKCE (Proof Key for Code Exchange) 保护
//! - 加密令牌存储
//! - Nonce验证防止重放攻击
//! - 会话过期和清理
//!
//! # Security
//!
//! - 所有令牌使用AES-256-GCM加密存储
//! - PKCE verifier使用安全随机数生成
//! - State参数验证防止CSRF攻击
//! - 会话有严格的过期时间

pub mod config;
pub mod handlers;
pub mod identity;
pub mod provisioning;
pub mod session;

pub use config::*;
pub use handlers::*;
pub use identity::*;
pub use provisioning::*;
pub use session::*;

use crate::error::LiteError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// SSO提供者类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SsoProviderType {
    Saml,
    Oidc,
    OAuth2,
    Ldap,
}

impl std::fmt::Display for SsoProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SsoProviderType::Saml => write!(f, "SAML 2.0"),
            SsoProviderType::Oidc => write!(f, "OpenID Connect"),
            SsoProviderType::OAuth2 => write!(f, "OAuth 2.0"),
            SsoProviderType::Ldap => write!(f, "LDAP/AD"),
        }
    }
}

/// SSO提供者配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProvider {
    pub id: String,
    pub name: String,
    pub provider_type: SsoProviderType,
    pub enabled: bool,
    pub config: SsoProviderConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SSO配置详情
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SsoProviderConfig {
    Saml(SamlConfig),
    Oidc(OidcConfig),
    OAuth2(OAuth2Config),
    Ldap(LdapConfig),
}

impl SsoProvider {
    /// 创建新的SAML提供者
    pub fn new_saml(name: &str, config: SamlConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            provider_type: SsoProviderType::Saml,
            enabled: true,
            config: SsoProviderConfig::Saml(config),
            created_at: now,
            updated_at: now,
        }
    }

    /// 创建新的OIDC提供者
    pub fn new_oidc(name: &str, config: OidcConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            provider_type: SsoProviderType::Oidc,
            enabled: true,
            config: SsoProviderConfig::Oidc(config),
            created_at: now,
            updated_at: now,
        }
    }

    /// 创建新的OAuth2提供者
    pub fn new_oauth2(name: &str, config: OAuth2Config) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            provider_type: SsoProviderType::OAuth2,
            enabled: true,
            config: SsoProviderConfig::OAuth2(config),
            created_at: now,
            updated_at: now,
        }
    }

    /// 禁用提供者
    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
    }

    /// 启用提供者
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
    }

    /// 获取元数据 (SAML SP元数据或OIDC发现端点)
    pub fn get_metadata(&self) -> Result<SsoMetadata, LiteError> {
        match &self.config {
            SsoProviderConfig::Saml(config) => {
                let metadata = SsoMetadata {
                    entity_id: config.sp_entity_id.clone(),
                    acs_url: config.acs_url.clone(),
                    slo_url: config.slo_url.clone(),
                    certificate: None,
                };
                Ok(metadata)
            }
            SsoProviderConfig::Oidc(config) => {
                let metadata = SsoMetadata {
                    entity_id: config.client_id.clone(),
                    acs_url: config.redirect_uri.clone(),
                    slo_url: config.end_session_endpoint.clone(),
                    certificate: None,
                };
                Ok(metadata)
            }
            SsoProviderConfig::OAuth2(config) => {
                let metadata = SsoMetadata {
                    entity_id: config.client_id.clone(),
                    acs_url: config.redirect_uri.clone(),
                    slo_url: None,
                    certificate: None,
                };
                Ok(metadata)
            }
            _ => Err(LiteError::Sso("Unsupported provider type".to_string())),
        }
    }
}

/// SSO元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoMetadata {
    pub entity_id: String,
    pub acs_url: String,
    pub slo_url: Option<String>,
    pub certificate: Option<String>,
}

/// SAML认证请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAuthRequest {
    pub id: String,
    pub provider_id: String,
    pub saml_request: String, // Base64编码的SAML请求
    pub relay_state: Option<String>,
    pub destination: String,
}

/// SAML认证响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAuthResponse {
    pub provider_id: String,
    pub saml_response: String, // Base64编码的SAML响应
    pub relay_state: Option<String>,
}

/// OIDC认证请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcAuthRequest {
    pub id: String,
    pub provider_id: String,
    pub authorization_url: String,
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: Option<String>,
}

/// OIDC令牌响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenResponse {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: i64,
}

/// OAuth2令牌响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: i64,
}

/// OIDC用户信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OidcUserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub groups: Option<Vec<String>>,
}

/// OAuth2用户信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OAuth2UserInfo {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub username: Option<String>,
}

/// 解析后的SSO用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoUserInfo {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub groups: Vec<String>,
    pub team_ids: Vec<String>,
    pub provider_type: SsoProviderType,
    pub provider_id: String,
    pub raw_attributes: HashMap<String, serde_json::Value>,
}

/// 团队SSO关联
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSsoMapping {
    pub team_id: String,
    pub provider_id: String,
    pub group_mappings: Vec<GroupToRoleMapping>,
    pub auto_provision: bool,
    pub default_role: String,
}

/// 组到角色映射
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct GroupToRoleMapping {
    pub sso_group: String,
    pub team_role: String,
}

/// SSO管理器 (兼容层)
pub struct SsoManager {
    providers: HashMap<String, SsoProvider>,
    sessions: HashMap<String, SsoSession>,
    team_mappings: HashMap<String, Vec<TeamSsoMapping>>, // team_id -> mappings
    pending_requests: HashMap<String, PendingAuthRequest>,
}

/// 待处理认证请求
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
struct PendingAuthRequest {
    request_id: String,
    provider_id: String,
    #[zeroize(skip)]
    created_at: DateTime<Utc>,
    #[zeroize(skip)]
    nonce: String,
    // PKCE verifier
    pkce_verifier: Option<String>,
    // 关联的state参数
    state: String,
}

use zeroize::{Zeroize, ZeroizeOnDrop};

impl SsoManager {
    /// 创建新的SSO管理器
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            sessions: HashMap::new(),
            team_mappings: HashMap::new(),
            pending_requests: HashMap::new(),
        }
    }

    /// 添加SSO提供者
    pub fn add_provider(&mut self, provider: SsoProvider) -> Result<(), LiteError> {
        let id = provider.id.clone();
        self.providers.insert(id, provider);
        Ok(())
    }

    /// 获取提供者
    pub fn get_provider(&self, provider_id: &str) -> Option<&SsoProvider> {
        self.providers.get(provider_id)
    }

    /// 获取提供者(可变)
    pub fn get_provider_mut(&mut self, provider_id: &str) -> Option<&mut SsoProvider> {
        self.providers.get_mut(provider_id)
    }

    /// 移除提供者
    pub fn remove_provider(&mut self, provider_id: &str) -> Result<(), LiteError> {
        // 清理相关会话
        self.sessions
            .retain(|_, session| session.provider_id != provider_id);
        self.providers.remove(provider_id);
        Ok(())
    }

    /// 列出所有提供者
    pub fn list_providers(&self) -> Vec<&SsoProvider> {
        self.providers.values().collect()
    }

    /// 列出启用的提供者
    pub fn list_enabled_providers(&self) -> Vec<&SsoProvider> {
        self.providers.values().filter(|p| p.enabled).collect()
    }

    /// 为团队配置SSO
    pub fn configure_team_sso(
        &mut self,
        team_id: &str,
        provider_id: &str,
        group_mappings: Vec<GroupToRoleMapping>,
        auto_provision: bool,
        default_role: &str,
    ) -> Result<(), LiteError> {
        // 验证提供者存在
        if !self.providers.contains_key(provider_id) {
            return Err(LiteError::Sso(format!(
                "Provider {} not found",
                provider_id
            )));
        }

        let mapping = TeamSsoMapping {
            team_id: team_id.to_string(),
            provider_id: provider_id.to_string(),
            group_mappings,
            auto_provision,
            default_role: default_role.to_string(),
        };

        self.team_mappings
            .entry(team_id.to_string())
            .or_default()
            .push(mapping);

        Ok(())
    }

    /// 根据SSO组映射获取团队角色
    pub fn map_sso_groups_to_team_role(
        &self,
        team_id: &str,
        sso_groups: &[String],
    ) -> Option<String> {
        let mappings = self.team_mappings.get(team_id)?;

        for mapping in mappings {
            for group_mapping in &mapping.group_mappings {
                if sso_groups.contains(&group_mapping.sso_group) {
                    return Some(group_mapping.team_role.clone());
                }
            }
        }

        None
    }

    /// 同步SSO用户到团队
    pub fn sync_user_to_team(
        &self,
        user_info: &SsoUserInfo,
        team_id: &str,
    ) -> Result<Option<String>, LiteError> {
        let mappings: Vec<&TeamSsoMapping> = self
            .team_mappings
            .get(team_id)
            .map(|m| m.iter().collect())
            .unwrap_or_default();

        if mappings.is_empty() {
            return Ok(None);
        }

        // 查找匹配的组映射
        let role = self.map_sso_groups_to_team_role(team_id, &user_info.groups);

        if let Some(role) = role {
            Ok(Some(role))
        } else {
            // 返回默认角色
            Ok(mappings.first().map(|m| m.default_role.clone()))
        }
    }

    /// 列出用户的活跃会话
    pub fn list_user_sessions(&self, user_id: &str) -> Vec<&SsoSession> {
        self.sessions
            .values()
            .filter(|s| s.user_id == user_id && !s.is_expired())
            .collect()
    }

    /// 终止用户的所有会话
    pub fn terminate_user_sessions(&mut self, user_id: &str) -> usize {
        let sessions_to_remove: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.user_id == user_id)
            .map(|(id, _)| id.clone())
            .collect();

        let count = sessions_to_remove.len();
        for id in sessions_to_remove {
            self.sessions.remove(&id);
        }

        count
    }
}

impl Default for SsoManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成高熵安全随机字符串
pub fn generate_secure_random(length: usize) -> String {
    use rand::RngCore;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut rng = rand::thread_rng();
    let mut bytes = vec![0u8; length];
    rng.fill_bytes(&mut bytes);

    bytes
        .iter()
        .map(|b| CHARSET[(b % CHARSET.len() as u8) as usize] as char)
        .collect()
}

/// 生成SSO令牌 (64字符)
pub fn generate_sso_token() -> String {
    generate_secure_random(64)
}

/// Base64 URL安全编码
pub fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

/// SHA256哈希
pub fn sha256_hash(input: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sso_provider_creation() {
        let saml_config = SamlConfig {
            idp_metadata_url: "https://idp.example.com/metadata".to_string(),
            sp_entity_id: "https://easyssh.pro".to_string(),
            acs_url: "https://easyssh.pro/sso/acs".to_string(),
            slo_url: None,
            signature_algorithm: "rsa-sha256".to_string(),
        };

        let provider = SsoProvider::new_saml("Okta SAML", saml_config);
        assert_eq!(provider.name, "Okta SAML");
        assert_eq!(provider.provider_type, SsoProviderType::Saml);
        assert!(provider.enabled);
    }

    #[test]
    fn test_oidc_config() {
        let config = OidcConfig::standard(
            "https://auth.example.com",
            "client123",
            "secret456",
            "https://easyssh.pro/callback",
        );

        assert_eq!(config.issuer_url, "https://auth.example.com");
        assert_eq!(config.client_id, "client123");
        assert!(config.use_pkce);
        assert!(config.scopes.contains(&"openid".to_string()));
    }

    #[test]
    fn test_sso_user_info() {
        let user_info = SsoUserInfo {
            user_id: "user123".to_string(),
            email: "user@example.com".to_string(),
            username: "user".to_string(),
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
            groups: vec!["users".to_string(), "developers".to_string()],
            team_ids: vec!["team1".to_string()],
            provider_type: SsoProviderType::Oidc,
            provider_id: "provider1".to_string(),
            raw_attributes: HashMap::new(),
        };

        assert_eq!(user_info.user_id, "user123");
        assert_eq!(user_info.groups.len(), 2);
    }
}
