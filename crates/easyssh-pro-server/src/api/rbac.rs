use axum::{
    extract::{Extension, Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};

use crate::{auth::Claims, models::*, services::rbac_service::RbacService, AppState};

pub fn rbac_routes() -> Router<AppState> {
    Router::new()
        // Role management
        .route("/roles", get(list_roles))
        .route("/roles", post(create_role))
        .route("/roles/{id}", get(get_role))
        .route("/roles/{id}", put(update_role))
        .route("/roles/{id}", delete(delete_role))
        // Permission management
        .route("/permissions", get(list_permissions))
        .route("/roles/{id}/permissions", get(get_role_permissions))
        .route("/roles/{id}/permissions", post(add_permission_to_role))
        .route(
            "/roles/{id}/permissions/{permission_id}",
            delete(remove_permission_from_role),
        )
        // Permission checking
        .route("/check", post(check_permission))
        .route("/user/permissions", get(get_user_permissions))
        // Team-specific roles
        .route("/team/{team_id}/roles", get(list_team_roles))
        .route("/team/{team_id}/roles", post(create_team_role))
        .route("/team/{team_id}/assign", post(assign_role_to_member))
        .route("/team/{team_id}/revoke", post(revoke_role_from_member))
}

async fn list_roles(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<PaginationParams>,
) -> Result<
    Json<SuccessResponse<PaginatedResponse<Role>>>,
    (axum::http::StatusCode, Json<ErrorResponse>),
> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    let (roles, total) = rbac_service
        .list_system_roles(params.page, params.limit)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "list_roles_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let total_pages = (total + limit - 1) / limit;

    Ok(Json(SuccessResponse {
        success: true,
        data: PaginatedResponse {
            data: roles,
            pagination: PaginationInfo {
                page,
                limit,
                total,
                total_pages,
                has_next: page < total_pages,
                has_prev: page > 1,
            },
        },
        message: None,
    }))
}

async fn create_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<Role>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    // Only admins can create system roles
    if !claims.is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only admins can create system roles".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let name = req.get("name").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "Role name is required".to_string(),
            code: Some("missing_name".to_string()),
            details: None,
        }),
    ))?;

    let description = req.get("description").and_then(|v| v.as_str());

    let role = rbac_service
        .create_system_role(name, description)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "create_role_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: role,
        message: Some("Role created successfully".to_string()),
    }))
}

async fn get_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<Role>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    let role = rbac_service.get_role(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "role_not_found".to_string(),
                message: e.to_string(),
                code: Some("not_found".to_string()),
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: role,
        message: None,
    }))
}

async fn update_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<Role>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    if !claims.is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only admins can update roles".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let name = req.get("name").and_then(|v| v.as_str());
    let description = req.get("description").and_then(|v| v.as_str());

    let role = rbac_service
        .update_role(&id, name, description)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "update_role_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: role,
        message: Some("Role updated successfully".to_string()),
    }))
}

async fn delete_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    if !claims.is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only admins can delete roles".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    rbac_service.delete_role(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "delete_role_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Role deleted successfully".to_string()),
    }))
}

async fn list_permissions(
    State(state): State<AppState>,
    _claims: Extension<Claims>,
) -> Result<Json<SuccessResponse<Vec<Permission>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    let permissions = rbac_service.list_permissions().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "list_permissions_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: permissions,
        message: None,
    }))
}

async fn get_role_permissions(
    State(state): State<AppState>,
    _claims: Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<Vec<Permission>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    let permissions = rbac_service.get_role_permissions(&id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "get_permissions_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: permissions,
        message: None,
    }))
}

async fn add_permission_to_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    if !claims.is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only admins can modify role permissions".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let permission_id = req.get("permission_id").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "permission_id is required".to_string(),
            code: Some("missing_permission_id".to_string()),
            details: None,
        }),
    ))?;

    rbac_service
        .add_permission_to_role(&id, permission_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "add_permission_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Permission added to role".to_string()),
    }))
}

async fn remove_permission_from_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((role_id, permission_id)): Path<(String, String)>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    if !claims.is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only admins can modify role permissions".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    rbac_service
        .remove_permission_from_role(&role_id, &permission_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "remove_permission_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Permission removed from role".to_string()),
    }))
}

async fn check_permission(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CheckPermissionRequest>,
) -> Result<
    Json<SuccessResponse<CheckPermissionResponse>>,
    (axum::http::StatusCode, Json<ErrorResponse>),
> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    // Check global permissions or team-specific permissions
    let allowed = rbac_service
        .check_user_permission(
            &claims.sub,
            req.team_id.as_deref(),
            &req.resource_type,
            &req.action,
            req.resource_id.as_deref(),
        )
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

    Ok(Json(SuccessResponse {
        success: true,
        data: CheckPermissionResponse {
            allowed,
            reason: if allowed {
                None
            } else {
                Some("Insufficient permissions".to_string())
            },
        },
        message: None,
    }))
}

async fn get_user_permissions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<SuccessResponse<Vec<String>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    let team_id = params.get("team_id").map(|s| s.as_str());

    let permissions = rbac_service
        .get_user_permissions(&claims.sub, team_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "get_permissions_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: permissions,
        message: None,
    }))
}

async fn list_team_roles(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
) -> Result<Json<SuccessResponse<Vec<Role>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    // Check if user is team member
    let is_member = check_team_membership(&state, &claims.sub, &team_id).await?;
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

    let roles = rbac_service.list_team_roles(&team_id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "list_roles_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: roles,
        message: None,
    }))
}

async fn create_team_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<Role>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    // Check if user has manage permission
    let can_manage = check_team_permission(&state, &claims.sub, &team_id, "manage").await?;
    if !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to create team roles".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let name = req.get("name").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "Role name is required".to_string(),
            code: Some("missing_name".to_string()),
            details: None,
        }),
    ))?;

    let description = req.get("description").and_then(|v| v.as_str());

    let role = rbac_service
        .create_team_role(&team_id, name, description)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "create_role_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: role,
        message: Some("Team role created successfully".to_string()),
    }))
}

async fn assign_role_to_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    let can_manage = check_team_permission(&state, &claims.sub, &team_id, "manage").await?;
    if !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to assign roles".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let user_id = req.get("user_id").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "user_id is required".to_string(),
            code: Some("missing_user_id".to_string()),
            details: None,
        }),
    ))?;

    let role_id = req.get("role_id").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "role_id is required".to_string(),
            code: Some("missing_role_id".to_string()),
            details: None,
        }),
    ))?;

    rbac_service
        .assign_role_to_member(&team_id, user_id, role_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "assign_role_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Role assigned to member".to_string()),
    }))
}

async fn revoke_role_from_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let rbac_service = RbacService::new(state.db.pool().clone());

    let can_manage = check_team_permission(&state, &claims.sub, &team_id, "manage").await?;
    if !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to revoke roles".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            }),
        ));
    }

    let user_id = req.get("user_id").and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_field".to_string(),
            message: "user_id is required".to_string(),
            code: Some("missing_user_id".to_string()),
            details: None,
        }),
    ))?;

    let role_id = req.get("role_id").and_then(|v| v.as_str());

    rbac_service
        .revoke_role_from_member(&team_id, user_id, role_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "revoke_role_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Role revoked from member".to_string()),
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
    permission: &str,
) -> Result<bool, (axum::http::StatusCode, Json<ErrorResponse>)> {
    // For now, use a simplified check based on role
    // In a full implementation, this would use the RBAC service
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
