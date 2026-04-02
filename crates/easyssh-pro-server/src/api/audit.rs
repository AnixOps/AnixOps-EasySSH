use axum::{
    extract::{Extension, Query, State},
    routing::get,
    Json, Router,
};
use chrono::Utc;

use crate::{auth::Claims, models::*, services::audit_service::AuditService, AppState};

pub fn audit_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(query_audit_logs))
        .route("/export", get(export_audit_logs))
        .route("/stats", get(get_audit_stats))
}

async fn query_audit_logs(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<QueryAuditLogsRequest>,
) -> Result<
    Json<SuccessResponse<AuditLogListResponse>>,
    (axum::http::StatusCode, Json<ErrorResponse>),
> {
    let audit_service = AuditService::new(state.db.pool().clone());

    // Check permissions
    if let Some(ref team_id) = params.team_id {
        // User must have audit read permission for this team
        let has_permission = check_audit_permission(&state, &claims.sub, team_id).await?;
        if !has_permission {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "forbidden".to_string(),
                    message: "You don't have permission to view audit logs for this team"
                        .to_string(),
                    code: Some("insufficient_permissions".to_string()),
                    details: None,
                }),
            ));
        }
    } else if !claims.is_admin {
        // Non-admin users must specify a team they have access to
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Must specify team_id to view audit logs".to_string(),
                code: Some("missing_team_id".to_string()),
                details: None,
            }),
        ));
    }

    let limit = params.limit.unwrap_or(50).min(1000);
    let offset = params.offset.unwrap_or(0);

    let (logs, total) = audit_service
        .query_logs(
            params.team_id.as_deref(),
            params.user_id.as_deref(),
            params.action.as_deref(),
            params.resource_type.as_deref(),
            params.from_date,
            params.to_date,
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "query_logs_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: AuditLogListResponse {
            logs,
            total,
            limit,
            offset,
        },
        message: None,
    }))
}

async fn export_audit_logs(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<QueryAuditLogsRequest>,
) -> Result<axum::response::Response, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let audit_service = AuditService::new(state.db.pool().clone());

    // Check permissions (same as query)
    if let Some(ref team_id) = params.team_id {
        let has_permission = check_audit_permission(&state, &claims.sub, team_id).await?;
        if !has_permission {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "forbidden".to_string(),
                    message: "You don't have permission to export audit logs for this team"
                        .to_string(),
                    code: Some("insufficient_permissions".to_string()),
                    details: None,
                }),
            ));
        }
    } else if !claims.is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Must specify team_id to export audit logs".to_string(),
                code: Some("missing_team_id".to_string()),
                details: None,
            }),
        ));
    }

    // Export all matching logs (no limit for export)
    let (logs, _) = audit_service
        .query_logs(
            params.team_id.as_deref(),
            params.user_id.as_deref(),
            params.action.as_deref(),
            params.resource_type.as_deref(),
            params.from_date,
            params.to_date,
            10000, // Max export size
            0,
        )
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "export_logs_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    // Serialize to CSV
    let csv = serialize_to_csv(&logs).map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "csv_serialization_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        )
    })?;

    let response = axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", "text/csv")
        .header(
            "Content-Disposition",
            "attachment; filename=\"audit_logs.csv\"",
        )
        .body(axum::body::Body::from(csv))
        .map_err(|e| (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "response_build_failed".to_string(),
                message: e.to_string(),
                code: None,
                details: None,
            }),
        ))?;

    Ok(response)
}

async fn get_audit_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<QueryAuditLogsRequest>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, (axum::http::StatusCode, Json<ErrorResponse>)>
{
    let audit_service = AuditService::new(state.db.pool().clone());

    // Check permissions
    if let Some(ref team_id) = params.team_id {
        let has_permission = check_audit_permission(&state, &claims.sub, team_id).await?;
        if !has_permission {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "forbidden".to_string(),
                    message: "You don't have permission to view audit stats for this team"
                        .to_string(),
                    code: Some("insufficient_permissions".to_string()),
                    details: None,
                }),
            ));
        }
    } else if !claims.is_admin {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "forbidden".to_string(),
                message: "Must specify team_id to view audit stats".to_string(),
                code: Some("missing_team_id".to_string()),
                details: None,
            }),
        ));
    }

    let stats = audit_service
        .get_stats(params.team_id.as_deref(), params.from_date, params.to_date)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "stats_query_failed".to_string(),
                    message: e.to_string(),
                    code: None,
                    details: None,
                }),
            )
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        data: stats,
        message: None,
    }))
}

async fn check_audit_permission(
    state: &AppState,
    user_id: &str,
    team_id: &str,
) -> Result<bool, (axum::http::StatusCode, Json<ErrorResponse>)> {
    // For now, check if user is a team member
    // In a real implementation, this would use the RBAC service
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
            error: "permission_check_failed".to_string(),
            message: e.to_string(),
            code: None,
            details: None,
        })
    ))?;

    Ok(result)
}

fn serialize_to_csv(logs: &[AuditLog]) -> anyhow::Result<String> {
    let mut csv = String::new();

    // Header
    csv.push_str("timestamp,user_id,team_id,action,resource_type,resource_id,success,ip_address,user_agent,details\n");

    // Rows
    for log in logs {
        let details = log
            .details
            .as_ref()
            .map(|d| d.to_string())
            .unwrap_or_default()
            .replace('"', "\"\"");

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},\"{}\"\n",
            log.timestamp.to_rfc3339(),
            log.user_id.as_deref().unwrap_or(""),
            log.team_id.as_deref().unwrap_or(""),
            &log.action,
            &log.resource_type,
            log.resource_id.as_deref().unwrap_or(""),
            log.success,
            log.ip_address.as_deref().unwrap_or(""),
            log.user_agent.as_deref().unwrap_or(""),
            details
        ));
    }

    Ok(csv)
}
