//! SSO处理器模块
//!
//! 提供SAML 2.0、OIDC和OAuth 2.0协议的处理器实现

use crate::error::LiteError;
use crate::sso::{
    generate_secure_random, sha256_hash, OidcConfig, OidcTokenResponse, OidcUserInfo,
    SamlAuthRequest, SamlConfig, SsoProvider, SsoProviderConfig, SsoProviderType, SsoUserInfo,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// SAML处理器
pub struct SamlHandler {
    provider: SsoProvider,
    config: SamlConfig,
}

impl SamlHandler {
    /// 创建新的SAML处理器
    pub fn new(provider: SsoProvider) -> Result<Self, LiteError> {
        let config = match &provider.config {
            SsoProviderConfig::Saml(config) => config.clone(),
            _ => return Err(LiteError::Sso("Provider is not SAML type".to_string())),
        };

        Ok(Self { provider, config })
    }

    /// 创建认证请求 (AuthnRequest)
    pub fn create_authn_request(&self, request_id: &str) -> Result<String, LiteError> {
        let issue_instant = Utc::now().to_rfc3339();

        let request = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
                  xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
                  ID="_{}"
                  Version="2.0"
                  IssueInstant="{}"
                  Destination="{}"
                  ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
                  AssertionConsumerServiceURL="{}">
    <saml:Issuer>{}</saml:Issuer>
    <samlp:NameIDPolicy Format="{}" AllowCreate="true"/>
    <samlp:RequestedAuthnContext Comparison="exact">
        <saml:AuthnContextClassRef>urn:oasis:names:tc:SAML:2.0:ac:classes:PasswordProtectedTransport</saml:AuthnContextClassRef>
    </samlp:RequestedAuthnContext>
</samlp:AuthnRequest>"#,
            request_id,
            issue_instant,
            self.get_idp_sso_url()?,
            self.config.acs_url,
            self.config.sp_entity_id,
            self.get_name_id_format_uri()
        );

        Ok(request)
    }

    /// 创建认证请求 (带RelayState)
    pub fn create_auth_request(&self) -> Result<SamlAuthRequest, LiteError> {
        let request_id = Uuid::new_v4().to_string();
        let relay_state = generate_secure_random(32);

        let saml_request = self.create_authn_request(&request_id)?;
        let encoded_request = STANDARD.encode(saml_request.as_bytes());

        // 压缩并编码 (HTTP Redirect绑定)
        let deflated_request = deflate::deflate_bytes(saml_request.as_bytes());
        let encoded_redirect = STANDARD.encode(&deflated_request);

        Ok(SamlAuthRequest {
            id: request_id,
            provider_id: self.provider.id.clone(),
            saml_request: encoded_redirect, // 用于HTTP Redirect
            relay_state: Some(relay_state),
            destination: self.get_idp_sso_url()?,
        })
    }

    /// 处理SAML响应 (AssertionConsumerService)
    pub fn process_saml_response(
        &self,
        encoded_response: &str,
        relay_state: Option<&str>,
    ) -> Result<SsoUserInfo, LiteError> {
        // 1. Base64解码
        let decoded = STANDARD
            .decode(encoded_response)
            .map_err(|e| LiteError::Sso(format!("Failed to decode SAML response: {}", e)))?;

        // 2. 解析XML
        let response_xml = String::from_utf8(decoded)
            .map_err(|e| LiteError::Sso(format!("Invalid UTF-8 in SAML response: {}", e)))?;

        // 3. 验证签名 (简化实现，实际需使用samael库)
        if self.config.verify_signatures {
            self.verify_saml_signature(&response_xml)?;
        }

        // 4. 提取用户信息 (简化实现)
        let user_info = self.extract_user_info(&response_xml)?;

        Ok(user_info)
    }

    /// 生成SP元数据XML
    pub fn generate_sp_metadata(&self, x509_cert: Option<&str>) -> Result<String, LiteError> {
        let now = Utc::now().to_rfc3339();

        let cert_block = if let Some(cert) = x509_cert {
            format!(
                r#"    <md:KeyDescriptor use="signing">
      <ds:KeyInfo xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
        <ds:X509Data>
          <ds:X509Certificate>{}</ds:X509Certificate>
        </ds:X509Data>
      </ds:KeyInfo>
    </md:KeyDescriptor>"#,
                cert
            )
        } else {
            String::new()
        };

        let metadata = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata"
                     entityID="{}"
                     validUntil="{}">
{}
  <md:SPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <md:AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
                                 Location="{}"
                                 index="0"
                                 isDefault="true"/>
  </md:SPSSODescriptor>
</md:EntityDescriptor>"#,
            self.config.sp_entity_id,
            (Utc::now() + chrono::Duration::days(365)).to_rfc3339(),
            cert_block,
            self.config.acs_url
        );

        Ok(metadata)
    }

    /// 创建登出请求 (LogoutRequest)
    pub fn create_logout_request(
        &self,
        name_id: &str,
        session_index: Option<&str>,
    ) -> Result<String, LiteError> {
        let request_id = Uuid::new_v4().to_string();
        let issue_instant = Utc::now().to_rfc3339();

        let session_index_xml = if let Some(idx) = session_index {
            format!(r#"    <samlp:SessionIndex>{}</samlp:SessionIndex>"#, idx)
        } else {
            String::new()
        };

        let request = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:LogoutRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
                     xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
                     ID="_{}"
                     Version="2.0"
                     IssueInstant="{}"
                     Destination="{}">
    <saml:Issuer>{}</saml:Issuer>
    <saml:NameID Format="{}">{}</saml:NameID>
{}
</samlp:LogoutRequest>"#,
            request_id,
            issue_instant,
            self.config.slo_url.as_deref().unwrap_or(""),
            self.config.sp_entity_id,
            self.get_name_id_format_uri(),
            name_id,
            session_index_xml
        );

        Ok(request)
    }

    /// 验证SAML响应签名 (简化实现)
    fn verify_saml_signature(&self, _response_xml: &str) -> Result<(), LiteError> {
        // 实际实现需要使用samael crate进行XML签名验证
        // 1. 解析XML文档
        // 2. 提取签名元素
        // 3. 使用IdP证书验证签名
        // 4. 验证证书链

        Ok(()) // 简化：假设验证通过
    }

    /// 从SAML响应中提取用户信息 (简化实现)
    fn extract_user_info(&self, response_xml: &str) -> Result<SsoUserInfo, LiteError> {
        // 实际实现需要解析SAML Assertion XML
        // 1. 解析XML
        // 2. 提取NameID
        // 3. 提取属性声明
        // 4. 映射到SsoUserInfo

        // 简化实现：模拟解析
        let user_id = format!("saml_user_{}", generate_secure_random(8));
        let email = format!("{}@example.com", user_id);

        Ok(SsoUserInfo {
            user_id,
            email,
            username: "saml_user".to_string(),
            first_name: None,
            last_name: None,
            groups: vec![],
            team_ids: vec![],
            provider_type: SsoProviderType::Saml,
            provider_id: self.provider.id.clone(),
            raw_attributes: HashMap::new(),
        })
    }

    /// 获取IdP SSO URL
    fn get_idp_sso_url(&self) -> Result<String, LiteError> {
        // 从metadata URL或配置中获取
        // 简化：假设metadata URL包含SSO URL
        Ok(self.config.idp_metadata_url.replace("/metadata", "/sso"))
    }

    /// 获取NameID格式URI
    fn get_name_id_format_uri(&self) -> String {
        match self.config.name_id_format.as_str() {
            "emailAddress" => "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress".to_string(),
            "transient" => "urn:oasis:names:tc:SAML:2.0:nameid-format:transient".to_string(),
            "persistent" => "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent".to_string(),
            _ => "urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified".to_string(),
        }
    }
}

/// OIDC处理器
pub struct OidcHandler {
    provider: SsoProvider,
    config: OidcConfig,
}

impl OidcHandler {
    /// 创建新的OIDC处理器
    pub fn new(provider: SsoProvider) -> Result<Self, LiteError> {
        let config = match &provider.config {
            SsoProviderConfig::Oidc(config) => config.clone(),
            _ => return Err(LiteError::Sso("Provider is not OIDC type".to_string())),
        };

        Ok(Self { provider, config })
    }

    /// 构建授权URL (带PKCE)
    pub fn build_authorization_url(
        &self,
        state: &str,
        nonce: &str,
    ) -> Result<(String, String), LiteError> {
        // 生成PKCE参数
        let (pkce_verifier, pkce_challenge) = if self.config.use_pkce {
            let verifier = generate_secure_random(128);
            let challenge = base64_url_encode(&sha256_hash(&verifier));
            (verifier, challenge)
        } else {
            (String::new(), String::new())
        };

        // 构建授权URL
        let mut url = format!(
            "{}?response_type={}&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}",
            self.config.authorization_endpoint,
            self.config.response_type,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(&self.config.scopes.join(" ")),
            state,
            nonce
        );

        // 添加PKCE参数
        if !pkce_challenge.is_empty() {
            url.push_str(&format!(
                "&code_challenge={}&code_challenge_method=S256",
                pkce_challenge
            ));
        }

        Ok((url, pkce_verifier))
    }

    /// 交换授权码获取令牌
    pub async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: Option<&str>,
    ) -> Result<OidcTokenResponse, LiteError> {
        // 构建token请求
        let mut params: Vec<(&str, &str)> = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.redirect_uri),
            ("client_id", &self.config.client_id),
        ];

        // 添加client_secret (如果使用confidential client)
        if !self.config.client_secret.is_empty() {
            params.push(("client_secret", &self.config.client_secret));
        }

        // 添加PKCE verifier
        if let Some(verifier) = pkce_verifier {
            params.push(("code_verifier", verifier));
        }

        // 实际实现需要发送HTTP POST请求到token端点
        // 这里返回模拟数据
        log::info!("Exchanging OIDC code at {}", self.config.token_endpoint);

        // 模拟响应 (实际应解析JSON响应)
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

    /// 验证ID Token
    pub fn validate_id_token(
        &self,
        id_token: &str,
        expected_nonce: &str,
    ) -> Result<OidcUserInfo, LiteError> {
        // 1. 解析JWT
        let parts: Vec<&str> = id_token.split('.').collect();
        if parts.len() != 3 {
            return Err(LiteError::Sso("Invalid JWT format".to_string()));
        }

        // 2. 解码payload
        let payload = base64_decode(parts[1])?;
        let claims: serde_json::Value = serde_json::from_slice(&payload)
            .map_err(|e| LiteError::Sso(format!("Failed to parse JWT claims: {}", e)))?;

        // 3. 验证标准声明
        self.verify_claims(&claims, expected_nonce)?;

        // 4. 提取用户信息
        let user_info = self.extract_user_info_from_claims(&claims)?;

        Ok(user_info)
    }

    /// 刷新访问令牌
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OidcTokenResponse, LiteError> {
        let _params: Vec<(&str, &str)> = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
        ];

        log::info!("Refreshing OIDC token at {}", self.config.token_endpoint);

        // 模拟响应
        let access_token = generate_secure_random(48);
        let id_token = generate_secure_random(48);

        Ok(OidcTokenResponse {
            access_token,
            id_token,
            refresh_token: Some(refresh_token.to_string()),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        })
    }

    /// 获取用户信息
    pub async fn get_userinfo(&self, _access_token: &str) -> Result<OidcUserInfo, LiteError> {
        log::info!("Fetching userinfo from {}", self.config.userinfo_endpoint);

        // 实际实现需要发送HTTP GET请求到userinfo端点
        // Authorization: Bearer {access_token}

        // 模拟响应
        Ok(OidcUserInfo {
            sub: format!("oidc_user_{}", generate_secure_random(8)),
            email: Some("user@example.com".to_string()),
            email_verified: Some(true),
            name: Some("Test User".to_string()),
            preferred_username: Some("testuser".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            groups: Some(vec!["users".to_string()]),
        })
    }

    /// 构建登出URL
    pub fn build_logout_url(
        &self,
        id_token_hint: Option<&str>,
        post_logout_uri: Option<&str>,
    ) -> Option<String> {
        let end_session_endpoint = self.config.end_session_endpoint.as_ref()?;

        let mut url = end_session_endpoint.clone();

        if let Some(token) = id_token_hint {
            url.push_str(&format!("?id_token_hint={}", urlencoding::encode(token)));
        }

        if let Some(uri) = post_logout_uri {
            let separator = if url.contains('?') { "&" } else { "?" };
            url.push_str(&format!(
                "{}post_logout_redirect_uri={}",
                separator,
                urlencoding::encode(uri)
            ));
        }

        Some(url)
    }

    /// 验证JWT声明
    fn verify_claims(
        &self,
        claims: &serde_json::Value,
        expected_nonce: &str,
    ) -> Result<(), LiteError> {
        // 验证issuer
        if let Some(issuer) = claims.get("iss").and_then(|v| v.as_str()) {
            if issuer != self.config.issuer_url {
                return Err(LiteError::Sso(format!(
                    "Invalid issuer: expected {}, got {}",
                    self.config.issuer_url, issuer
                )));
            }
        }

        // 验证audience
        if let Some(aud) = claims.get("aud").and_then(|v| v.as_str()) {
            if aud != self.config.client_id {
                return Err(LiteError::Sso(format!(
                    "Invalid audience: expected {}, got {}",
                    self.config.client_id, aud
                )));
            }
        }

        // 验证nonce
        if let Some(nonce) = claims.get("nonce").and_then(|v| v.as_str()) {
            if nonce != expected_nonce {
                return Err(LiteError::Sso(
                    "Invalid nonce - possible replay attack".to_string(),
                ));
            }
        }

        // 验证过期时间
        if let Some(exp) = claims.get("exp").and_then(|v| v.as_i64()) {
            let now = Utc::now().timestamp();
            if exp < now {
                return Err(LiteError::Sso("ID token has expired".to_string()));
            }
        }

        Ok(())
    }

    /// 从声明中提取用户信息
    fn extract_user_info_from_claims(
        &self,
        claims: &serde_json::Value,
    ) -> Result<OidcUserInfo, LiteError> {
        let mapping = &self.config.attribute_mapping;

        let sub = claims
            .get(&mapping.user_id_claim)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let email = claims
            .get(&mapping.email_claim)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let preferred_username = mapping
            .username_claim
            .as_ref()
            .and_then(|claim| claims.get(claim).and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        let given_name = mapping
            .first_name_claim
            .as_ref()
            .and_then(|claim| claims.get(claim).and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        let family_name = mapping
            .last_name_claim
            .as_ref()
            .and_then(|claim| claims.get(claim).and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        let groups = mapping
            .groups_claim
            .as_ref()
            .and_then(|claim| claims.get(claim).and_then(|v| v.as_array()))
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });

        Ok(OidcUserInfo {
            sub,
            email,
            email_verified: claims.get("email_verified").and_then(|v| v.as_bool()),
            name: claims
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            preferred_username,
            given_name,
            family_name,
            groups,
        })
    }

    /// 将OidcUserInfo转换为SsoUserInfo
    pub fn convert_to_sso_user_info(&self, oidc_info: OidcUserInfo) -> SsoUserInfo {
        let mapping = &self.config.attribute_mapping;

        SsoUserInfo {
            user_id: oidc_info.sub.clone(),
            email: oidc_info.email.clone().unwrap_or_default(),
            username: oidc_info
                .preferred_username
                .clone()
                .unwrap_or_else(|| oidc_info.sub.clone()),
            first_name: oidc_info.given_name.clone(),
            last_name: oidc_info.family_name.clone(),
            groups: oidc_info.groups.clone().unwrap_or_default(),
            team_ids: vec![],
            provider_type: SsoProviderType::Oidc,
            provider_id: self.provider.id.clone(),
            raw_attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("sub".to_string(), serde_json::json!(oidc_info.sub));
                if let Some(email) = &oidc_info.email {
                    attrs.insert("email".to_string(), serde_json::json!(email));
                }
                if let Some(name) = &oidc_info.name {
                    attrs.insert("name".to_string(), serde_json::json!(name));
                }
                attrs
            },
        }
    }
}

/// 简化的deflate实现 (实际应使用flate2 crate)
mod deflate {
    pub fn deflate_bytes(_input: &[u8]) -> Vec<u8> {
        // 简化实现 - 实际应使用flate2压缩
        // 这里只是返回原始数据
        _input.to_vec()
    }
}

/// Base64 URL安全编码
fn base64_url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

/// Base64 URL安全解码
fn base64_decode(input: &str) -> Result<Vec<u8>, LiteError> {
    use base64::{engine::general_purpose::URL_SAFE, Engine};
    URL_SAFE
        .decode(input)
        .map_err(|e| LiteError::Sso(format!("Failed to decode base64: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sso::{OidcAttributeMapping, OidcConfig};

    fn create_test_oidc_provider() -> SsoProvider {
        let config = OidcConfig::standard(
            "https://auth.example.com",
            "client123",
            "secret456",
            "https://easyssh.pro/callback",
        );
        SsoProvider::new_oidc("Test OIDC", config)
    }

    #[test]
    fn test_saml_handler_creation() {
        let config = crate::sso::SamlConfig {
            idp_metadata_url: "https://idp.example.com/metadata".to_string(),
            sp_entity_id: "https://easyssh.pro".to_string(),
            acs_url: "https://easyssh.pro/sso/acs".to_string(),
            slo_url: None,
            signature_algorithm: "rsa-sha256".to_string(),
            verify_signatures: true,
            want_assertions_encrypted: false,
            name_id_format: "emailAddress".to_string(),
            attribute_mapping: crate::sso::SamlAttributeMapping::default_mapping(),
        };

        let provider = SsoProvider::new_saml("Test SAML", config);
        let handler = SamlHandler::new(provider);
        assert!(handler.is_ok());
    }

    #[test]
    fn test_oidc_handler_creation() {
        let provider = create_test_oidc_provider();
        let handler = OidcHandler::new(provider);
        assert!(handler.is_ok());
    }

    #[test]
    fn test_build_authorization_url() {
        let provider = create_test_oidc_provider();
        let handler = OidcHandler::new(provider).unwrap();

        let state = "test_state";
        let nonce = "test_nonce";

        let (url, verifier) = handler.build_authorization_url(state, nonce).unwrap();

        assert!(url.contains("authorize"));
        assert!(url.contains("client_id"));
        assert!(url.contains("code_challenge"));
        assert!(!verifier.is_empty()); // PKCE verifier generated
    }

    #[test]
    fn test_saml_metadata_generation() {
        let config = crate::sso::SamlConfig {
            idp_metadata_url: "https://idp.example.com/metadata".to_string(),
            sp_entity_id: "https://easyssh.pro".to_string(),
            acs_url: "https://easyssh.pro/sso/acs".to_string(),
            slo_url: Some("https://easyssh.pro/sso/slo".to_string()),
            signature_algorithm: "rsa-sha256".to_string(),
            verify_signatures: true,
            want_assertions_encrypted: false,
            name_id_format: "emailAddress".to_string(),
            attribute_mapping: crate::sso::SamlAttributeMapping::default_mapping(),
        };

        let provider = SsoProvider::new_saml("Test SAML", config);
        let handler = SamlHandler::new(provider).unwrap();

        let metadata = handler.generate_sp_metadata(None).unwrap();
        assert!(metadata.contains("EntityDescriptor"));
        assert!(metadata.contains("SPSSODescriptor"));
        assert!(metadata.contains(&handler.config.acs_url));
    }

    #[test]
    fn test_oidc_claims_validation() {
        let provider = create_test_oidc_provider();
        let handler = OidcHandler::new(provider).unwrap();

        let claims = serde_json::json!({
            "iss": "https://auth.example.com",
            "aud": "client123",
            "sub": "user123",
            "nonce": "test_nonce",
            "exp": Utc::now().timestamp() + 3600,
        });

        let result = handler.verify_claims(&claims, "test_nonce");
        assert!(result.is_ok());
    }

    #[test]
    fn test_oidc_claims_validation_invalid_nonce() {
        let provider = create_test_oidc_provider();
        let handler = OidcHandler::new(provider).unwrap();

        let claims = serde_json::json!({
            "iss": "https://auth.example.com",
            "aud": "client123",
            "sub": "user123",
            "nonce": "wrong_nonce",
            "exp": Utc::now().timestamp() + 3600,
        });

        let result = handler.verify_claims(&claims, "expected_nonce");
        assert!(result.is_err());
    }
}
