use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    auth::Claims,
    models::*,
    services::team_service::TeamService,
    AppState,
};

pub fn team_routes() -> Router<AppState> {
    Router::new()
        // Team CRUD
        .route("/", post(create_team))
        .route("/", get(list_teams))
        .route("/:id", get(get_team))
        .route("/:id", put(update_team))
        .route("/:id", delete(delete_team))
        // Members
        .route("/:id/members", get(list_team_members))
        .route("/:id/members", post(invite_member))
        .route("/:id/members/:member_id", delete(remove_member))
        .route("/:id/members/:member_id/role", put(update_member_role))
        // Invitations
        .route("/invitations/:token/accept", post(accept_invitation))
        .route("/invitations/:token/decline", post(decline_invitation))
        .route("/:id/invitations", get(list_invitations))
        .route("/:id/invitations/:invitation_id", delete(cancel_invitation))
        // Settings
        .route("/:id/settings", get(get_team_settings))
        .route("/:id/settings", put(update_team_settings))
}

async fn create_team(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateTeamRequest>,
) -> Result<Json<SuccessResponse<Team>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    let team = team_service
        .create_team(&claims.sub, &req.name, req.description.as_deref(), req.settings)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "team_creation_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: team,
        message: Some("Team created successfully".to_string()),
    }))
}

async fn list_teams(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<SuccessResponse<PaginatedResponse<Team>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    let (teams, total) = team_service
        .list_user_teams(&claims.sub, params.page, params.limit)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "list_teams_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let total_pages = (total + limit - 1) / limit;

    Ok(Json(SuccessResponse {
        success: true,
        data: PaginatedResponse {
            data: teams,
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

async fn get_team(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<Team>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    let team = team_service
        .get_team(&id)
        .await
        .map_err(|e| (
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "team_not_found".to_string(),
                message: e.to_string(),
                code: Some("team_not_found".to_string()),
                details: None,
            })
        ))?;

    // Check if user is a member
    let is_member = team_service
        .is_team_member(&id, &claims.sub)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !is_member {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You are not a member of this team".to_string(),
                code: Some("not_team_member".to_string()),
                details: None,
            })
        ));
    }

    Ok(Json(SuccessResponse {
        success: true,
        data: team,
        message: None,
    }))
}

async fn update_team(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTeamRequest>,
) -> Result<Json<SuccessResponse<Team>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    // Check if user has permission to update
    let can_manage = team_service
        .has_team_permission(&id, &claims.sub, "manage")
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to update this team".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            })
        ));
    }

    let team = team_service
        .update_team(&id, req.name.as_deref(), req.description.as_deref(), req.settings)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "team_update_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: team,
        message: Some("Team updated successfully".to_string()),
    }))
}

async fn delete_team(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    // Check if user is owner
    let is_owner = team_service
        .is_team_owner(&id, &claims.sub)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !is_owner {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Only team owner can delete the team".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            })
        ));
    }

    team_service
        .delete_team(&id)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "team_deletion_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Team deleted successfully".to_string()),
    }))
}

async fn list_team_members(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse<Vec<TeamMember>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    // Check if user is a member
    let is_member = team_service
        .is_team_member(&id, &claims.sub)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !is_member {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You are not a member of this team".to_string(),
                code: Some("not_team_member".to_string()),
                details: None,
            })
        ));
    }

    let members = team_service
        .list_team_members(&id)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "list_members_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: members,
        message: None,
    }))
}

async fn invite_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(req): Json<InviteMemberRequest>,
) -> Result<Json<SuccessResponse<Invitation>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    // Check if user has permission to invite
    let can_invite = team_service
        .has_team_permission(&id, &claims.sub, "invite")
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !can_invite {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to invite members".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            })
        ));
    }

    let invitation = team_service
        .invite_member(&id, &req.email, req.role, &claims.sub)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "invitation_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    // Send invitation email (async)
    if let (Some(smtp_host), Some(smtp_from)) = (state.config.smtp_host.as_ref(), state.config.smtp_from.as_ref()) {
        // Email sending logic would go here
        // For now, just log
        tracing::info!("Would send invitation email to {} for team {}", req.email, id);
    }

    Ok(Json(SuccessResponse {
        success: true,
        data: invitation,
        message: Some("Invitation sent successfully".to_string()),
    }))
}

async fn remove_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((team_id, member_id)): Path<(String, String)>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    // Check permissions
    let is_owner_or_admin = team_service
        .has_team_permission(&team_id, &claims.sub, "manage")
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    // Can also remove yourself
    let is_self = member_id == claims.sub;

    if !is_owner_or_admin && !is_self {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to remove this member".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            })
        ));
    }

    team_service
        .remove_member(&team_id, &member_id)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "remove_member_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Member removed successfully".to_string()),
    }))
}

async fn update_member_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((team_id, member_id)): Path<(String, String)>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<TeamMember>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    // Check permissions
    let can_manage = team_service
        .has_team_permission(&team_id, &claims.sub, "manage")
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to change member roles".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            })
        ));
    }

    let role_str = req.get("role")
        .and_then(|v| v.as_str())
        .ok_or((
            axum::http::StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing_role".to_string(),
                message: "Role is required".to_string(),
                code: Some("missing_field".to_string()),
                details: None,
            })
        ))?;

    let role = match role_str {
        "admin" => TeamRole::Admin,
        "member" => TeamRole::Member,
        "guest" => TeamRole::Guest,
        _ => return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_role".to_string(),
                message: format!("Invalid role: {}", role_str),
                code: Some("invalid_value".to_string()),
                details: None,
            })
        ))
    };

    let member = team_service
        .update_member_role(&team_id, &member_id, role)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "update_role_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: member,
        message: Some("Member role updated successfully".to_string()),
    }))
}

async fn accept_invitation(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(token): Path<String>,
) -> Result<Json<SuccessResponse<TeamMember>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    let member = team_service
        .accept_invitation(&token, &claims.sub)
        .await
        .map_err(|e| (
            axum::http::StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "accept_invitation_failed".to_string(),
                message: e.to_string(),
                code: Some("invitation_error".to_string()),
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: member,
        message: Some("Invitation accepted successfully".to_string()),
    }))
}

async fn decline_invitation(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(token): Path<String>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    team_service
        .decline_invitation(&token, &claims.sub)
        .await
        .map_err(|e| (
            axum::http::StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "decline_invitation_failed".to_string(),
                message: e.to_string(),
                code: Some("invitation_error".to_string()),
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Invitation declined".to_string()),
    }))
}

async fn list_invitations(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
) -> Result<Json<SuccessResponse<Vec<Invitation>>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    // Check permissions
    let can_manage = team_service
        .has_team_permission(&team_id, &claims.sub, "manage")
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to view invitations".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            })
        ));
    }

    let invitations = team_service
        .list_invitations(&team_id)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "list_invitations_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: invitations,
        message: None,
    }))
}

async fn cancel_invitation(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((team_id, invitation_id)): Path<(String, String)>,
) -> Result<Json<SuccessResponse<()>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    // Check permissions
    let can_manage = team_service
        .has_team_permission(&team_id, &claims.sub, "manage")
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to cancel invitations".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            })
        ));
    }

    team_service
        .cancel_invitation(&invitation_id, &claims.sub)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "cancel_invitation_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: (),
        message: Some("Invitation cancelled".to_string()),
    }))
}

async fn get_team_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    let is_member = team_service
        .is_team_member(&team_id, &claims.sub)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !is_member {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You are not a member of this team".to_string(),
                code: Some("not_team_member".to_string()),
                details: None,
            })
        ));
    }

    let settings = team_service
        .get_team_settings(&team_id)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "get_settings_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: settings,
        message: None,
    }))
}

async fn update_team_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<String>,
    Json(settings): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let team_service = TeamService::new(state.db.pool().clone(), state.redis.clone());

    let can_manage = team_service
        .has_team_permission(&team_id, &claims.sub, "manage")
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "permission_check_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    if !can_manage {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "You don't have permission to update team settings".to_string(),
                code: Some("insufficient_permissions".to_string()),
                details: None,
            })
        ));
    }

    let updated_settings = team_service
        .update_team_settings(&team_id, settings)
        .await
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "update_settings_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            })
        ))?;

    Ok(Json(SuccessResponse {
        success: true,
        data: updated_settings,
        message: Some("Team settings updated successfully".to_string()),
    }))
}