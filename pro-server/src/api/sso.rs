use axum::{
    extract::{Extension, Path, Query, State},
    response::Redirect,
    routing::{get, post},
    Json, Router,
};
use std::collections::HashMap;

use crate::{auth::Claims, models::*, services::sso_service::SsoService, AppState};

pub fn sso_routes() -> Router<AppState> {
    Router::new()
        // SSO Configuration (team admins)
        .route("/config", post(create_sso_config))
        .route("/config", get(list_sso_configs))
        .route("/config/:id", get(get_sso_config))
        .route("/config/:id", post(update_sso_config))
        .route("/config/:id", delete(delete_sso_config))
        // SAML endpoints
        .route("/saml/:team_id/login", get(saml_login))
        .route("/saml/:team_id/acs", post(saml_acs))
        .route("/saml/:team_id/metadata", get(saml_metadata))
        // OIDC endpoints
        .route("/oidc/:team_id/login", get(oidc_login))
        .route("/oidc/:team_id/callback", get(oidc_callback))
}

async fn create_sso_config(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<SsoConfig>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    let team_id = req.get("team_id").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "team_id is required".to_string(),
            code: Some("missing_team_id".to_string()),
            details: None,
        }),
    ))?;

    // Check if user is team admin
    let is_admin = check_team_admin(&state, &claims.sub, team_id).await?;
    if !is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only team admins can configure SSO".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let provider_type_str = req.get("provider_type").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "provider_type is required".to_string(),
            code: Some("missing_provider_type".to_string()),
            details: None,
        }),
    ))?;

    let provider_type = match provider_type_str {
        "saml" => SsoProviderType::Saml,
        "oidc" => SsoProviderType::Oidc,
        _ => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_provider".to_string(),
                    message: format!("Invalid provider_type: {}", provider_type_str),
                    code: Some("invalid_value".to_string()),
                    details: None,
                }),
            ))
        }
    };

    let provider_name = req.get("provider_name").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "provider_name is required".to_string(),
            code: Some("missing_provider_name".to_string()),
            details: None,
        }),
    ))?;

    let config = req.get("config").cloned().ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "config is required".to_string(),
            code: Some("missing_config".to_string()),
            details: None,
        }),
    ))?;

    let sso_config = sso_service
        .create_sso_config(team_id, provider_type, provider_name, config)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "sso_config_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: sso_config,
        message: Some("SSO configuration created".to_string()),
    }))
}

async fn list_sso_configs(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SuccessResponse<Vec<SsoConfig>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    let team_id = params.get("team_id").ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_param".to_string(),
            message: "team_id query parameter is required".to_string(),
            code: Some("missing_team_id".to_string()),
            details: None,
        }),
    ))?;

    // Check if user is team admin
    let is_admin = check_team_admin(&state, &claims.sub, team_id).await?;
    if !is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only team admins can view SSO configurations".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let configs = sso_service
        .list_team_sso_configs(team_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "list_configs_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: configs,
        message: None,
    }))
}

async fn get_sso_config(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<SsoConfig>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    let config = sso_service.get_sso_config(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "config_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Check if user is team admin
    let is_admin = check_team_admin(&state, &claims.sub, &config.team_id).await?;
    if !is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only team admins can view SSO configurations".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    Ok(Json(SuccessResponse {
        success: true,
        data: config,
        message: None,
    }))
}

async fn update_sso_config(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<SsoConfig>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    // Get existing config
    let existing = sso_service.get_sso_config(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "config_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Check if user is team admin
    let is_admin = check_team_admin(&state, &claims.sub, &existing.team_id).await?;
    if !is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only team admins can update SSO configurations".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let updates = req.get("config").cloned();
    let is_enabled = req.get("is_enabled").and_then(|v| v.as_bool());

    let config = sso_service
        .update_sso_config(&id, updates, is_enabled)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "update_config_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: config,
        message: Some("SSO configuration updated".to_string()),
    }))
}

async fn delete_sso_config(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    // Get existing config
    let existing = sso_service.get_sso_config(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "config_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Check if user is team admin
    let is_admin = check_team_admin(&state, &claims.sub, &existing.team_id).await?;
    if !is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only team admins can delete SSO configurations".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    sso_service.delete_sso_config(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "delete_config_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("SSO configuration deleted".to_string()),
    }))
}

// SAML endpoints
async fn saml_login(
    State(state): State<AppState>,
    Path(team_id): Path<String>,
) -> Result<Redirect, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    let login_url = sso_service
        .generate_saml_login_url(&team_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "saml_login_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Redirect::temporary(&login_url.url))
}

async fn saml_acs(
    State(state): State<AppState>,
    Path(team_id): Path<String>,
    body: axum::extract::Form<HashMap<String, String>>,
) -> Result<Json<SuccessResponse<LoginResponse>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    let saml_response = body.get("SAMLResponse").ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_saml_response".to_string(),
            message: "SAMLResponse is required".to_string(),
            code: Some("missing_saml_response".to_string()),
            details: None,
        }),
    ))?;

    let relay_state = body.get("RelayState").cloned();

    let login_result = sso_service
        .process_saml_response(&team_id, saml_response, relay_state.as_deref())
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "saml_authentication_failed".to_string(),
                    message: e.to_string(),
                    code: Some("sso_auth_failed".to_string()),
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: login_result,
        message: Some("SSO authentication successful".to_string()),
    }))
}

async fn saml_metadata(
    State(state): State<AppState>,
    Path(team_id): Path<String>,
) -> Result<axum::response::Response, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    let metadata = sso_service
        .generate_saml_metadata(&team_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "metadata_generation_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    let response = axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", "application/xml")
        .body(metadata)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "response_build_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(response)
}

// OIDC endpoints
async fn oidc_login(
    State(state): State<AppState>,
    Path(team_id): Path<String>,
) -> Result<Redirect, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    let login_url = sso_service
        .generate_oidc_login_url(&team_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "oidc_login_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Redirect::temporary(&login_url.url))
}

async fn oidc_callback(
    State(state): State<AppState>,
    Path(team_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SuccessResponse<LoginResponse>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let sso_service = SsoService::new(state.db.pool().clone(), state.config.clone());

    let code = params.get("code").ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_code".to_string(),
            message: "Authorization code is required".to_string(),
            code: Some("missing_code".to_string()),
            details: None,
        }),
    ))?;

    let state_param = params.get("state").cloned();

    let login_result = sso_service
        .process_oidc_callback(&team_id, code, state_param.as_deref())
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "oidc_authentication_failed".to_string(),
                    message: e.to_string(),
                    code: Some("sso_auth_failed".to_string()),
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: login_result,
        message: Some("SSO authentication successful".to_string()),
    }))
}

// Helper functions
async fn check_team_admin(
    state: &AppState,
    user_id: &str,
    team_id: &str,
) -> Result<bool, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let result = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM team_members
            WHERE team_id = ? AND user_id = ? AND is_active = TRUE
            AND (role = 'owner' OR role = 'admin')
        )",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "admin_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(result)
}
