use anyhow::Result;
use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub refresh_token_expiry_days: u64,
    pub rate_limit_requests: u32,
    pub rate_limit_window_secs: u64,
    pub encryption_key: String,
    // SSO Configuration
    pub saml_issuer: Option<String>,
    pub saml_idp_sso_url: Option<String>,
    pub saml_idp_cert: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_authorization_url: Option<String>,
    pub oidc_token_url: Option<String>,
    pub oidc_userinfo_url: Option<String>,
    pub oidc_redirect_url: Option<String>,
    // Email Configuration
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .expect("Invalid PORT");

        let base_url = env::var("BASE_URL").unwrap_or_else(|| format!("http://{}:{}", host, port));

        Ok(Self {
            host,
            port,
            base_url,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./pro_server.db".to_string()),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string()),
            jwt_expiry_hours: env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .expect("Invalid JWT_EXPIRY_HOURS"),
            refresh_token_expiry_days: env::var("REFRESH_TOKEN_EXPIRY_DAYS")
                .unwrap_or_else(|_| "7".to_string())
                .parse()
                .expect("Invalid REFRESH_TOKEN_EXPIRY_DAYS"),
            rate_limit_requests: env::var("RATE_LIMIT_REQUESTS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .expect("Invalid RATE_LIMIT_REQUESTS"),
            rate_limit_window_secs: env::var("RATE_LIMIT_WINDOW_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .expect("Invalid RATE_LIMIT_WINDOW_SECS"),
            encryption_key: env::var("ENCRYPTION_KEY")
                .unwrap_or_else(|_| "your-32-byte-encryption-key-here!".to_string()),
            // SSO
            saml_issuer: env::var("SAML_ISSUER").ok(),
            saml_idp_sso_url: env::var("SAML_IDP_SSO_URL").ok(),
            saml_idp_cert: env::var("SAML_IDP_CERT").ok(),
            oidc_client_id: env::var("OIDC_CLIENT_ID").ok(),
            oidc_client_secret: env::var("OIDC_CLIENT_SECRET").ok(),
            oidc_authorization_url: env::var("OIDC_AUTHORIZATION_URL").ok(),
            oidc_token_url: env::var("OIDC_TOKEN_URL").ok(),
            oidc_userinfo_url: env::var("OIDC_USERINFO_URL").ok(),
            oidc_redirect_url: env::var("OIDC_REDIRECT_URL").ok(),
            // Email
            smtp_host: env::var("SMTP_HOST").ok(),
            smtp_port: env::var("SMTP_PORT").ok().and_then(|p| p.parse().ok()),
            smtp_username: env::var("SMTP_USERNAME").ok(),
            smtp_password: env::var("SMTP_PASSWORD").ok(),
            smtp_from: env::var("SMTP_FROM").ok(),
        })
    }
}
