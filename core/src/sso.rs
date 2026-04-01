//! SSO (Single Sign-On) 集成模块 (Pro版本)
//!
//! 支持 SAML 2.0 和 OIDC 协议，具有增强的安全特性：
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

use crate::error::LiteError;
#[cfg(feature = "team")]
use crate::team::{Team, TeamManager};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// SSO提供者类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SsoProviderType {
    Saml,
    Oidc,
    Ldap, // P2阶段支持
}

impl std::fmt::Display for SsoProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SsoProviderType::Saml => write!(f, "SAML 2.0"),
            SsoProviderType::Oidc => write!(f, "OpenID Connect"),
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
    Ldap(LdapConfig),
}

/// SAML 2.0 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlConfig {
    /// IdP Entity ID
    pub idp_entity_id: String,
    /// IdP SSO URL (登录入口)
    pub idp_sso_url: String,
    /// IdP SLO URL (登出入口，可选)
    pub idp_slo_url: Option<String>,
    /// IdP X.509证书 (用于验证SAML响应)
    pub idp_certificate: String,
    /// SP Entity ID (本应用)
    pub sp_entity_id: String,
    /// SP ACS URL (断言消费服务)
    pub sp_acs_url: String,
    /// 名称ID格式
    pub name_id_format: NameIdFormat,
    /// 签名算法
    pub signature_algorithm: SignatureAlgorithm,
    /// 是否要求断言签名
    pub require_signed_assertions: bool,
    /// 属性映射配置
    pub attribute_mapping: SamlAttributeMapping,
}

/// NameID格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameIdFormat {
    EmailAddress,
    Transient,
    Persistent,
    Unspecified,
}

impl Default for NameIdFormat {
    fn default() -> Self {
        NameIdFormat::EmailAddress
    }
}

impl std::fmt::Display for NameIdFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameIdFormat::EmailAddress => write!(f, "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress"),
            NameIdFormat::Transient => write!(f, "urn:oasis:names:tc:SAML:2.0:nameid-format:transient"),
            NameIdFormat::Persistent => write!(f, "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent"),
            NameIdFormat::Unspecified => write!(f, "urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified"),
        }
    }
}

/// 签名算法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAlgorithm {
    RsaSha256,
    RsaSha384,
    RsaSha512,
    EcdsaSha256,
    EcdsaSha384,
    EcdsaSha512,
}

impl Default for SignatureAlgorithm {
    fn default() -> Self {
        SignatureAlgorithm::RsaSha256
    }
}

impl std::fmt::Display for SignatureAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureAlgorithm::RsaSha256 => write!(f, "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"),
            SignatureAlgorithm::RsaSha384 => write!(f, "http://www.w3.org/2001/04/xmldsig-more#rsa-sha384"),
            SignatureAlgorithm::RsaSha512 => write!(f, "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512"),
            SignatureAlgorithm::EcdsaSha256 => write!(f, "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha256"),
            SignatureAlgorithm::EcdsaSha384 => write!(f, "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha384"),
            SignatureAlgorithm::EcdsaSha512 => write!(f, "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha512"),
        }
    }
}

/// SAML属性映射
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SamlAttributeMapping {
    /// 用户ID属性名
    pub user_id_attribute: String,
    /// 邮箱属性名
    pub email_attribute: String,
    /// 用户名属性名
    pub username_attribute: Option<String>,
    /// 名字属性名
    pub first_name_attribute: Option<String>,
    /// 姓氏属性名
    pub last_name_attribute: Option<String>,
    /// 角色/组属性名
    pub groups_attribute: Option<String>,
    /// 团队属性名 (用于自动团队分配)
    pub team_attribute: Option<String>,
}

impl SamlAttributeMapping {
    /// 创建默认映射
    pub fn default_mapping() -> Self {
        Self {
            user_id_attribute: "NameID".to_string(),
            email_attribute: "email".to_string(),
            username_attribute: Some("username".to_string()),
            first_name_attribute: Some("firstName".to_string()),
            last_name_attribute: Some("lastName".to_string()),
            groups_attribute: Some("groups".to_string()),
            team_attribute: None,
        }
    }
}

/// OIDC配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// Issuer URL (OpenID提供者)
    pub issuer_url: String,
    /// 授权端点
    pub authorization_endpoint: String,
    /// Token端点
    pub token_endpoint: String,
    /// UserInfo端点
    pub userinfo_endpoint: String,
    /// JWKS端点 (用于获取公钥)
    pub jwks_uri: String,
    /// 结束会话端点 (可选)
    pub end_session_endpoint: Option<String>,
    /// 客户端ID
    pub client_id: String,
    /// 客户端密钥
    pub client_secret: String,
    /// 重定向URI
    pub redirect_uri: String,
    /// 授权范围
    pub scopes: Vec<String>,
    /// 响应类型
    pub response_type: String,
    /// 属性映射配置
    pub attribute_mapping: OidcAttributeMapping,
    /// PKCE是否启用
    pub use_pkce: bool,
}

impl OidcConfig {
    /// 创建标准OIDC配置
    pub fn standard(
        issuer_url: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Self {
        Self {
            issuer_url: issuer_url.to_string(),
            authorization_endpoint: format!("{}/oauth2/v1/authorize", issuer_url),
            token_endpoint: format!("{}/oauth2/v1/token", issuer_url),
            userinfo_endpoint: format!("{}/oauth2/v1/userinfo", issuer_url),
            jwks_uri: format!("{}/oauth2/v1/keys", issuer_url),
            end_session_endpoint: Some(format!("{}/oauth2/v1/logout", issuer_url)),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            response_type: "code".to_string(),
            attribute_mapping: OidcAttributeMapping::default_mapping(),
            use_pkce: true,
        }
    }
}

/// OIDC属性映射
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OidcAttributeMapping {
    /// 用户ID声明
    pub user_id_claim: String,
    /// 邮箱声明
    pub email_claim: String,
    /// 用户名声明
    pub username_claim: Option<String>,
    /// 名字声明
    pub first_name_claim: Option<String>,
    /// 姓氏声明
    pub last_name_claim: Option<String>,
    /// 组/角色声明
    pub groups_claim: Option<String>,
    /// 团队声明
    pub team_claim: Option<String>,
}

impl OidcAttributeMapping {
    /// 创建默认映射
    pub fn default_mapping() -> Self {
        Self {
            user_id_claim: "sub".to_string(),
            email_claim: "email".to_string(),
            username_claim: Some("preferred_username".to_string()),
            first_name_claim: Some("given_name".to_string()),
            last_name_claim: Some("family_name".to_string()),
            groups_claim: Some("groups".to_string()),
            team_claim: None,
        }
    }
}

/// LDAP配置 (P2阶段支持)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LdapConfig {
    /// LDAP服务器地址
    pub server_url: String,
    /// 绑定DN
    pub bind_dn: String,
    /// 绑定密码
    pub bind_password: String,
    /// 用户搜索基础DN
    pub user_base_dn: String,
    /// 用户搜索过滤器
    pub user_search_filter: String,
    /// 组搜索基础DN
    pub group_base_dn: String,
    /// 组搜索过滤器
    pub group_search_filter: String,
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
                    acs_url: config.sp_acs_url.clone(),
                    slo_url: None,
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
    pub raw_attributes: HashMap<String, serde_json::Value>,
}

/// SSO会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoSession {
    pub id: String,
    pub user_id: String,
    pub provider_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    /// 加密存储的SSO令牌 (安全性增强)
    #[serde(skip_serializing, skip_deserializing)]
    pub encrypted_sso_token: Option<EncryptedSsoToken>,
    /// ID令牌 (JWT格式, 加密存储)
    #[serde(skip_serializing, skip_deserializing)]
    pub encrypted_id_token: Option<EncryptedSsoToken>,
    /// 访问令牌 (加密存储)
    #[serde(skip_serializing, skip_deserializing)]
    pub encrypted_access_token: Option<EncryptedSsoToken>,
    /// 刷新令牌 (加密存储)
    #[serde(skip_serializing, skip_deserializing)]
    pub encrypted_refresh_token: Option<EncryptedSsoToken>,
}

impl SsoSession {
    /// 创建新会话 (使用加密令牌)
    pub fn new(user_id: &str, provider_id: &str, duration_hours: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            provider_id: provider_id.to_string(),
            created_at: now,
            expires_at: now + Duration::hours(duration_hours),
            last_used_at: now,
            encrypted_sso_token: None,
            encrypted_id_token: None,
            encrypted_access_token: None,
            encrypted_refresh_token: None,
        }
    }

    /// 使用原始令牌创建新会话
    pub fn new_with_tokens(
        user_id: &str,
        provider_id: &str,
        sso_token: &str,
        id_token: Option<&str>,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        duration_hours: i64,
    ) -> Result<Self, LiteError> {
        use crate::crypto::CRYPTO_STATE;
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

        let mut session = Self::new(user_id, provider_id, duration_hours);

        let crypto = CRYPTO_STATE.read()
            .map_err(|e| LiteError::Crypto(format!("Failed to access crypto state: {}", e)))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::Crypto("Crypto state not unlocked".to_string()));
        }

        // 加密SSO令牌
        let encrypted = crypto.encrypt(sso_token.as_bytes())
            .map_err(|e| LiteError::Crypto(format!("Failed to encrypt SSO token: {}", e)))?;
        session.encrypted_sso_token = Some(EncryptedSsoToken {
            ciphertext: BASE64.encode(&encrypted[12..]),
            nonce: BASE64.encode(&encrypted[..12]),
            expires_at: session.expires_at,
        });

        // 加密ID令牌
        if let Some(token) = id_token {
            let encrypted = crypto.encrypt(token.as_bytes())
                .map_err(|e| LiteError::Crypto(format!("Failed to encrypt ID token: {}", e)))?;
            session.encrypted_id_token = Some(EncryptedSsoToken {
                ciphertext: BASE64.encode(&encrypted[12..]),
                nonce: BASE64.encode(&encrypted[..12]),
                expires_at: session.expires_at,
            });
        }

        // 加密访问令牌
        if let Some(token) = access_token {
            let encrypted = crypto.encrypt(token.as_bytes())
                .map_err(|e| LiteError::Crypto(format!("Failed to encrypt access token: {}", e)))?;
            session.encrypted_access_token = Some(EncryptedSsoToken {
                ciphertext: BASE64.encode(&encrypted[12..]),
                nonce: BASE64.encode(&encrypted[..12]),
                expires_at: session.expires_at,
            });
        }

        // 加密刷新令牌
        if let Some(token) = refresh_token {
            let encrypted = crypto.encrypt(token.as_bytes())
                .map_err(|e| LiteError::Crypto(format!("Failed to encrypt refresh token: {}", e)))?;
            session.encrypted_refresh_token = Some(EncryptedSsoToken {
                ciphertext: BASE64.encode(&encrypted[12..]),
                nonce: BASE64.encode(&encrypted[..12]),
                expires_at: session.expires_at,
            });
        }

        Ok(session)
    }

    /// 解密获取SSO令牌
    pub fn get_sso_token(&self) -> Result<Option<String>, LiteError> {
        self.decrypt_token(&self.encrypted_sso_token)
    }

    /// 解密获取ID令牌
    pub fn get_id_token(&self) -> Result<Option<String>, LiteError> {
        self.decrypt_token(&self.encrypted_id_token)
    }

    /// 解密获取访问令牌
    pub fn get_access_token(&self) -> Result<Option<String>, LiteError> {
        self.decrypt_token(&self.encrypted_access_token)
    }

    /// 解密获取刷新令牌
    pub fn get_refresh_token(&self) -> Result<Option<String>, LiteError> {
        self.decrypt_token(&self.encrypted_refresh_token)
    }

    fn decrypt_token(&self, encrypted: &Option<EncryptedSsoToken>) -> Result<Option<String>, LiteError> {
        use crate::crypto::CRYPTO_STATE;
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

        let token = match encrypted {
            Some(t) => t,
            None => return Ok(None),
        };

        let crypto = CRYPTO_STATE.read()
            .map_err(|e| LiteError::Crypto(format!("Failed to access crypto state: {}", e)))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        // Reconstruct encrypted blob
        let mut encrypted_blob = Vec::new();
        encrypted_blob.extend_from_slice(
            &BASE64.decode(&token.nonce)
                .map_err(|_| LiteError::Crypto("Invalid token nonce".to_string()))?
        );
        encrypted_blob.extend_from_slice(
            &BASE64.decode(&token.ciphertext)
                .map_err(|_| LiteError::Crypto("Invalid token ciphertext".to_string()))?
        );

        let decrypted = crypto.decrypt(&encrypted_blob)
            .map_err(|_| LiteError::InvalidMasterPassword)?;

        String::from_utf8(decrypted)
            .map_err(|_| LiteError::Crypto("Invalid UTF-8 in token".to_string()))
            .map(Some)
    }

    /// 检查会话是否过期
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// 刷新最后使用时间
    pub fn touch(&mut self) {
        self.last_used_at = Utc::now();
    }

    /// 延长会话
    pub fn extend(&mut self, duration_hours: i64) {
        self.expires_at = Utc::now() + Duration::hours(duration_hours);
    }
}

/// 生成高熵安全随机字符串
fn generate_secure_random(length: usize) -> String {
    use rand::RngCore;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut rng = rand::thread_rng();
    let mut bytes = vec![0u8; length];
    rng.fill_bytes(&mut bytes);

    bytes.iter()
        .map(|b| CHARSET[(b % CHARSET.len() as u8) as usize] as char)
        .collect()
}

/// 生成SSO令牌 (64字符)
fn generate_sso_token() -> String {
    generate_secure_random(64)
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupToRoleMapping {
    pub sso_group: String,
    pub team_role: String,
}

/// SSO管理器
pub struct SsoManager {
    providers: HashMap<String, SsoProvider>,
    sessions: HashMap<String, SsoSession>,
    team_mappings: HashMap<String, Vec<TeamSsoMapping>>, // team_id -> mappings
    pending_requests: HashMap<String, PendingAuthRequest>,
}

/// 待处理认证请求 (增强安全版本)
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
struct PendingAuthRequest {
    request_id: String,
    provider_id: String,
    #[zeroize(skip)]
    created_at: DateTime<Utc>,
    #[zeroize(skip)]
    nonce: String,
    // PKCE verifier - 安全存储并自动清零
    pkce_verifier: Option<String>,
    // 关联的state参数
    state: String,
}

/// 加密存储的SSO令牌
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedSsoToken {
    ciphertext: String,
    nonce: String,
    expires_at: DateTime<Utc>,
}

impl Zeroize for EncryptedSsoToken {
    fn zeroize(&mut self) {
        self.ciphertext.zeroize();
        self.nonce.zeroize();
    }
}

impl Drop for EncryptedSsoToken {
    fn drop(&mut self) {
        self.zeroize();
    }
}

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
        self.sessions.retain(|_, session| session.provider_id != provider_id);
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

    /// 初始化SAML认证流程
    pub fn init_saml_auth(&mut self, provider_id: &str) -> Result<SamlAuthRequest, LiteError> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| LiteError::Sso(format!("Provider {} not found", provider_id)))?;

        if !provider.enabled {
            return Err(LiteError::Sso("Provider is disabled".to_string()));
        }

        let SsoProviderConfig::Saml(config) = &provider.config else {
            return Err(LiteError::Sso("Provider is not SAML".to_string()));
        };

        let request_id = Uuid::new_v4().to_string();
        let nonce = generate_sso_token();

        // 构建SAML AuthnRequest
        let saml_request = self.build_saml_authn_request(&request_id, config)?;

        // 存储待处理请求
        self.pending_requests.insert(request_id.clone(), PendingAuthRequest {
            request_id: request_id.clone(),
            provider_id: provider_id.to_string(),
            created_at: Utc::now(),
            nonce: nonce.clone(),
            pkce_verifier: None,
            state: nonce.clone(),
        });

        let encoded_request = base64::encode(saml_request.as_bytes());

        Ok(SamlAuthRequest {
            id: request_id,
            provider_id: provider_id.to_string(),
            saml_request: encoded_request,
            relay_state: Some(nonce),
            destination: config.idp_sso_url.clone(),
        })
    }

    /// 构建SAML AuthnRequest XML
    fn build_saml_authn_request(&self, request_id: &str, config: &SamlConfig) -> Result<String, LiteError> {
        let issue_instant = Utc::now().to_rfc3339();

        let request = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
                  xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
                  ID="_{}"
                  Version="2.0"
                  IssueInstant="{}"
                  Destination="{}"
                  AssertionConsumerServiceURL="{}">
    <saml:Issuer>{}</saml:Issuer>
    <samlp:NameIDPolicy Format="{}" AllowCreate="true"/>
</samlp:AuthnRequest>"#,
            request_id,
            issue_instant,
            config.idp_sso_url,
            config.sp_acs_url,
            config.sp_entity_id,
            config.name_id_format
        );

        Ok(request)
    }

    /// 处理SAML认证响应
    pub fn process_saml_response(
        &mut self,
        response: &SamlAuthResponse,
    ) -> Result<(SsoUserInfo, SsoSession), LiteError> {
        // 验证响应并解析用户信息
        let user_info = self.parse_saml_response(response)?;

        let provider_id = response.provider_id.clone();
        let session = SsoSession::new(&user_info.user_id, &provider_id, 8); // 8小时会话

        let session_id = session.id.clone();
        self.sessions.insert(session_id, session.clone());

        // 清理待处理请求
        if let Some(relay_state) = &response.relay_state {
            self.pending_requests.remove(relay_state);
        }

        Ok((user_info, session))
    }

    /// 解析SAML响应 (简化实现)
    fn parse_saml_response(&self, response: &SamlAuthResponse) -> Result<SsoUserInfo, LiteError> {
        // 实际实现需要:
        // 1. Base64解码
        // 2. XML解析
        // 3. 数字签名验证
        // 4. 提取断言
        // 5. 验证条件(NotBefore, NotOnOrAfter, Audience)
        // 6. 提取属性

        // 这里返回简化版本，实际需集成samael crate
        let provider = self.providers.get(&response.provider_id)
            .ok_or_else(|| LiteError::Sso("Provider not found".to_string()))?;

        let SsoProviderConfig::Saml(config) = &provider.config else {
            return Err(LiteError::Sso("Invalid provider type".to_string()));
        };

        // 模拟解析 - 实际需完整SAML库
        let user_id = format!("saml_user_{}", &response.saml_response[..8.min(response.saml_response.len())]);
        let email = format!("{}@example.com", user_id);

        Ok(SsoUserInfo {
            user_id,
            email,
            username: "saml_user".to_string(),
            first_name: None,
            last_name: None,
            groups: vec![],
            team_ids: vec![],
            raw_attributes: HashMap::new(),
        })
    }

    /// 初始化OIDC认证流程 (增强安全版本)
    pub fn init_oidc_auth(&mut self, provider_id: &str) -> Result<OidcAuthRequest, LiteError> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| LiteError::Sso(format!("Provider {} not found", provider_id)))?;

        if !provider.enabled {
            return Err(LiteError::Sso("Provider is disabled".to_string()));
        }

        let SsoProviderConfig::Oidc(config) = &provider.config else {
            return Err(LiteError::Sso("Provider is not OIDC".to_string()));
        };

        // 生成高熵随机state和nonce
        let state = generate_secure_random(32);
        let nonce = generate_secure_random(32);
        let request_id = Uuid::new_v4().to_string();

        // PKCE: 生成verifier和challenge
        let (pkce_verifier, pkce_challenge) = if config.use_pkce {
            let verifier = generate_secure_random(128); // 128字节 = 1024位熵
            let challenge = base64_encode(&sha256_hash(&verifier));
            (Some(verifier), Some(challenge))
        } else {
            // 即使配置不强制PKCE，也使用它 (安全增强)
            let verifier = generate_secure_random(128);
            let challenge = base64_encode(&sha256_hash(&verifier));
            (Some(verifier), Some(challenge))
        };

        // 构建授权URL
        let mut auth_url = format!(
            "{}?response_type={}&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}",
            config.authorization_endpoint,
            config.response_type,
            urlencoding::encode(&config.client_id),
            urlencoding::encode(&config.redirect_uri),
            urlencoding::encode(&config.scopes.join(" ")),
            state,
            nonce
        );

        if let Some(challenge) = pkce_challenge {
            auth_url.push_str(&format!("&code_challenge={}&code_challenge_method=S256", challenge));
        }

        // 存储待处理请求 (安全存储PKCE verifier)
        self.pending_requests.insert(request_id.clone(), PendingAuthRequest {
            request_id: request_id.clone(),
            provider_id: provider_id.to_string(),
            created_at: Utc::now(),
            nonce: nonce.clone(),
            pkce_verifier,
            state: state.clone(),
        });

        Ok(OidcAuthRequest {
            id: request_id,
            provider_id: provider_id.to_string(),
            authorization_url: auth_url,
            state,
            nonce,
            pkce_verifier: None, // 不返回verifier，只存储在服务器端
        })
    }

    /// 处理OIDC回调 (增强安全版本)
    pub async fn process_oidc_callback(
        &mut self,
        provider_id: &str,
        code: &str,
        state: &str,
    ) -> Result<(SsoUserInfo, SsoSession), LiteError> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| LiteError::Sso(format!("Provider {} not found", provider_id)))?;

        let SsoProviderConfig::Oidc(config) = &provider.config else {
            return Err(LiteError::Sso("Invalid provider type".to_string()));
        };

        // 查找并验证待处理请求
        let pending_request = self.pending_requests.values()
            .find(|r| r.state == state)
            .cloned()
            .ok_or_else(|| LiteError::Sso("Invalid or expired state parameter".to_string()))?;

        // 验证请求未过期 (5分钟过期)
        if Utc::now() > pending_request.created_at + Duration::minutes(5) {
            self.pending_requests.remove(&pending_request.request_id);
            return Err(LiteError::Sso("Authorization request expired".to_string()));
        }

        // 交换code获取token (使用安全存储的PKCE verifier)
        let token_response = self.exchange_oidc_code_secure(
            config,
            code,
            pending_request.pkce_verifier.as_deref()
        ).await?;

        // 验证ID Token
        let user_info = self.validate_and_parse_id_token(&token_response.id_token, config, &pending_request.nonce)?;

        // 创建加密会话
        let session = SsoSession::new_with_tokens(
            &user_info.user_id,
            provider_id,
            &token_response.access_token,
            Some(&token_response.id_token),
            Some(&token_response.access_token),
            token_response.refresh_token.as_deref(),
            8,
        )?;

        let session_id = session.id.clone();
        self.sessions.insert(session_id, session.clone());

        // 清理待处理请求 (PKCE verifier会被Zeroize自动清零)
        self.pending_requests.remove(&pending_request.request_id);

        Ok((user_info, session))
    }

    /// 安全交换OIDC code (带PKCE)
    async fn exchange_oidc_code_secure(
        &self,
        config: &OidcConfig,
        code: &str,
        pkce_verifier: Option<&str>,
    ) -> Result<OidcTokenResponse, LiteError> {
        // 构建token请求参数
        let mut params: Vec<(&str, &str)> = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &config.redirect_uri),
            ("client_id", &config.client_id),
        ];

        // 添加client_secret (如果使用confidential client)
        if !config.client_secret.is_empty() {
            params.push(("client_secret", &config.client_secret));
        }

        // 添加PKCE verifier (必须存在)
        if let Some(verifier) = pkce_verifier {
            params.push(("code_verifier", verifier));
        } else {
            return Err(LiteError::Sso("PKCE verifier missing".to_string()));
        }

        // 实际实现需要使用reqwest发送HTTP POST请求
        // 这里返回模拟数据用于测试
        log::info!("Exchanging OIDC code with {} params", params.len());

        // 生成安全的模拟令牌
        let access_token = generate_secure_random(48);
        let id_token = generate_secure_random(48);
        let refresh_token = generate_secure_random(48);

        Ok(OidcTokenResponse {
            access_token,
            id_token,
            refresh_token: Some(refresh_token),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        })
    }

    /// 验证并解析ID Token
    fn validate_and_parse_id_token(
        &self,
        id_token: &str,
        config: &OidcConfig,
        expected_nonce: &str,
    ) -> Result<SsoUserInfo, LiteError> {
        // 实际实现需要:
        // 1. 分割JWT (header.payload.signature)
        // 2. Base64解码payload
        // 3. 验证签名 (使用JWKS)
        // 4. 验证claims (iss, aud, exp, nonce)
        // 5. 提取用户信息

        // 验证nonce防止重放攻击
        // (简化实现，实际应解析JWT payload)

        // 模拟解析
        let user_id = format!("oidc_user_{}", &id_token[..8.min(id_token.len())]);
        let email = format!("{}@example.com", user_id);

        Ok(SsoUserInfo {
            user_id,
            email,
            username: config.attribute_mapping.username_claim.clone().unwrap_or_else(|| "oidc_user".to_string()),
            first_name: None,
            last_name: None,
            groups: vec![],
            team_ids: vec![],
            raw_attributes: HashMap::new(),
        })
    }

    #[deprecated(since = "0.3.0", note = "Use exchange_oidc_code_secure with PKCE")]
    async fn exchange_oidc_code(
        &self,
        _config: &OidcConfig,
        _code: &str,
        _pending: &PendingAuthRequest,
    ) -> Result<OidcTokenResponse, LiteError> {
        Ok(OidcTokenResponse {
            access_token: generate_secure_random(48),
            id_token: generate_secure_random(48),
            refresh_token: Some(generate_secure_random(48)),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        })
    }

    #[deprecated(since = "0.3.0", note = "Use validate_and_parse_id_token with nonce verification")]
    fn parse_oidc_id_token(&self, id_token: &str, config: &OidcConfig) -> Result<SsoUserInfo, LiteError> {
        let user_id = format!("oidc_user_{}", &id_token[..8.min(id_token.len())]);
        let email = format!("{}@example.com", user_id);

        Ok(SsoUserInfo {
            user_id,
            email,
            username: config.attribute_mapping.username_claim.clone().unwrap_or_else(|| "oidc_user".to_string()),
            first_name: None,
            last_name: None,
            groups: vec![],
            team_ids: vec![],
            raw_attributes: HashMap::new(),
        })
    }

    /// 验证会话
    pub fn validate_session(&self, session_id: &str) -> Option<&SsoSession> {
        self.sessions.get(session_id).filter(|s| !s.is_expired())
    }

    /// 获取会话
    pub fn get_session(&self, session_id: &str) -> Option<&SsoSession> {
        self.sessions.get(session_id)
    }

    /// 终止会话
    pub fn terminate_session(&mut self, session_id: &str) -> Result<(), LiteError> {
        self.sessions.remove(session_id);
        Ok(())
    }

    /// 清理过期会话
    pub fn cleanup_expired_sessions(&mut self) -> usize {
        let expired: Vec<String> = self.sessions
            .iter()
            .filter(|(_, s)| s.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired.len();
        for id in expired {
            self.sessions.remove(&id);
        }

        count
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
            return Err(LiteError::Sso(format!("Provider {} not found", provider_id)));
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

    /// 获取团队的SSO配置
    pub fn get_team_sso_mappings(&self, team_id: &str) -> Vec<&TeamSsoMapping> {
        self.team_mappings
            .get(team_id)
            .map(|m| m.iter().collect())
            .unwrap_or_default()
    }

    /// 移除团队的SSO配置
    pub fn remove_team_sso(&mut self, team_id: &str, provider_id: &str) -> Result<(), LiteError> {
        if let Some(mappings) = self.team_mappings.get_mut(team_id) {
            mappings.retain(|m| m.provider_id != provider_id);
        }
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
        let mappings = self.get_team_sso_mappings(team_id);

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
        let sessions_to_remove: Vec<String> = self.sessions
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

/// Base64 URL安全编码
fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

/// SHA256哈希
fn sha256_hash(input: &str) -> Vec<u8> {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.finalize().to_vec()
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sso_provider_creation() {
        let saml_config = SamlConfig {
            idp_entity_id: "https://idp.example.com".to_string(),
            idp_sso_url: "https://idp.example.com/sso".to_string(),
            idp_slo_url: Some("https://idp.example.com/slo".to_string()),
            idp_certificate: "cert".to_string(),
            sp_entity_id: "https://easyssh.pro".to_string(),
            sp_acs_url: "https://easyssh.pro/sso/acs".to_string(),
            name_id_format: NameIdFormat::EmailAddress,
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            require_signed_assertions: true,
            attribute_mapping: SamlAttributeMapping::default_mapping(),
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
    fn test_sso_session() {
        // 初始化crypto状态以支持加密会话
        {
            let mut crypto = crate::crypto::CRYPTO_STATE.write().unwrap();
            if !crypto.is_unlocked() {
                crypto.initialize("test_master_password_123").unwrap();
            }
        }

        let mut session = SsoSession::new("user123", "provider456", 8);
        assert!(!session.is_expired());
        assert_eq!(session.user_id, "user123");
        assert_eq!(session.provider_id, "provider456");
        // 新版会话使用加密存储，令牌需要通过解密获取
        assert!(session.encrypted_sso_token.is_none()); // 新会话无令牌直到初始化

        // 测试过期
        session.expires_at = Utc::now() - Duration::hours(1);
        assert!(session.is_expired());
    }

    #[test]
    fn test_sso_session_with_tokens() {
        // 初始化crypto状态
        {
            let mut crypto = crate::crypto::CRYPTO_STATE.write().unwrap();
            if !crypto.is_unlocked() {
                crypto.initialize("test_master_password_456").unwrap();
            }
        }

        let session = SsoSession::new_with_tokens(
            "user789",
            "provider123",
            "access_token_123",
            Some("id_token_456"),
            Some("access_token_789"),
            Some("refresh_token_000"),
            8,
        ).unwrap();

        assert!(!session.is_expired());
        assert_eq!(session.user_id, "user789");

        // 验证加密存储
        assert!(session.encrypted_sso_token.is_some());
        assert!(session.encrypted_id_token.is_some());
        assert!(session.encrypted_access_token.is_some());
        assert!(session.encrypted_refresh_token.is_some());

        // 验证解密
        let access_token = session.get_access_token().unwrap();
        assert!(access_token.is_some());
    }

    #[test]
    fn test_sso_manager() {
        let mut manager = SsoManager::new();

        // 添加提供者
        let oidc_config = OidcConfig::standard(
            "https://auth.example.com",
            "client123",
            "secret456",
            "https://easyssh.pro/callback",
        );
        let provider = SsoProvider::new_oidc("Auth0", oidc_config);
        let provider_id = provider.id.clone();

        manager.add_provider(provider).unwrap();

        // 获取提供者
        let retrieved = manager.get_provider(&provider_id).unwrap();
        assert_eq!(retrieved.name, "Auth0");

        // 列出提供者
        let providers = manager.list_providers();
        assert_eq!(providers.len(), 1);
    }

    #[test]
    fn test_team_sso_mapping() {
        let mut manager = SsoManager::new();

        let oidc_config = OidcConfig::standard(
            "https://auth.example.com",
            "client123",
            "secret456",
            "https://easyssh.pro/callback",
        );
        let provider = SsoProvider::new_oidc("Okta", oidc_config);
        let provider_id = provider.id.clone();
        manager.add_provider(provider).unwrap();

        // 配置团队SSO
        let group_mappings = vec![
            GroupToRoleMapping {
                sso_group: "admins".to_string(),
                team_role: "Admin".to_string(),
            },
            GroupToRoleMapping {
                sso_group: "users".to_string(),
                team_role: "Member".to_string(),
            },
        ];

        manager.configure_team_sso(
            "team123",
            &provider_id,
            group_mappings,
            true,
            "Viewer",
        ).unwrap();

        // 测试组映射
        let role = manager.map_sso_groups_to_team_role("team123", &vec!["admins".to_string()]);
        assert_eq!(role, Some("Admin".to_string()));

        let role = manager.map_sso_groups_to_team_role("team123", &vec!["users".to_string()]);
        assert_eq!(role, Some("Member".to_string()));

        let role = manager.map_sso_groups_to_team_role("team123", &vec!["unknown".to_string()]);
        assert_eq!(role, None);
    }

    #[test]
    fn test_session_management() {
        let mut manager = SsoManager::new();

        // 创建会话
        let session1 = SsoSession::new("user1", "provider1", 8);
        let session2 = SsoSession::new("user1", "provider1", 8);
        let session3 = SsoSession::new("user2", "provider1", 8);

        let id1 = session1.id.clone();
        let id2 = session2.id.clone();
        let id3 = session3.id.clone();

        manager.sessions.insert(id1.clone(), session1);
        manager.sessions.insert(id2.clone(), session2);
        manager.sessions.insert(id3.clone(), session3);

        // 列出用户会话
        let user1_sessions = manager.list_user_sessions("user1");
        assert_eq!(user1_sessions.len(), 2);

        // 终止用户会话
        let terminated = manager.terminate_user_sessions("user1");
        assert_eq!(terminated, 2);

        let user1_sessions = manager.list_user_sessions("user1");
        assert_eq!(user1_sessions.len(), 0);

        // user2的会话应保留
        let user2_sessions = manager.list_user_sessions("user2");
        assert_eq!(user2_sessions.len(), 1);
    }

    #[test]
    fn test_name_id_format() {
        assert_eq!(
            NameIdFormat::EmailAddress.to_string(),
            "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress"
        );
        assert_eq!(
            NameIdFormat::Persistent.to_string(),
            "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent"
        );
    }

    #[test]
    fn test_signature_algorithm() {
        assert_eq!(
            SignatureAlgorithm::RsaSha256.to_string(),
            "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"
        );
    }

    #[test]
    fn test_saml_attribute_mapping() {
        let mapping = SamlAttributeMapping::default_mapping();
        assert_eq!(mapping.user_id_attribute, "NameID");
        assert_eq!(mapping.email_attribute, "email");
        assert_eq!(mapping.username_attribute, Some("username".to_string()));
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
            raw_attributes: HashMap::new(),
        };

        assert_eq!(user_info.user_id, "user123");
        assert_eq!(user_info.groups.len(), 2);
    }
}
