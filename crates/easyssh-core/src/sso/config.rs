//! SSO配置模块
//!
//! 定义SAML 2.0、OIDC、OAuth 2.0和LDAP的配置结构

use serde::{Deserialize, Serialize};

/// SAML 2.0 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlConfig {
    /// IdP元数据URL
    pub idp_metadata_url: String,
    /// SP Entity ID (本应用)
    pub sp_entity_id: String,
    /// ACS URL (断言消费服务)
    pub acs_url: String,
    /// SLO URL (单点登出，可选)
    pub slo_url: Option<String>,
    /// 签名算法
    pub signature_algorithm: String,
    /// 是否验证IdP签名
    pub verify_signatures: bool,
    /// 是否加密断言
    pub want_assertions_encrypted: bool,
    /// 名称ID格式
    pub name_id_format: String,
    /// 属性映射配置
    pub attribute_mapping: SamlAttributeMapping,
}

impl SamlConfig {
    /// 创建Okta标准配置
    pub fn okta(
        domain: &str,
        sp_entity_id: &str,
        acs_url: &str,
    ) -> Self {
        Self {
            idp_metadata_url: format!("https://{}/.well-known/saml.xml", domain),
            sp_entity_id: sp_entity_id.to_string(),
            acs_url: acs_url.to_string(),
            slo_url: Some(format!("https://{}/sso/slo", domain)),
            signature_algorithm: "rsa-sha256".to_string(),
            verify_signatures: true,
            want_assertions_encrypted: false,
            name_id_format: "emailAddress".to_string(),
            attribute_mapping: SamlAttributeMapping::default_mapping(),
        }
    }

    /// 创建Azure AD标准配置
    pub fn azure_ad(
        tenant_id: &str,
        sp_entity_id: &str,
        acs_url: &str,
    ) -> Self {
        Self {
            idp_metadata_url: format!(
                "https://login.microsoftonline.com/{}/federationmetadata/2007-06/federationmetadata.xml",
                tenant_id
            ),
            sp_entity_id: sp_entity_id.to_string(),
            acs_url: acs_url.to_string(),
            slo_url: Some(format!(
                "https://login.microsoftonline.com/{}/saml2",
                tenant_id
            )),
            signature_algorithm: "rsa-sha256".to_string(),
            verify_signatures: true,
            want_assertions_encrypted: false,
            name_id_format: "persistent".to_string(),
            attribute_mapping: SamlAttributeMapping::azure_ad_mapping(),
        }
    }

    /// 创建Google Workspace标准配置
    pub fn google_workspace(
        domain: &str,
        sp_entity_id: &str,
        acs_url: &str,
    ) -> Self {
        Self {
            idp_metadata_url: format!("https://accounts.google.com/o/saml2/metadata?idpid={}", domain),
            sp_entity_id: sp_entity_id.to_string(),
            acs_url: acs_url.to_string(),
            slo_url: None,
            signature_algorithm: "rsa-sha256".to_string(),
            verify_signatures: true,
            want_assertions_encrypted: false,
            name_id_format: "emailAddress".to_string(),
            attribute_mapping: SamlAttributeMapping::google_mapping(),
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

    /// 创建Azure AD标准映射
    pub fn azure_ad_mapping() -> Self {
        Self {
            user_id_attribute: "http://schemas.microsoft.com/identity/claims/objectidentifier".to_string(),
            email_attribute: "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress".to_string(),
            username_attribute: Some("http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name".to_string()),
            first_name_attribute: Some("http://schemas.xmlsoap.org/ws/2005/05/identity/claims/givenname".to_string()),
            last_name_attribute: Some("http://schemas.xmlsoap.org/ws/2005/05/identity/claims/surname".to_string()),
            groups_attribute: Some("http://schemas.microsoft.com/ws/2008/06/identity/claims/groups".to_string()),
            team_attribute: None,
        }
    }

    /// 创建Google Workspace标准映射
    pub fn google_mapping() -> Self {
        Self {
            user_id_attribute: "email".to_string(),
            email_attribute: "email".to_string(),
            username_attribute: Some("email".to_string()),
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
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
            response_type: "code".to_string(),
            attribute_mapping: OidcAttributeMapping::default_mapping(),
            use_pkce: true,
        }
    }

    /// 创建Okta OIDC配置
    pub fn okta(
        domain: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Self {
        Self {
            issuer_url: format!("https://{}", domain),
            authorization_endpoint: format!("https://{}/oauth2/v1/authorize", domain),
            token_endpoint: format!("https://{}/oauth2/v1/token", domain),
            userinfo_endpoint: format!("https://{}/oauth2/v1/userinfo", domain),
            jwks_uri: format!("https://{}/oauth2/v1/keys", domain),
            end_session_endpoint: Some(format!("https://{}/oauth2/v1/logout", domain)),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
                "groups".to_string(),
            ],
            response_type: "code".to_string(),
            attribute_mapping: OidcAttributeMapping::okta_mapping(),
            use_pkce: true,
        }
    }

    /// 创建Azure AD OIDC配置
    pub fn azure_ad(
        tenant_id: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Self {
        Self {
            issuer_url: format!("https://login.microsoftonline.com/{}/v2.0", tenant_id),
            authorization_endpoint: format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize",
                tenant_id
            ),
            token_endpoint: format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
                tenant_id
            ),
            userinfo_endpoint: "https://graph.microsoft.com/oidc/userinfo".to_string(),
            jwks_uri: format!(
                "https://login.microsoftonline.com/{}/discovery/v2.0/keys",
                tenant_id
            ),
            end_session_endpoint: Some(format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/logout",
                tenant_id
            )),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
                "User.Read".to_string(),
            ],
            response_type: "code".to_string(),
            attribute_mapping: OidcAttributeMapping::azure_ad_mapping(),
            use_pkce: true,
        }
    }

    /// 创建Google Workspace OIDC配置
    pub fn google_workspace(
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Self {
        Self {
            issuer_url: "https://accounts.google.com".to_string(),
            authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
            userinfo_endpoint: "https://openidconnect.googleapis.com/v1/userinfo".to_string(),
            jwks_uri: "https://www.googleapis.com/oauth2/v3/certs".to_string(),
            end_session_endpoint: Some("https://accounts.google.com/logout".to_string()),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
            response_type: "code".to_string(),
            attribute_mapping: OidcAttributeMapping::google_mapping(),
            use_pkce: true,
        }
    }

    /// 创建Keycloak配置
    pub fn keycloak(
        realm_url: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Self {
        Self {
            issuer_url: format!("{}", realm_url),
            authorization_endpoint: format!("{}/protocol/openid-connect/auth", realm_url),
            token_endpoint: format!("{}/protocol/openid-connect/token", realm_url),
            userinfo_endpoint: format!("{}/protocol/openid-connect/userinfo", realm_url),
            jwks_uri: format!("{}/protocol/openid-connect/certs", realm_url),
            end_session_endpoint: Some(format!("{}/protocol/openid-connect/logout", realm_url)),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
            response_type: "code".to_string(),
            attribute_mapping: OidcAttributeMapping::default_mapping(),
            use_pkce: true,
        }
    }

    /// 从发现端点URL创建配置
    pub fn from_discovery_url(
        discovery_url: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Self {
        // 从发现端点URL推断基础URL
        let base_url = discovery_url.trim_end_matches("/.well-known/openid-configuration");

        Self {
            issuer_url: base_url.to_string(),
            authorization_endpoint: format!("{}/oauth2/authorize", base_url),
            token_endpoint: format!("{}/oauth2/token", base_url),
            userinfo_endpoint: format!("{}/oauth2/userinfo", base_url),
            jwks_uri: format!("{}/oauth2/keys", base_url),
            end_session_endpoint: Some(format!("{}/oauth2/logout", base_url)),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
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

    /// 创建Okta标准映射
    pub fn okta_mapping() -> Self {
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

    /// 创建Azure AD标准映射
    pub fn azure_ad_mapping() -> Self {
        Self {
            user_id_claim: "oid".to_string(),
            email_claim: "email".to_string(),
            username_claim: Some("preferred_username".to_string()),
            first_name_claim: Some("given_name".to_string()),
            last_name_claim: Some("family_name".to_string()),
            groups_claim: Some("groups".to_string()),
            team_claim: None,
        }
    }

    /// 创建Google Workspace标准映射
    pub fn google_mapping() -> Self {
        Self {
            user_id_claim: "sub".to_string(),
            email_claim: "email".to_string(),
            username_claim: Some("email".to_string()),
            first_name_claim: Some("given_name".to_string()),
            last_name_claim: Some("family_name".to_string()),
            groups_claim: None,
            team_claim: None,
        }
    }
}

/// OAuth 2.0 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    /// 授权端点
    pub authorization_endpoint: String,
    /// Token端点
    pub token_endpoint: String,
    /// 用户信息端点 (可选)
    pub userinfo_endpoint: Option<String>,
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
    /// PKCE是否启用
    pub use_pkce: bool,
    /// 属性映射配置
    pub attribute_mapping: OAuth2AttributeMapping,
}

impl OAuth2Config {
    /// 创建标准OAuth2配置
    pub fn standard(
        authorization_endpoint: &str,
        token_endpoint: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Self {
        Self {
            authorization_endpoint: authorization_endpoint.to_string(),
            token_endpoint: token_endpoint.to_string(),
            userinfo_endpoint: None,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            response_type: "code".to_string(),
            use_pkce: true,
            attribute_mapping: OAuth2AttributeMapping::default_mapping(),
        }
    }

    /// 创建GitHub OAuth2配置
    pub fn github(
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Self {
        Self {
            authorization_endpoint: "https://github.com/login/oauth/authorize".to_string(),
            token_endpoint: "https://github.com/login/oauth/access_token".to_string(),
            userinfo_endpoint: Some("https://api.github.com/user".to_string()),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec!["read:user".to_string(), "user:email".to_string()],
            response_type: "code".to_string(),
            use_pkce: false,
            attribute_mapping: OAuth2AttributeMapping::github_mapping(),
        }
    }
}

/// OAuth2属性映射
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OAuth2AttributeMapping {
    /// 用户ID字段
    pub user_id_field: String,
    /// 邮箱字段
    pub email_field: String,
    /// 用户名字段
    pub username_field: Option<String>,
    /// 名字字段
    pub first_name_field: Option<String>,
    /// 姓氏字段
    pub last_name_field: Option<String>,
}

impl OAuth2AttributeMapping {
    /// 创建默认映射
    pub fn default_mapping() -> Self {
        Self {
            user_id_field: "id".to_string(),
            email_field: "email".to_string(),
            username_field: Some("username".to_string()),
            first_name_field: Some("first_name".to_string()),
            last_name_field: Some("last_name".to_string()),
        }
    }

    /// 创建GitHub映射
    pub fn github_mapping() -> Self {
        Self {
            user_id_field: "id".to_string(),
            email_field: "email".to_string(),
            username_field: Some("login".to_string()),
            first_name_field: Some("name".to_string()),
            last_name_field: None,
        }
    }
}

/// LDAP配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LdapConfig {
    /// LDAP服务器地址
    pub server_url: String,
    /// 绑定DN
    pub bind_dn: String,
    /// 绑定密码
    #[serde(skip_serializing)]
    pub bind_password: String,
    /// 用户搜索基础DN
    pub user_base_dn: String,
    /// 用户搜索过滤器
    pub user_search_filter: String,
    /// 组搜索基础DN
    pub group_base_dn: String,
    /// 组搜索过滤器
    pub group_search_filter: String,
    /// 是否使用TLS
    pub use_tls: bool,
    /// 是否使用StartTLS
    pub use_starttls: bool,
    /// TLS证书验证
    pub tls_verify_cert: bool,
    /// 属性映射配置
    pub attribute_mapping: LdapAttributeMapping,
}

/// LDAP属性映射
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LdapAttributeMapping {
    /// 用户ID属性
    pub user_id_attribute: String,
    /// 邮箱属性
    pub email_attribute: String,
    /// 用户名属性
    pub username_attribute: String,
    /// 名字属性
    pub first_name_attribute: String,
    /// 姓氏属性
    pub last_name_attribute: String,
    /// 组成员属性
    pub member_of_attribute: String,
}

impl LdapAttributeMapping {
    /// 创建标准Active Directory映射
    pub fn active_directory() -> Self {
        Self {
            user_id_attribute: "objectGUID".to_string(),
            email_attribute: "mail".to_string(),
            username_attribute: "sAMAccountName".to_string(),
            first_name_attribute: "givenName".to_string(),
            last_name_attribute: "sn".to_string(),
            member_of_attribute: "memberOf".to_string(),
        }
    }

    /// 创建标准OpenLDAP映射
    pub fn openldap() -> Self {
        Self {
            user_id_attribute: "entryUUID".to_string(),
            email_attribute: "mail".to_string(),
            username_attribute: "uid".to_string(),
            first_name_attribute: "givenName".to_string(),
            last_name_attribute: "sn".to_string(),
            member_of_attribute: "memberOf".to_string(),
        }
    }
}

/// OneLogin SAML配置快捷方式
pub fn onelogin_saml(
    account_id: &str,
    sp_entity_id: &str,
    acs_url: &str,
) -> SamlConfig {
    SamlConfig {
        idp_metadata_url: format!(
            "https://app.onelogin.com/saml/metadata/{}",
            account_id
        ),
        sp_entity_id: sp_entity_id.to_string(),
        acs_url: acs_url.to_string(),
        slo_url: Some(format!(
            "https://app.onelogin.com/trust/saml2/http-redirect/slo/{}/",
            account_id
        )),
        signature_algorithm: "rsa-sha256".to_string(),
        verify_signatures: true,
        want_assertions_encrypted: false,
        name_id_format: "emailAddress".to_string(),
        attribute_mapping: SamlAttributeMapping::default_mapping(),
    }
}

/// Authing OIDC配置快捷方式
pub fn authing_oidc(
    user_pool_id: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> OidcConfig {
    let base_url = format!("https://{}.authing.cn", user_pool_id);

    OidcConfig {
        issuer_url: base_url.clone(),
        authorization_endpoint: format!("{}/oidc/auth", base_url),
        token_endpoint: format!("{}/oidc/token", base_url),
        userinfo_endpoint: format!("{}/oidc/me", base_url),
        jwks_uri: format!("{}/oidc/.well-known/jwks.json", base_url),
        end_session_endpoint: Some(format!("{}/oidc/session/end", base_url)),
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        redirect_uri: redirect_uri.to_string(),
        scopes: vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
        ],
        response_type: "code".to_string(),
        attribute_mapping: OidcAttributeMapping::default_mapping(),
        use_pkce: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saml_config_okta() {
        let config = SamlConfig::okta(
            "example.okta.com",
            "https://easyssh.pro",
            "https://easyssh.pro/sso/acs",
        );

        assert!(config.idp_metadata_url.contains("okta.com"));
        assert_eq!(config.signature_algorithm, "rsa-sha256");
        assert!(config.verify_signatures);
    }

    #[test]
    fn test_oidc_config_azure() {
        let config = OidcConfig::azure_ad(
            "tenant-123",
            "client-id",
            "client-secret",
            "https://easyssh.pro/callback",
        );

        assert!(config.issuer_url.contains("microsoftonline.com"));
        assert!(config.authorization_endpoint.contains("authorize"));
        assert!(config.use_pkce);
    }

    #[test]
    fn test_oidc_config_google() {
        let config = OidcConfig::google_workspace(
            "client-id",
            "client-secret",
            "https://easyssh.pro/callback",
        );

        assert!(config.issuer_url.contains("google.com"));
        assert_eq!(config.scopes.len(), 3);
    }

    #[test]
    fn test_attribute_mappings() {
        let default_mapping = OidcAttributeMapping::default_mapping();
        assert_eq!(default_mapping.user_id_claim, "sub");
        assert_eq!(default_mapping.email_claim, "email");

        let azure_mapping = OidcAttributeMapping::azure_ad_mapping();
        assert_eq!(azure_mapping.user_id_claim, "oid");

        let google_mapping = OidcAttributeMapping::google_mapping();
        assert_eq!(google_mapping.user_id_claim, "sub");
    }

    #[test]
    fn test_ldap_mappings() {
        let ad_mapping = LdapAttributeMapping::active_directory();
        assert_eq!(ad_mapping.user_id_attribute, "objectGUID");
        assert_eq!(ad_mapping.username_attribute, "sAMAccountName");

        let openldap_mapping = LdapAttributeMapping::openldap();
        assert_eq!(openldap_mapping.user_id_attribute, "entryUUID");
        assert_eq!(openldap_mapping.username_attribute, "uid");
    }

    #[test]
    fn test_authing_config() {
        let config = authing_oidc(
            "userpool123",
            "client-id",
            "client-secret",
            "https://easyssh.pro/callback",
        );

        assert!(config.issuer_url.contains("authing.cn"));
        assert!(config.authorization_endpoint.contains("/oidc/auth"));
    }
}
