use crate::{
    auth::{create_access_token, create_refresh_token, hash_api_key, Claims, LoginResponse},
    config::AppConfig,
    models::*,
    services::auth_service::AuthService,
};
use anyhow::Result;
use chrono::Utc;
use sqlx::AnyPool;
use uuid::Uuid;

pub struct SsoService {
    db: AnyPool,
    config: std::sync::Arc<AppConfig>,
}

impl SsoService {
    pub fn new(db: AnyPool, config: std::sync::Arc<AppConfig>) -> Self {
        Self { db, config }
    }

    pub async fn create_sso_config(
        &self,
        team_id: &str,
        provider_type: SsoProviderType,
        provider_name: &str,
        config: serde_json::Value,
    ) -> Result<SsoConfig> {
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

    pub async fn list_team_sso_configs(&self, team_id: &str) -> Result<Vec<SsoConfig>> {
        let configs = sqlx::query_as::<_, SsoConfig>(
            "SELECT * FROM sso_configs WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&self.db)
        .await?;

        Ok(configs)
    }

    pub async fn get_sso_config(&self, id: &str) -> Result<SsoConfig> {
        let config = sqlx::query_as::<_, SsoConfig>("SELECT * FROM sso_configs WHERE id = ?")
            .bind(id)
            .fetch_one(&self.db)
            .await?;

        Ok(config)
    }

    pub async fn update_sso_config(
        &self,
        id: &str,
        config_updates: Option<serde_json::Value>,
        is_enabled: Option<bool>,
    ) -> Result<SsoConfig> {
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

    pub async fn delete_sso_config(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM sso_configs WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    // SAML methods
    pub async fn generate_saml_login_url(&self, team_id: &str) -> Result<SsoLoginUrl> {
        let config = sqlx::query_as::<_, SsoConfig>(
            "SELECT * FROM sso_configs WHERE team_id = ? AND provider_type = 'saml' AND is_enabled = TRUE"
        )
        .bind(team_id)
        .fetch_one(&self.db)
        .await?;

        // In a real implementation, this would:
        // 1. Generate a SAML AuthnRequest
        // 2. Sign it
        // 3. Base64 encode and URL encode
        // 4. Construct the redirect URL

        let state = Uuid::new_v4().to_string();
        let sso_url = config
            .config
            .get("sso_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SSO URL not configured"))?;

        Ok(SsoLoginUrl {
            url: format!("{}?SAMLRequest=placeholder&RelayState={}", sso_url, state),
            state,
        })
    }

    pub async fn process_saml_response(
        &self,
        team_id: &str,
        saml_response: &str,
        _relay_state: Option<&str>,
    ) -> Result<LoginResponse> {
        // In a real implementation, this would:
        // 1. Decode the Base64 response
        // 2. Parse the XML
        // 3. Verify the signature
        // 4. Extract user attributes
        // 5. Match or create user

        let _config = sqlx::query_as::<_, SsoConfig>(
            "SELECT * FROM sso_configs WHERE team_id = ? AND provider_type = 'saml' AND is_enabled = TRUE"
        )
        .bind(team_id)
        .fetch_one(&self.db)
        .await?;

        // Mock user extraction from SAML response
        let email = "user@example.com"; // Would be extracted from SAML assertion
        let name = "SSO User"; // Would be extracted from SAML assertion

        // Find or create user
        let user = self
            .find_or_create_sso_user(email, name, "saml", team_id)
            .await?;

        // Generate tokens
        let scopes = vec!["read".to_string(), "write".to_string()];
        let access_token = create_access_token(
            &user.id,
            &user.email,
            &self.config.jwt_secret,
            self.config.jwt_expiry_hours,
            scopes.clone(),
        )?;

        let (refresh_token, _) = create_refresh_token(
            &user.id,
            &self.config.jwt_secret,
            self.config.refresh_token_expiry_days,
        )?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: (self.config.jwt_expiry_hours * 3600) as i64,
            user: user.into(),
        })
    }

    pub async fn generate_saml_metadata(&self, team_id: &str) -> Result<String> {
        // Generate SAML Service Provider metadata XML
        let entity_id = format!("https://easyssh.io/sso/saml/{}", team_id);
        let acs_url = format!("https://easyssh.io/sso/saml/{}/acs", team_id);

        let metadata = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="{}">
  <md:SPSSODescriptor AuthnRequestsSigned="false" WantAssertionsSigned="true" protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</md:NameIDFormat>
    <md:AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="{}" index="0" isDefault="true"/>
  </md:SPSSODescriptor>
</md:EntityDescriptor>"#,
            entity_id, acs_url
        );

        Ok(metadata)
    }

    // OIDC methods
    pub async fn generate_oidc_login_url(&self, team_id: &str) -> Result<SsoLoginUrl> {
        let config = sqlx::query_as::<_, SsoConfig>(
            "SELECT * FROM sso_configs WHERE team_id = ? AND provider_type = 'oidc' AND is_enabled = TRUE"
        )
        .bind(team_id)
        .fetch_one(&self.db)
        .await?;

        let state = Uuid::new_v4().to_string();
        let authorization_url = config
            .config
            .get("authorization_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Authorization URL not configured"))?;
        let client_id = config
            .config
            .get("client_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Client ID not configured"))?;
        let redirect_url = config
            .config
            .get("redirect_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Redirect URL not configured"))?;

        let scopes = config
            .config
            .get("scopes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("+")
            })
            .unwrap_or_else(|| "openid+profile+email".to_string());

        let url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&state={}",
            authorization_url,
            client_id,
            urlencoding::encode(redirect_url),
            scopes,
            state
        );

        Ok(SsoLoginUrl { url, state })
    }

    pub async fn process_oidc_callback(
        &self,
        team_id: &str,
        code: &str,
        _state: Option<&str>,
    ) -> Result<LoginResponse> {
        let config = sqlx::query_as::<_, SsoConfig>(
            "SELECT * FROM sso_configs WHERE team_id = ? AND provider_type = 'oidc' AND is_enabled = TRUE"
        )
        .bind(team_id)
        .fetch_one(&self.db)
        .await?;

        // In a real implementation:
        // 1. Exchange code for tokens
        // 2. Get user info from userinfo endpoint
        // 3. Find or create user

        let _token_url = config
            .config
            .get("token_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Token URL not configured"))?;
        let _client_id = config
            .config
            .get("client_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Client ID not configured"))?;
        let _client_secret = config
            .config
            .get("client_secret")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Client secret not configured"))?;

        // Mock user info
        let email = "user@example.com";
        let name = "OIDC User";

        let user = self
            .find_or_create_sso_user(email, name, "oidc", team_id)
            .await?;

        let scopes = vec!["read".to_string(), "write".to_string()];
        let access_token = create_access_token(
            &user.id,
            &user.email,
            &self.config.jwt_secret,
            self.config.jwt_expiry_hours,
            scopes.clone(),
        )?;

        let (refresh_token, _) = create_refresh_token(
            &user.id,
            &self.config.jwt_secret,
            self.config.refresh_token_expiry_days,
        )?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: (self.config.jwt_expiry_hours * 3600) as i64,
            user: user.into(),
        })
    }

    async fn find_or_create_sso_user(
        &self,
        email: &str,
        name: &str,
        sso_provider: &str,
        _team_id: &str,
    ) -> Result<User> {
        // Try to find existing user
        let existing: Option<User> =
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ? AND is_active = TRUE")
                .bind(email)
                .fetch_optional(&self.db)
                .await?;

        if let Some(mut user) = existing {
            // Update SSO info if not set
            if user.sso_provider.is_none() {
                sqlx::query("UPDATE users SET sso_provider = ?, sso_id = ? WHERE id = ?")
                    .bind(sso_provider)
                    .bind(email)
                    .bind(&user.id)
                    .execute(&self.db)
                    .await?;
                user.sso_provider = Some(sso_provider.to_string());
                user.sso_id = Some(email.to_string());
            }
            return Ok(user);
        }

        // Create new user
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO users (id, email, name, created_at, updated_at, is_active, sso_provider, sso_id) VALUES (?, ?, ?, ?, ?, TRUE, ?, ?)"
        )
        .bind(&id)
        .bind(email)
        .bind(name)
        .bind(now)
        .bind(now)
        .bind(sso_provider)
        .bind(email)
        .execute(&self.db)
        .await?;

        // Auto-join team if this is an SSO-initiated signup
        // (Would add team member record here)

        Ok(User {
            id,
            email: email.to_string(),
            password_hash: None,
            name: name.to_string(),
            avatar_url: None,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            is_active: true,
            is_admin: false,
            sso_provider: Some(sso_provider.to_string()),
            sso_id: Some(email.to_string()),
            mfa_enabled: false,
            mfa_secret: None,
        })
    }
}
