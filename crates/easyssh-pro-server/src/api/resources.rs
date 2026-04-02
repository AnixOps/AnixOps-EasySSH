use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use std::collections::HashMap;

use crate::{auth::Claims, models::*, services::resource_service::ResourceService, AppState};

pub fn resource_routes() -> Router<AppState> {
    Router::new()
        // Shared servers
        .route("/servers", post(share_server))
        .route("/servers", get(list_shared_servers))
        .route("/servers/:id", get(get_shared_server))
        .route("/servers/:id", delete(unshare_server))
        .route("/servers/:id/permissions", put(update_server_permissions))
        // Snippets
        .route("/snippets", post(create_snippet))
        .route("/snippets", get(list_snippets))
        .route("/snippets/:id", get(get_snippet))
        .route("/snippets/:id", put(update_snippet))
        .route("/snippets/:id", delete(delete_snippet))
        .route("/snippets/:id/share", post(share_snippet))
        .route("/snippets/:id/unshare", post(unshare_snippet))
}

// Server sharing endpoints
async fn share_server(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<SharedServer>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    let server_id = req.get("server_id").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "server_id is required".to_string(),
            code: Some("missing_server_id".to_string()),
            details: None,
        }),
    ))?;

    let team_id = req.get("team_id").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "team_id is required".to_string(),
            code: Some("missing_team_id".to_string()),
            details: None,
        }),
    ))?;

    // Check if user has permission to share
    let can_share = check_team_permission(&state, &claims.sub, team_id, "share").await?;
    if !can_share {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to share servers with this team".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let permissions = req.get("permissions").cloned();

    let shared_server = resource_service
        .share_server(server_id, team_id, &claims.sub, permissions)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "share_server_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: shared_server,
        message: Some("Server shared successfully".to_string()),
    }))
}

async fn list_shared_servers(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SuccessResponse<Vec<SharedServer>>>, (axum::http::StatusCode, Json<ErrorResponse>)>
{
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    let team_id = params.get("team_id");

    // If team_id is specified, check membership
    if let Some(tid) = team_id {
        let is_member = check_team_membership(&state, &claims.sub, tid).await?;
        if !is_member {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "forbidden".to_string(),
                    message: "You are not a member of this team".to_string(),
                    code: Some("not_team_member".to_string()),
                    details: None,
                }),
            ));
        }
    }

    let servers = resource_service
        .list_shared_servers(team_id.map(|s| s.as_str()), &claims.sub)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "list_servers_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: servers,
        message: None,
    }))
}

async fn get_shared_server(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<SharedServer>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    let server = resource_service.get_shared_server(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "server_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Check if user has access
    let is_member = check_team_membership(&state, &claims.sub, &server.team_id).await?;
    if !is_member {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have access to this shared server".to_string(),
                code: Some("access_denied".to_string()),
                details: None,
            }),
        ));
    }

    Ok(Json(SuccessResponse {
        success: true,
        data: server,
        message: None,
    }))
}

async fn unshare_server(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    // Get server to check ownership
    let server = resource_service.get_shared_server(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "server_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Only the user who shared it or team admin can unshare
    let is_owner = server.shared_by == claims.sub;
    let can_manage = check_team_permission(&state, &claims.sub, &server.team_id, "manage").await?;

    if !is_owner && !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to unshare this server".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    resource_service.unshare_server(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "unshare_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Server unshared successfully".to_string()),
    }))
}

async fn update_server_permissions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(req): Json<ServerPermissions>,
) -> Result<Json<SuccessResponse<SharedServer>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    // Get server to check ownership
    let server = resource_service.get_shared_server(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "server_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    let is_owner = server.shared_by == claims.sub;
    let can_manage = check_team_permission(&state, &claims.sub, &server.team_id, "manage").await?;

    if !is_owner && !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to update server permissions".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let permissions = serde_json::to_value(req).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_permissions".to_string(),
                message: e.to_string(),
                code: Some("serialization_error".to_string()),
                details: None,
            }),
        )
    })?;

    let updated = resource_service
        .update_server_permissions(&id, permissions)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "update_permissions_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: updated,
        message: Some("Server permissions updated".to_string()),
    }))
}

// Snippet endpoints
async fn create_snippet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateSnippetRequest>,
) -> Result<Json<SuccessResponse<Snippet>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    let snippet = resource_service
        .create_snippet(&req, &claims.sub)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "create_snippet_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: snippet,
        message: Some("Snippet created successfully".to_string()),
    }))
}

async fn list_snippets(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SuccessResponse<Vec<Snippet>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    let team_id = params.get("team_id").map(|s| s.to_string());

    // If team_id is specified, check membership
    if let Some(ref tid) = team_id {
        let is_member = check_team_membership(&state, &claims.sub, tid).await?;
        if !is_member {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "forbidden".to_string(),
                    message: "You are not a member of this team".to_string(),
                    code: Some("not_team_member".to_string()),
                    details: None,
                }),
            ));
        }
    }

    let snippets = resource_service
        .list_snippets(team_id.as_deref(), &claims.sub)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "list_snippets_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: snippets,
        message: None,
    }))
}

async fn get_snippet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<Snippet>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    let snippet = resource_service.get_snippet(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "snippet_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Check access
    if snippet.created_by != claims.sub {
        let is_member = check_team_membership(&state, &claims.sub, &snippet.team_id).await?;
        if !is_member {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "forbidden".to_string(),
                    message: "You don't have access to this snippet".to_string(),
                    code: Some("access_denied".to_string()),
                    details: None,
                }),
            ));
        }
    }

    Ok(Json(SuccessResponse {
        success: true,
        data: snippet,
        message: None,
    }))
}

async fn update_snippet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSnippetRequest>,
) -> Result<Json<SuccessResponse<Snippet>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    // Get snippet to check ownership
    let snippet = resource_service.get_snippet(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "snippet_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Check permissions
    let can_edit = snippet.created_by == claims.sub
        || check_team_permission(&state, &claims.sub, &snippet.team_id, "manage").await?;

    if !can_edit {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to update this snippet".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let updated = resource_service
        .update_snippet(&id, req)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "update_snippet_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: updated,
        message: Some("Snippet updated successfully".to_string()),
    }))
}

async fn delete_snippet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    // Get snippet to check ownership
    let snippet = resource_service.get_snippet(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "snippet_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Check permissions
    let can_delete = snippet.created_by == claims.sub
        || check_team_permission(&state, &claims.sub, &snippet.team_id, "manage").await?;

    if !can_delete {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to delete this snippet".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    resource_service.delete_snippet(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "delete_snippet_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Snippet deleted successfully".to_string()),
    }))
}

async fn share_snippet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<Snippet>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    // Get snippet to check ownership
    let snippet = resource_service.get_snippet(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "snippet_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Check permissions
    let can_share = snippet.created_by == claims.sub
        || check_team_permission(&state, &claims.sub, &snippet.team_id, "share").await?;

    if !can_share {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to share this snippet".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let updated = resource_service
        .set_snippet_public(&id, true)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "share_snippet_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: updated,
        message: Some("Snippet is now public".to_string()),
    }))
}

async fn unshare_snippet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<Snippet>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let resource_service = ResourceService::new(state.db.pool().clone(), state.redis.clone());

    // Get snippet to check ownership
    let snippet = resource_service.get_snippet(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "snippet_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    // Check permissions
    let can_unshare = snippet.created_by == claims.sub
        || check_team_permission(&state, &claims.sub, &snippet.team_id, "manage").await?;

    if !can_unshare {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to unshare this snippet".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let updated = resource_service
        .set_snippet_public(&id, false)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "unshare_snippet_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: updated,
        message: Some("Snippet is now private".to_string()),
    }))
}

// Helper functions
async fn check_team_membership(
    state: &AppState,
    user_id: &str,
    team_id: &str,
) -> Result<bool, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let result = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = ? AND user_id = ? AND is_active = TRUE)"
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "membership_check_failed".to_string(),
            message: e.to_string(),
            code: None,
            details: None,
        })
    ))?;

    Ok(result)
}

async fn check_team_permission(
    state: &AppState,
    user_id: &str,
    team_id: &str,
    _permission: &str,
) -> Result<bool, (axum::http::StatusCode, Json<ErrorResponse>)> {
    // Simplified: check if user is owner or admin
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
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(result)
}
