use axum::{
    extract::{Extension, Json, State},
    routing::{delete, get, post},
    Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{models::*, redis_cache::RedisCache, services::auth_service::AuthService, AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub email: String,
    pub exp: i64,    // Expiration timestamp
    pub iat: i64,    // Issued at timestamp
    pub jti: String, // JWT ID (for revocation)
    pub scopes: Vec<String>,
    pub is_admin: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: String,
    pub jti: String,
    pub exp: i64,
}

pub fn create_access_token(
    user_id: &str,
    email: &str,
    secret: &str,
    expiry_hours: u64,
    scopes: Vec<String>,
) -> anyhow::Result<String> {
    let now = Utc::now();
    let expiration = now + Duration::hours(expiry_hours as i64);
    let jti = Uuid::new_v4().to_string();

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp: expiration.timestamp(),
        iat: now.timestamp(),
        jti: jti.clone(),
        scopes,
        is_admin: false, // Default to false, should be fetched from user data
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn create_refresh_token(
    user_id: &str,
    secret: &str,
    expiry_days: u64,
) -> anyhow::Result<(String, String)> {
    let now = Utc::now();
    let expiration = now + Duration::days(expiry_days as i64);
    let jti = Uuid::new_v4().to_string();

    let claims = RefreshClaims {
        sub: user_id.to_string(),
        jti: jti.clone(),
        exp: expiration.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok((token, jti))
}

pub fn decode_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let valid = verify(password, hash)?;
    Ok(valid)
}

use sha2::{Digest, Sha256};

pub fn generate_api_key() -> (String, String, String) {
    let key = format!("esk_{}", Uuid::new_v4().to_string().replace("-", ""));
    let hash = format!("{:x}", Sha256::digest(&key));
    let prefix = key[..11].to_string(); // "esk_" + first 7 chars
    (key, hash, prefix)
}

pub fn hash_api_key(key: &str) -> String {
    format!("{:x}", Sha256::digest(key))
}

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/me", get(get_current_user))
        .route("/api-keys", post(create_api_key))
        .route("/api-keys", get(list_api_keys))
        .route("/api-keys/{id}", delete(revoke_api_key))
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<SuccessResponse<UserProfile>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let auth_service = AuthService::new(state.db.pool().clone(), state.redis.clone());

    let password_hash = hash_password(&req.password).map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "password_hash_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    let user = auth_service
        .create_user(&req.email, &password_hash, &req.name)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "registration_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: user.into(),
        message: Some("User registered successfully".to_string()),
    }))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let auth_service = AuthService::new(state.db.pool().clone(), state.redis.clone());

    let user = auth_service
        .authenticate_user(&req.email, &req.password)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "authentication_failed".to_string(),
                    message: e.to_string(),
                    code: Some("invalid_credentials".to_string()),
                    details: None,
                }),
            )
        })?;

    // MFA check
    if user.mfa_enabled {
        let code = req.mfa_code.ok_or((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "mfa_required".to_string(),
                message: "MFA code required".to_string(),
                code: Some("mfa_required".to_string()),
                details: None,
            }),
        ))?;

        // Verify MFA code (implement TOTP verification)
        // For now, placeholder
    }

    let scopes = vec!["read".to_string(), "write".to_string()];

    let access_token = create_access_token(
        &user.id,
        &user.email,
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
        scopes.clone(),
    )
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "token_creation_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    let (refresh_token, jti) = create_refresh_token(
        &user.id,
        &state.config.jwt_secret,
        state.config.refresh_token_expiry_days,
    )
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "token_creation_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    // Store session in database
    auth_service
        .create_session(
            &user.id,
            &hash_api_key(&access_token),
            &hash_api_key(&refresh_token),
            None,
            &jti,
        )
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "session_creation_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: (state.config.jwt_expiry_hours * 3600) as i64,
        user: user.into(),
    }))
}

async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<LoginResponse>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let refresh_token = req.get("refresh_token").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_token".to_string(),
            message: "Refresh token is required".to_string(),
            code: Some("missing_refresh_token".to_string()),
            details: None,
        }),
    ))?;

    let claims = decode::<RefreshClaims>(
        refresh_token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid_token".to_string(),
                message: e.to_string(),
                code: Some("token_decode_failed".to_string()),
                details: None,
            }),
        )
    })?
    .claims;

    // Check if token is revoked
    let jti_hash = hash_api_key(refresh_token);
    let is_revoked = state
        .redis
        .get(&format!("revoked:{}", jti_hash))
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "redis_error".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?
        .is_some();

    if is_revoked {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "token_revoked".to_string(),
                message: "Refresh token has been revoked".to_string(),
                code: Some("token_revoked".to_string()),
                details: None,
            }),
        ));
    }

    let auth_service = AuthService::new(state.db.pool().clone(), state.redis.clone());
    let user = auth_service
        .get_user_by_id(&claims.sub)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "user_not_found".to_string(),
                    message: e.to_string(),
                    code: Some("user_not_found".to_string()),
                    details: None,
                }),
            )
        })?;

    let scopes = vec!["read".to_string(), "write".to_string()];

    let new_access_token = create_access_token(
        &user.id,
        &user.email,
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
        scopes.clone(),
    )
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "token_creation_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    let (new_refresh_token, new_jti) = create_refresh_token(
        &user.id,
        &state.config.jwt_secret,
        state.config.refresh_token_expiry_days,
    )
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "token_creation_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    // Revoke old refresh token
    let ttl = std::time::Duration::from_secs(state.config.refresh_token_expiry_days * 86400);
    state
        .redis
        .set(&format!("revoked:{}", jti_hash), "1", ttl)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "redis_error".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(LoginResponse {
        access_token: new_access_token,
        refresh_token: new_refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: (state.config.jwt_expiry_hours * 3600) as i64,
        user: user.into(),
    }))
}

async fn logout(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    // Extract token from Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    if let Some(token) = auth_header {
        let jti_hash = hash_api_key(token);
        let ttl = std::time::Duration::from_secs(state.config.jwt_expiry_hours * 3600);
        state
            .redis
            .set(&format!("revoked:{}", jti_hash), "1", ttl)
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "redis_error".to_string(),
                        message: e.to_string(),
                        code: None,
                        details: None,
                    }),
                )
            })?;
    }

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Logged out successfully".to_string()),
    }))
}

async fn get_current_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<SuccessResponse<UserProfile>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let auth_service = AuthService::new(state.db.pool().clone(), state.redis.clone());

    let user = auth_service
        .get_user_by_id(&claims.sub)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "user_not_found".to_string(),
                    message: e.to_string(),
                    code: Some("user_not_found".to_string()),
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: user.into(),
        message: None,
    }))
}

async fn create_api_key(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<
    Json<SuccessResponse<CreateApiKeyResponse>>,
    (axum::http::StatusCode, Json<ErrorResponse>),
> {
    let auth_service = AuthService::new(state.db.pool().clone(), state.redis.clone());

    let (api_key, key_hash, key_prefix) = generate_api_key();

    let expires_at = req
        .expires_in_days
        .map(|days| Utc::now() + Duration::days(days as i64));

    let api_key_record = auth_service
        .create_api_key(
            &claims.sub,
            &req.name,
            &key_hash,
            &key_prefix,
            req.scopes
                .as_ref()
                .map(|s| serde_json::to_value(s).unwrap()),
            expires_at,
        )
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "api_key_creation_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    // Store in Redis for fast lookup
    let ttl = req
        .expires_in_days
        .map(|d| std::time::Duration::from_secs(d as u64 * 86400))
        .unwrap_or(std::time::Duration::from_secs(365 * 86400));
    state
        .redis
        .store_api_key(&key_hash, &claims.sub, Some(ttl))
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "redis_error".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: CreateApiKeyResponse {
            id: api_key_record.id,
            name: api_key_record.name,
            api_key, // Only returned once!
            key_prefix: api_key_record.key_prefix,
            scopes: req.scopes.unwrap_or_default(),
            created_at: api_key_record.created_at,
            expires_at: api_key_record.expires_at,
        },
        message: Some(
            "API key created successfully. Store it securely - it won't be shown again."
                .to_string(),
        ),
    }))
}

async fn list_api_keys(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<SuccessResponse<Vec<ApiKey>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let auth_service = AuthService::new(state.db.pool().clone(), state.redis.clone());

    let keys = auth_service.list_api_keys(&claims.sub).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "list_api_keys_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: keys,
        message: None,
    }))
}

async fn revoke_api_key(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let auth_service = AuthService::new(state.db.pool().clone(), state.redis.clone());

    // Get the API key first to revoke in Redis
    let key = auth_service.get_api_key(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "api_key_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Ensure user owns this key
    if key.user_id != claims.sub {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You can only revoke your own API keys".to_string(),
                code: Some("forbidden".to_string()),
                details: None,
            }),
        ));
    }

    // Revoke in Redis
    state
        .redis
        .revoke_api_key(&key.key_hash)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "redis_error".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    // Delete from database
    auth_service.delete_api_key(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "revoke_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("API key revoked successfully".to_string()),
    }))
}
