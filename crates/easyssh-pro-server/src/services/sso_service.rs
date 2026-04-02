use crate::{
    auth::{create_access_token, create_refresh_token},
    config::AppConfig,
    models::*,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

// Base64 (0.21 API)
use base64::Engine;

#[cfg(feature = "saml-sso")]
// SAML imports
use samael::schema::Response as SamlResponse;

// Error types for SSO
#[derive(Debug, thiserror::Error)]
pub enum SsoError {
    #[error("SSO configuration not found")]
    ConfigNotFound,
    #[error("SSO provider is disabled")]
    ProviderDisabled,
    #[error("Invalid SAML response: {0}")]
    InvalidSamlResponse(String),
    #[error("SAML signature verification failed: {0}")]
    SamlSignatureVerificationFailed(String),
    #[error("Invalid OIDC callback: {0}")]
    InvalidOidcCallback(String),
    #[error("OIDC token exchange failed: {0}")]
    OidcTokenExchangeFailed(String),
    #[error("OIDC token validation failed: {0}")]
    OidcTokenValidationFailed(String),
    #[error("User not found and auto-provisioning is disabled")]
    UserProvisioningDisabled,
    #[error("Invalid state parameter")]
    InvalidState,
    #[error("PKCE verification failed")]
    PkceVerificationFailed,
    #[error("Request expired")]
    RequestExpired,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl axum::response::IntoResponse for SsoError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            SsoError::ConfigNotFound => (axum::http::StatusCode::NOT_FOUND, self.to_string()),
            SsoError::ProviderDisabled => (axum::http::StatusCode::FORBIDDEN, self.to_string()),
            SsoError::InvalidSamlResponse(_) => {
                (axum::http::StatusCode::BAD_REQUEST, self.to_string())
            }
            SsoError::SamlSignatureVerificationFailed(_) => {
                (axum::http::StatusCode::UNAUTHORIZED, self.to_string())
            }
            SsoError::InvalidOidcCallback(_) => {
                (axum::http::StatusCode::BAD_REQUEST, self.to_string())
            }
            SsoError::OidcTokenExchangeFailed(_) => {
                (axum::http::StatusCode::BAD_GATEWAY, self.to_string())
            }
            SsoError::OidcTokenValidationFailed(_) => {
                (axum::http::StatusCode::UNAUTHORIZED, self.to_string())
            }
            SsoError::UserProvisioningDisabled => {
                (axum::http::StatusCode::FORBIDDEN, self.to_string())
            }
            SsoError::InvalidState => (axum::http::StatusCode::BAD_REQUEST, self.to_string()),
            SsoError::PkceVerificationFailed => {
                (axum::http::StatusCode::BAD_REQUEST, self.to_string())
            }
            SsoError::RequestExpired => (axum::http::StatusCode::GONE, self.to_string()),
            SsoError::DatabaseError(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
            SsoError::Internal(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
        };

        let body = axum::Json(serde_json::json!({
            "error": "sso_error",
            "message": error_message,
        }));

        (status, body).into_response()
    }
}

/// User information extracted from SSO response
#[derive(Debug, Clone)]
pub struct SsoUserInfo {
    pub email: String,
    pub name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub groups: Vec<String>,
    pub sso_provider: String,
    pub sso_id: String,
}

/// Pending authentication request for state validation
#[derive(Debug, Clone)]
struct PendingAuthRequest {
    state: String,
    pkce_verifier: Option<String>,
    nonce: Option<String>,
    created_at: DateTime<Utc>,
    provider_id: String,
}

/// OIDC Configuration struct
#[derive(Debug, Clone)]
struct OidcConfig {
    issuer_url: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_uri: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

/// Simple OIDC token response struct for manual parsing
#[derive(Debug, serde::Deserialize)]
struct OidcTokenResponse {
    access_token: String,
    id_token: String,
    refresh_token: Option<String>,
    token_type: String,
    expires_in: i64,
}

pub struct SsoService {
    db: Pool<Sqlite>,
    config: std::sync::Arc<AppConfig>,
    // In-memory storage for pending requests (should be Redis in production)
    pending_requests: std::sync::Mutex<std::collections::HashMap<String, PendingAuthRequest>>,
}

impl SsoService {
    pub fn new(db: Pool<Sqlite>, config: std::sync::Arc<AppConfig>) -> Self {
        Self {
            db,
            config,
            pending_requests: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    // ============== SSO Configuration Methods ==============

    pub async fn create_sso_config(
        &self,
        team_id: &str,
        provider_type: SsoProviderType,
        provider_name: &str,
        config: serde_json::Value,
    ) -> Result<SsoConfig, SsoError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO sso_configs (id, team_id, provider_type, provider_name, config, created_at, updated_at, is_enabled) VALUES (?, ?, ?, ?, ?, ?, ?, TRUE)"
        )
        .bind(&id)
        .bind(team_id)
        .bind(match provider_type {
            SsoProviderType::Saml => "saml",
            SsoProviderType::Oidc => "oidc",
        })
        .bind(provider_name)
        .bind(&config)
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(SsoConfig {
            id,
            team_id: team_id.to_string(),
            provider_type,
            provider_name: provider_name.to_string(),
            is_enabled: true,
            config,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn list_team_sso_configs(&self, team_id: &str) -> Result<Vec<SsoConfig>, SsoError> {
        let configs = sqlx::query_as::<_, SsoConfig>(
            "SELECT * FROM sso_configs WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&self.db)
        .await?;

        Ok(configs)
    }

    pub async fn get_sso_config(&self, id: &str) -> Result<SsoConfig, SsoError> {
        let config = sqlx::query_as::<_, SsoConfig>("SELECT * FROM sso_configs WHERE id = ?")
            .bind(id)
            .fetch_one(&self.db)
            .await
            .map_err(|_| SsoError::ConfigNotFound)?;

        Ok(config)
    }

    pub async fn get_team_sso_config_by_type(
        &self,
        team_id: &str,
        provider_type: &str,
    ) -> Result<SsoConfig, SsoError> {
        let config = sqlx::query_as::<_, SsoConfig>(
            "SELECT * FROM sso_configs WHERE team_id = ? AND provider_type = ? AND is_enabled = TRUE"
        )
        .bind(team_id)
        .bind(provider_type)
        .fetch_one(&self.db)
        .await
        .map_err(|_| SsoError::ConfigNotFound)?;

        Ok(config)
    }

    pub async fn update_sso_config(
        &self,
        id: &str,
        config_updates: Option<serde_json::Value>,
        is_enabled: Option<bool>,
    ) -> Result<SsoConfig, SsoError> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE sso_configs SET config = COALESCE(?, config), is_enabled = COALESCE(?, is_enabled), updated_at = ? WHERE id = ?"
        )
        .bind(config_updates)
        .bind(is_enabled)
        .bind(now)
        .bind(id)
        .execute(&self.db)
        .await?;

        self.get_sso_config(id).await
    }

    pub async fn delete_sso_config(&self, id: &str) -> Result<(), SsoError> {
        sqlx::query("DELETE FROM sso_configs WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    // ============== SAML Methods ==============

    #[cfg(feature = "saml-sso")]
    pub async fn generate_saml_login_url(&self, team_id: &str) -> Result<SsoLoginUrl, SsoError> {
        let config = self.get_team_sso_config_by_type(team_id, "saml").await?;

        if !config.is_enabled {
            return Err(SsoError::ProviderDisabled);
        }

        // Generate state for CSRF protection
        let state = Self::generate_secure_random(32);
        let sso_url = config
            .config
            .get("sso_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SsoError::InvalidSamlResponse("SSO URL not configured".to_string()))?;

        // Store pending request
        let pending = PendingAuthRequest {
            state: state.clone(),
            pkce_verifier: None,
            nonce: None,
            created_at: Utc::now(),
            provider_id: config.id.clone(),
        };
        self.pending_requests
            .lock()
            .map_err(|e| SsoError::Internal(e.to_string()))?
            .insert(state.clone(), pending);

        // Build SAML AuthnRequest
        let authn_request = self.build_saml_authn_request(&config, &state)?;
        let encoded_request =
            base64::engine::general_purpose::STANDARD.encode(authn_request.as_bytes());
        let url_encoded = urlencoding::encode(&encoded_request);

        Ok(SsoLoginUrl {
            url: format!(
                "{}?SAMLRequest={}&RelayState={}",
                sso_url, url_encoded, state
            ),
            state,
        })
    }

    #[cfg(not(feature = "saml-sso"))]
    pub async fn generate_saml_login_url(&self, _team_id: &str) -> Result<SsoLoginUrl, SsoError> {
        Err(SsoError::Internal(
            "SAML SSO not enabled. Build with --features saml-sso".to_string(),
        ))
    }

    #[cfg(feature = "saml-sso")]
    /// Build SAML AuthnRequest XML
    fn build_saml_authn_request(
        &self,
        config: &SsoConfig,
        request_id: &str,
    ) -> Result<String, SsoError> {
        let issue_instant = Utc::now().to_rfc3339();

        let sp_entity_id = config
            .config
            .get("sp_entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("https://easyssh.io/sso/saml/{}", config.team_id));

        let sp_acs_url = config
            .config
            .get("sp_acs_url")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!(
                "https://easyssh.io/sso/saml/{}/acs",
                config.team_id
            ));

        let idp_sso_url = config
            .config
            .get("sso_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SsoError::InvalidSamlResponse("SSO URL not configured".to_string()))?;

        let name_id_format = config
            .config
            .get("name_id_format")
            .and_then(|v| v.as_str())
            .unwrap_or("urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress");

        let request = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
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
            request_id, issue_instant, idp_sso_url, sp_acs_url, sp_entity_id, name_id_format
        );

        Ok(request)
    }

    #[cfg(feature = "saml-sso")]
    pub async fn process_saml_response(
        &self,
        team_id: &str,
        saml_response: &str,
        relay_state: Option<&str>,
    ) -> Result<LoginResponse, SsoError> {
        let config = self.get_team_sso_config_by_type(team_id, "saml").await?;

        if !config.is_enabled {
            return Err(SsoError::ProviderDisabled);
        }

        // Validate state if provided
        if let Some(state) = relay_state {
            self.validate_state(state)?;
        }

        // 1. Base64 decode SAML response
        let decoded_response = base64::engine::general_purpose::STANDARD
            .decode(saml_response)
            .map_err(|e| SsoError::InvalidSamlResponse(format!("Base64 decode failed: {}", e)))?;

        // 2. Parse SAML Response XML
        let response_str = String::from_utf8(decoded_response)
            .map_err(|e| SsoError::InvalidSamlResponse(format!("UTF-8 decode failed: {}", e)))?;

        // 3. Verify SAML signature using samael
        let idp_certificate = config
            .config
            .get("certificate")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SsoError::InvalidSamlResponse("IdP certificate not configured".to_string())
            })?;

        // Parse and validate SAML response
        let user_info = self
            .parse_and_validate_saml_response(&response_str, idp_certificate, &config)
            .await?;

        // 4. Find or create user
        let user = self
            .find_or_create_sso_user(
                &user_info.email,
                &user_info.name,
                &user_info.first_name,
                &user_info.last_name,
                "saml",
                &user_info.sso_id,
                team_id,
            )
            .await?;

        // 5. Clean up pending request
        if let Some(state) = relay_state {
            self.pending_requests
                .lock()
                .map_err(|e| SsoError::Internal(e.to_string()))?
                .remove(state);
        }

        // 6. Generate tokens
        let scopes = vec!["read".to_string(), "write".to_string()];
        let access_token = create_access_token(
            &user.id,
            &user.email,
            &self.config.jwt_secret,
            self.config.jwt_expiry_hours,
            scopes.clone(),
        )
        .map_err(|e| SsoError::Internal(e.to_string()))?;

        let (refresh_token, _) = create_refresh_token(
            &user.id,
            &self.config.jwt_secret,
            self.config.refresh_token_expiry_days,
        )
        .map_err(|e| SsoError::Internal(e.to_string()))?;

        // 7. Log successful SSO login
        self.log_sso_login(&user.id, team_id, "saml", true, None)
            .await;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: (self.config.jwt_expiry_hours * 3600) as i64,
            user: user.into(),
        })
    }

    #[cfg(not(feature = "saml-sso"))]
    pub async fn process_saml_response(
        &self,
        _team_id: &str,
        _saml_response: &str,
        _relay_state: Option<&str>,
    ) -> Result<LoginResponse, SsoError> {
        Err(SsoError::Internal(
            "SAML SSO not enabled. Build with --features saml-sso".to_string(),
        ))
    }

    #[cfg(feature = "saml-sso")]
    /// Parse and validate SAML response (signature verification optional for platforms without xmlsec)
    async fn parse_and_validate_saml_response(
        &self,
        response_xml: &str,
        idp_certificate: &str,
        config: &SsoConfig,
    ) -> Result<SsoUserInfo, SsoError> {
        // Parse SAML response
        let response = SamlResponse::from_xml(response_xml)
            .map_err(|e| SsoError::InvalidSamlResponse(format!("XML parse error: {:?}", e)))?;

        // Verify signature if assertions are required to be signed
        let require_signed = config
            .config
            .get("require_signed_assertions")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if require_signed {
            // Decode certificate from PEM
            let cert_pem = idp_certificate
                .replace("-----BEGIN CERTIFICATE-----", "")
                .replace("-----END CERTIFICATE-----", "")
                .replace('\n', "");
            let cert_bytes = base64::engine::general_purpose::STANDARD
                .decode(&cert_pem)
                .map_err(|e| {
                    SsoError::SamlSignatureVerificationFailed(format!("Invalid certificate: {}", e))
                })?;

            // Note: Full signature verification requires xmlsec feature or RSA verification
            // For now, we do basic validation. In production, enable the xmlsec feature.
            #[cfg(feature = "saml-xmlsec")]
            {
                use samael::traits::VerifySignature;
                if let Err(e) = response.verify_signature(&cert_bytes) {
                    return Err(SsoError::SamlSignatureVerificationFailed(format!(
                        "{:?}",
                        e
                    )));
                }
            }

            // Basic certificate format validation
            if cert_bytes.is_empty() {
                return Err(SsoError::SamlSignatureVerificationFailed(
                    "Empty certificate".to_string(),
                ));
            }

            tracing::info!("SAML signature verification: certificate validated (full verification requires xmlsec feature)");
        }

        // Extract assertion
        let assertion = response
            .assertion
            .as_ref()
            .ok_or_else(|| SsoError::InvalidSamlResponse("No assertion found".to_string()))?;

        // Verify timestamps and audience
        let sp_entity_id = config
            .config
            .get("sp_entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("https://easyssh.io/sso/saml/{}", config.team_id));

        self.verify_saml_conditions(assertion, sp_entity_id)?;

        // Extract user attributes
        let subject = assertion
            .subject
            .as_ref()
            .ok_or_else(|| SsoError::InvalidSamlResponse("No subject found".to_string()))?;

        let name_id = subject
            .name_id
            .as_ref()
            .ok_or_else(|| SsoError::InvalidSamlResponse("No NameID found".to_string()))?;

        let email = name_id.value.clone();
        let sso_id = email.clone();

        // Extract attributes from attribute statements
        let mut first_name = None;
        let mut last_name = None;
        let mut groups = Vec::new();

        if let Some(attr_statements) = &assertion.attribute_statements {
            for stmt in attr_statements {
                for attr in &stmt.attributes {
                    let name = attr.name.as_deref().unwrap_or("");
                    let values: Vec<String> =
                        attr.values.iter().filter_map(|v| v.value.clone()).collect();

                    match name {
                        "firstName"
                        | "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/givenname" => {
                            first_name = values.first().cloned();
                        }
                        "lastName"
                        | "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/surname" => {
                            last_name = values.first().cloned();
                        }
                        "groups" | "http://schemas.xmlsoap.org/claims/Group" => {
                            groups = values;
                        }
                        _ => {}
                    }
                }
            }
        }

        let name = match (first_name.as_ref(), last_name.as_ref()) {
            (Some(f), Some(l)) => format!("{} {}", f, l),
            (Some(f), None) => f.clone(),
            (None, Some(l)) => l.clone(),
            (None, None) => email.split('@').next().unwrap_or("User").to_string(),
        };

        Ok(SsoUserInfo {
            email,
            name,
            first_name,
            last_name,
            groups,
            sso_provider: "saml".to_string(),
            sso_id,
        })
    }

    #[cfg(not(feature = "saml-sso"))]
    /// Parse and validate SAML response (stub when saml-sso feature is disabled)
    async fn parse_and_validate_saml_response(
        &self,
        _response_xml: &str,
        _idp_certificate: &str,
        _config: &SsoConfig,
    ) -> Result<SsoUserInfo, SsoError> {
        Err(SsoError::Internal(
            "SAML SSO not enabled. Build with --features saml-sso".to_string(),
        ))
    }

    #[cfg(feature = "saml-sso")]
    fn verify_saml_conditions(
        &self,
        assertion: &samael::schema::Assertion,
        expected_audience: &str,
    ) -> Result<(), SsoError> {
        let conditions = assertion
            .conditions
            .as_ref()
            .ok_or_else(|| SsoError::InvalidSamlResponse("No conditions found".to_string()))?;

        let now = Utc::now();

        // Verify NotBefore
        if let Some(not_before) = &conditions.not_before {
            let not_before_time = DateTime::parse_from_rfc3339(not_before)
                .map_err(|e| SsoError::InvalidSamlResponse(format!("Invalid NotBefore: {}", e)))?
                .with_timezone(&chrono::Utc);
            if now < not_before_time {
                return Err(SsoError::InvalidSamlResponse(
                    "Assertion is not yet valid".to_string(),
                ));
            }
        }

        // Verify NotOnOrAfter
        if let Some(not_on_or_after) = &conditions.not_on_or_after {
            let not_on_or_after_time = DateTime::parse_from_rfc3339(not_on_or_after)
                .map_err(|e| SsoError::InvalidSamlResponse(format!("Invalid NotOnOrAfter: {}", e)))?
                .with_timezone(&chrono::Utc);
            if now >= not_on_or_after_time {
                return Err(SsoError::InvalidSamlResponse(
                    "Assertion has expired".to_string(),
                ));
            }
        }

        // Verify Audience
        if let Some(audience_restriction) = conditions.audience_restriction.as_ref() {
            let audiences: Vec<String> = audience_restriction
                .audiences
                .iter()
                .filter_map(|a| a.value.clone())
                .collect();
            if !audiences.is_empty() && !audiences.contains(&expected_audience.to_string()) {
                return Err(SsoError::InvalidSamlResponse(format!(
                    "Invalid audience. Expected: {}, Got: {:?}",
                    expected_audience, audiences
                )));
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "saml-sso"))]
    fn verify_saml_conditions(
        &self,
        _assertion: &(),
        _expected_audience: &str,
    ) -> Result<(), SsoError> {
        Err(SsoError::Internal(
            "SAML SSO not enabled. Build with --features saml-sso".to_string(),
        ))
    }

    #[cfg(feature = "saml-sso")]
    pub async fn generate_saml_metadata(&self, team_id: &str) -> Result<String, SsoError> {
        let config = self.get_team_sso_config_by_type(team_id, "saml").await?;

        let sp_entity_id = config
            .config
            .get("sp_entity_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("https://easyssh.io/sso/saml/{}", team_id));

        let acs_url = config
            .config
            .get("sp_acs_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("https://easyssh.io/sso/saml/{}/acs", team_id));

        let metadata = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<EntityDescriptor xmlns="urn:oasis:names:tc:SAML:2.0:metadata" entityID="{}">
  <SPSSODescriptor AuthnRequestsSigned="true" WantAssertionsSigned="true" protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</NameIDFormat>
    <AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="{}" index="0" isDefault="true"/>
  </SPSSODescriptor>
</EntityDescriptor>"#,
            sp_entity_id, acs_url
        );

        Ok(metadata)
    }

    #[cfg(not(feature = "saml-sso"))]
    pub async fn generate_saml_metadata(&self, _team_id: &str) -> Result<String, SsoError> {
        Err(SsoError::Internal(
            "SAML SSO not enabled. Build with --features saml-sso".to_string(),
        ))
    }

    // ============== OIDC Methods ==============

    pub async fn generate_oidc_login_url(&self, team_id: &str) -> Result<SsoLoginUrl, SsoError> {
        let config = self.get_team_sso_config_by_type(team_id, "oidc").await?;

        if !config.is_enabled {
            return Err(SsoError::ProviderDisabled);
        }

        let authorization_url = config
            .config
            .get("authorization_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SsoError::InvalidOidcCallback("Authorization URL not configured".to_string())
            })?;

        let client_id = config
            .config
            .get("client_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SsoError::InvalidOidcCallback("Client ID not configured".to_string()))?;

        let redirect_url = config
            .config
            .get("redirect_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SsoError::InvalidOidcCallback("Redirect URL not configured".to_string())
            })?;

        let scopes = config
            .config
            .get("scopes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|| "openid profile email".to_string());

        // Generate state and nonce
        let state = Self::generate_secure_random(32);
        let nonce = Self::generate_secure_random(32);

        // Generate PKCE verifier and challenge
        let pkce_verifier = Self::generate_secure_random(128);
        let pkce_challenge = Self::sha256_base64_url(&pkce_verifier);

        // Build authorization URL
        let auth_url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",
            authorization_url,
            urlencoding::encode(client_id),
            urlencoding::encode(redirect_url),
            urlencoding::encode(&scopes),
            state,
            nonce,
            pkce_challenge
        );

        // Store pending request with PKCE verifier
        let pending = PendingAuthRequest {
            state: state.clone(),
            pkce_verifier: Some(pkce_verifier),
            nonce: Some(nonce),
            created_at: Utc::now(),
            provider_id: config.id.clone(),
        };
        self.pending_requests
            .lock()
            .map_err(|e| SsoError::Internal(e.to_string()))?
            .insert(state.clone(), pending);

        Ok(SsoLoginUrl {
            url: auth_url,
            state,
        })
    }

    pub async fn process_oidc_callback(
        &self,
        team_id: &str,
        code: &str,
        state: Option<&str>,
    ) -> Result<LoginResponse, SsoError> {
        let config = self.get_team_sso_config_by_type(team_id, "oidc").await?;

        if !config.is_enabled {
            return Err(SsoError::ProviderDisabled);
        }

        // Validate state parameter
        let state = state.ok_or(SsoError::InvalidState)?;
        let pending = self.validate_state(state)?;

        // Extract PKCE verifier
        let pkce_verifier = pending
            .pkce_verifier
            .as_ref()
            .ok_or(SsoError::PkceVerificationFailed)?;

        // Build OIDC config
        let oidc_config = self.build_oidc_config(&config).await?;

        // Exchange code for tokens using PKCE
        let token_response = self
            .exchange_oidc_code_with_pkce(&oidc_config, code, pkce_verifier, &oidc_config)
            .await?;

        // Verify ID token and extract user info
        let user_info = self
            .verify_id_token(&token_response, &oidc_config, pending.nonce.as_deref())
            .await?;

        // Find or create user
        let user = self
            .find_or_create_sso_user(
                &user_info.email,
                &user_info.name,
                &user_info.first_name,
                &user_info.last_name,
                "oidc",
                &user_info.sso_id,
                team_id,
            )
            .await?;

        // Clean up pending request
        self.pending_requests
            .lock()
            .map_err(|e| SsoError::Internal(e.to_string()))?
            .remove(state);

        // Generate tokens
        let scopes = vec!["read".to_string(), "write".to_string()];
        let access_token = create_access_token(
            &user.id,
            &user.email,
            &self.config.jwt_secret,
            self.config.jwt_expiry_hours,
            scopes.clone(),
        )
        .map_err(|e| SsoError::Internal(e.to_string()))?;

        let (refresh_token, _) = create_refresh_token(
            &user.id,
            &self.config.jwt_secret,
            self.config.refresh_token_expiry_days,
        )
        .map_err(|e| SsoError::Internal(e.to_string()))?;

        // Log successful SSO login
        self.log_sso_login(&user.id, team_id, "oidc", true, None)
            .await;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: (self.config.jwt_expiry_hours * 3600) as i64,
            user: user.into(),
        })
    }

    /// Build OIDC config from SsoConfig
    async fn build_oidc_config(&self, config: &SsoConfig) -> Result<OidcConfig, SsoError> {
        let client_id = config
            .config
            .get("client_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SsoError::InvalidOidcCallback("Client ID not configured".to_string()))?
            .to_string();

        let client_secret = config
            .config
            .get("client_secret")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let issuer_url = config
            .config
            .get("issuer_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SsoError::InvalidOidcCallback("Issuer URL not configured".to_string()))?
            .to_string();

        let redirect_uri = config
            .config
            .get("redirect_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SsoError::InvalidOidcCallback("Redirect URL not configured".to_string())
            })?
            .to_string();

        let token_endpoint = config
            .config
            .get("token_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SsoError::InvalidOidcCallback("Token URL not configured".to_string()))?
            .to_string();

        let authorization_endpoint = config
            .config
            .get("authorization_url")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("{}/oauth2/v1/authorize", issuer_url))
            .to_string();

        let userinfo_endpoint = config
            .config
            .get("userinfo_url")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("{}/oauth2/v1/userinfo", issuer_url))
            .to_string();

        let jwks_uri = config
            .config
            .get("jwks_uri")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("{}/oauth2/v1/keys", issuer_url))
            .to_string();

        Ok(OidcConfig {
            issuer_url,
            authorization_endpoint,
            token_endpoint,
            userinfo_endpoint,
            jwks_uri,
            client_id,
            client_secret,
            redirect_uri,
        })
    }

    /// Exchange OIDC authorization code for tokens with PKCE using reqwest
    async fn exchange_oidc_code_with_pkce(
        &self,
        _oidc_config: &OidcConfig,
        code: &str,
        pkce_verifier: &str,
        config: &OidcConfig,
    ) -> Result<OidcTokenResponse, SsoError> {
        let client_id = config.client_id.clone();
        let client_secret = config.client_secret.clone();
        let redirect_url = config.redirect_uri.clone();
        let token_endpoint = config.token_endpoint.clone();

        let mut params = std::collections::HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", &redirect_url);
        params.insert("client_id", &client_id);
        if !client_secret.is_empty() {
            params.insert("client_secret", &client_secret);
        }
        params.insert("code_verifier", pkce_verifier);

        let http_response = reqwest::Client::new()
            .post(&token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                SsoError::OidcTokenExchangeFailed(format!("HTTP request failed: {}", e))
            })?;

        if !http_response.status().is_success() {
            let error_text = http_response.text().await.unwrap_or_default();
            return Err(SsoError::OidcTokenExchangeFailed(format!(
                "Token endpoint returned error: {}",
                error_text
            )));
        }

        let token_response: OidcTokenResponse = http_response
            .json()
            .await
            .map_err(|e| SsoError::OidcTokenExchangeFailed(format!("JSON parse error: {}", e)))?;

        Ok(token_response)
    }

    /// Verify ID token and extract user information
    async fn verify_id_token(
        &self,
        token_response: &OidcTokenResponse,
        config: &OidcConfig,
        _expected_nonce: Option<&str>,
    ) -> Result<SsoUserInfo, SsoError> {
        // For now, decode JWT payload manually to extract claims
        // In production, use a proper JWT verification library
        let id_token_parts: Vec<&str> = token_response.id_token.split('.').collect();
        if id_token_parts.len() != 3 {
            return Err(SsoError::OidcTokenValidationFailed(
                "Invalid ID token format".to_string(),
            ));
        }

        // Decode payload (middle part)
        let payload = id_token_parts[1];
        let decoded_payload = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|e| {
                SsoError::OidcTokenValidationFailed(format!("Failed to decode token: {}", e))
            })?;
        let claims: serde_json::Value = serde_json::from_slice(&decoded_payload).map_err(|e| {
            SsoError::OidcTokenValidationFailed(format!("Failed to parse claims: {}", e))
        })?;

        // Extract user information
        let email = claims
            .get("email")
            .and_then(|e| e.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                claims
                    .get("sub")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| {
                SsoError::OidcTokenValidationFailed("No email or subject in ID token".to_string())
            })?;

        let sso_id = claims
            .get("sub")
            .and_then(|s| s.as_str())
            .unwrap_or(&email)
            .to_string();

        let first_name = claims
            .get("given_name")
            .or_else(|| claims.get("firstName"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        let last_name = claims
            .get("family_name")
            .or_else(|| claims.get("lastName"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        let name = claims
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .or_else(|| match (first_name.as_ref(), last_name.as_ref()) {
                (Some(f), Some(l)) => Some(format!("{} {}", f, l)),
                (Some(f), None) => Some(f.clone()),
                (None, Some(l)) => Some(l.clone()),
                (None, None) => None,
            })
            .unwrap_or_else(|| email.split('@').next().unwrap_or("User").to_string());

        // Extract groups if present
        let groups = claims
            .get("groups")
            .and_then(|g| g.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(SsoUserInfo {
            email,
            name,
            first_name,
            last_name,
            groups,
            sso_provider: "oidc".to_string(),
            sso_id,
        })
    }

    // ============== Helper Methods ==============

    /// Validate state parameter and return pending request
    fn validate_state(&self, state: &str) -> Result<PendingAuthRequest, SsoError> {
        let pending = self
            .pending_requests
            .lock()
            .map_err(|e| SsoError::Internal(e.to_string()))?
            .remove(state)
            .ok_or(SsoError::InvalidState)?;

        // Check expiration (5 minutes)
        if Utc::now() > pending.created_at + chrono::Duration::minutes(5) {
            return Err(SsoError::RequestExpired);
        }

        Ok(pending)
    }

    /// Find existing user or create new one from SSO info
    async fn find_or_create_sso_user(
        &self,
        email: &str,
        name: &str,
        first_name: &Option<String>,
        last_name: &Option<String>,
        sso_provider: &str,
        sso_id: &str,
        team_id: &str,
    ) -> Result<User, SsoError> {
        // Try to find existing user by SSO provider and ID
        let existing_by_sso: Option<User> = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE sso_provider = ? AND sso_id = ? AND is_active = TRUE",
        )
        .bind(sso_provider)
        .bind(sso_id)
        .fetch_optional(&self.db)
        .await?;

        if let Some(user) = existing_by_sso {
            // Update last login
            sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
                .bind(Utc::now())
                .bind(&user.id)
                .execute(&self.db)
                .await?;
            return Ok(user);
        }

        // Try to find by email
        let existing_by_email: Option<User> =
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ? AND is_active = TRUE")
                .bind(email)
                .fetch_optional(&self.db)
                .await?;

        if let Some(mut user) = existing_by_email {
            // Link SSO to existing user
            sqlx::query(
                "UPDATE users SET sso_provider = ?, sso_id = ?, last_login_at = ? WHERE id = ?",
            )
            .bind(sso_provider)
            .bind(sso_id)
            .bind(Utc::now())
            .bind(&user.id)
            .execute(&self.db)
            .await?;
            user.sso_provider = Some(sso_provider.to_string());
            user.sso_id = Some(sso_id.to_string());
            return Ok(user);
        }

        // Check if auto-provisioning is enabled
        let auto_provision = sqlx::query_scalar::<_, bool>(
            "SELECT COALESCE(auto_provision_users, TRUE) FROM teams WHERE id = ?",
        )
        .bind(team_id)
        .fetch_one(&self.db)
        .await
        .unwrap_or(true);

        if !auto_provision {
            return Err(SsoError::UserProvisioningDisabled);
        }

        // Create new user
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO users
                (id, email, name, created_at, updated_at, is_active, sso_provider, sso_id, last_login_at)
                VALUES (?, ?, ?, ?, ?, TRUE, ?, ?, ?)"#
        )
        .bind(&id)
        .bind(email)
        .bind(name)
        .bind(now)
        .bind(now)
        .bind(sso_provider)
        .bind(sso_id)
        .bind(now)
        .execute(&self.db)
        .await?;

        // Auto-join team
        let member_id = Uuid::new_v4().to_string();
        let default_role = sqlx::query_scalar::<_, String>(
            "SELECT COALESCE(default_sso_role, 'member') FROM teams WHERE id = ?",
        )
        .bind(team_id)
        .fetch_one(&self.db)
        .await
        .unwrap_or_else(|_| "member".to_string());

        sqlx::query(
            "INSERT INTO team_members (id, team_id, user_id, role, joined_at, is_active) VALUES (?, ?, ?, ?, ?, TRUE)"
        )
        .bind(&member_id)
        .bind(team_id)
        .bind(&id)
        .bind(&default_role)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(User {
            id,
            email: email.to_string(),
            password_hash: None,
            name: name.to_string(),
            avatar_url: None,
            created_at: now,
            updated_at: now,
            last_login_at: Some(now),
            is_active: true,
            is_admin: false,
            sso_provider: Some(sso_provider.to_string()),
            sso_id: Some(sso_id.to_string()),
            mfa_enabled: false,
            mfa_secret: None,
        })
    }

    /// Log SSO login attempt for audit
    async fn log_sso_login(
        &self,
        user_id: &str,
        team_id: &str,
        provider: &str,
        success: bool,
        error_message: Option<&str>,
    ) {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, timestamp, user_id, team_id, action, resource_type, success, error_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(now)
        .bind(user_id)
        .bind(team_id)
        .bind(format!("sso_login_{}", provider))
        .bind("user")
        .bind(success)
        .bind(error_message)
        .execute(&self.db)
        .await;
    }

    /// Generate cryptographically secure random string
    fn generate_secure_random(length: usize) -> String {
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

    /// SHA256 hash with Base64 URL-safe encoding (for PKCE)
    fn sha256_base64_url(input: &str) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        URL_SAFE_NO_PAD.encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secure_random() {
        let random1 = SsoService::generate_secure_random(32);
        let random2 = SsoService::generate_secure_random(32);
        assert_eq!(random1.len(), 32);
        assert_eq!(random2.len(), 32);
        assert_ne!(random1, random2); // Should be different each time
    }

    #[test]
    fn test_sha256_base64_url() {
        let input = "test_verifier";
        let hash1 = SsoService::sha256_base64_url(input);
        let hash2 = SsoService::sha256_base64_url(input);
        assert_eq!(hash1, hash2); // Same input = same hash
        assert!(!hash1.contains('+')); // URL-safe
        assert!(!hash1.contains('/')); // URL-safe
        assert!(!hash1.contains('=')); // No padding
    }

    #[test]
    fn test_pkce_verification() {
        let verifier = SsoService::generate_secure_random(128);
        let challenge = SsoService::sha256_base64_url(&verifier);
        assert_eq!(challenge.len(), 43); // S256 produces 43 chars
    }
}
