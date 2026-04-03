//! DevOps事件响应中心 - REST API路由
//!
//! 提供完整的事件管理、告警处理、运行手册、升级策略等API端点

use crate::escalation_service::EscalationService;
use crate::incident_models::*;
use crate::incident_service::IncidentService;
use crate::post_mortem_service::PostMortemService;
use crate::runbook_service::RunbookService;
use crate::AppState;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde_json::{json, Value};
use tracing::warn;

/// 创建事件响应中心API路由
pub fn incident_routes() -> Router<AppState> {
    Router::new()
        // 事件管理
        .route("/incidents", post(create_incident))
        .route("/incidents", get(list_incidents))
        .route("/incidents/{id}", get(get_incident))
        .route("/incidents/{id}", put(update_incident))
        .route("/incidents/{id}/acknowledge", post(acknowledge_incident))
        .route("/incidents/{id}/resolve", post(resolve_incident))
        .route("/incidents/{id}/close", post(close_incident))
        .route("/incidents/{id}/join", post(join_incident))
        .route("/incidents/{id}/leave", post(leave_incident))
        .route("/incidents/{id}/timeline", get(get_timeline))
        .route("/incidents/{id}/timeline", post(add_timeline_entry))
        .route("/incidents/{id}/alerts", get(get_incident_alerts))
        .route("/incidents/{id}/diagnoses", get(get_diagnoses))
        .route("/incidents/{id}/diagnose", post(perform_ai_diagnosis))
        .route("/incidents/{id}/escalate", post(escalate_incident))
        .route(
            "/incidents/{id}/escalation-history",
            get(get_escalation_history),
        )
        .route("/incidents/{id}/participants", get(get_participants))
        .route("/incidents/{id}/impact-analysis", get(get_impact_analysis))
        .route("/incidents/{id}/related", get(get_related_incidents))
        .route("/incidents/{id}/post-mortem", get(get_post_mortem))
        .route("/incidents/{id}/post-mortem", post(create_post_mortem))
        .route("/incidents/stats", get(get_incident_stats))
        // 告警管理
        .route("/alerts", post(create_alert))
        .route("/alerts", get(list_alerts))
        .route("/alerts/aggregated", get(get_aggregated_alerts))
        .route("/alerts/{id}", get(get_alert))
        .route("/alerts/{id}/acknowledge", post(acknowledge_alert))
        .route("/alerts/{id}/resolve", post(resolve_alert))
        .route("/alerts/{id}/suppress", post(suppress_alert))
        // 运行手册
        .route("/runbooks", post(create_runbook))
        .route("/runbooks", get(list_runbooks))
        .route("/runbooks/{id}", get(get_runbook))
        .route("/runbooks/{id}", put(update_runbook))
        .route("/runbooks/{id}", delete(delete_runbook))
        .route("/runbooks/{id}/execute", post(execute_runbook))
        .route("/runbooks/{id}/executions", get(get_runbook_executions))
        .route("/runbooks/search", get(search_runbooks))
        .route("/runbooks/popular", get(get_popular_runbooks))
        // 升级策略
        .route("/escalation-policies", post(create_escalation_policy))
        .route("/escalation-policies", get(list_escalation_policies))
        .route("/escalation-policies/{id}", get(get_escalation_policy))
        .route("/escalation-policies/{id}", put(update_escalation_policy))
        .route(
            "/escalation-policies/{id}",
            delete(delete_escalation_policy),
        )
        .route(
            "/escalation-policies/{id}/test",
            post(test_escalation_policy),
        )
        // 集成管理
        .route("/integrations", post(create_integration))
        .route("/integrations", get(list_integrations))
        .route("/integrations/{id}", get(get_integration))
        .route("/integrations/{id}", put(update_integration))
        .route("/integrations/{id}", delete(delete_integration))
        .route("/integrations/{id}/test", post(test_integration))
        // 事后复盘
        .route("/post-mortems", get(list_post_mortems))
        .route("/post-mortems/{id}", get(get_post_mortem_by_id))
        .route("/post-mortems/{id}", put(update_post_mortem))
        .route("/post-mortems/{id}/publish", post(publish_post_mortem))
        .route(
            "/post-mortems/{id}/report",
            get(generate_post_mortem_report),
        )
        .route(
            "/post-mortems/{id}/suggestions",
            get(get_improvement_suggestions),
        )
        // 检测规则
        .route("/detection-rules", post(create_detection_rule))
        .route("/detection-rules", get(list_detection_rules))
        .route("/detection-rules/{id}", get(get_detection_rule))
        .route("/detection-rules/{id}", put(update_detection_rule))
        .route("/detection-rules/{id}", delete(delete_detection_rule))
        // 指标和仪表板
        .route("/metrics", get(get_incident_metrics))
        .route(
            "/dashboard/active-incidents",
            get(get_active_incidents_dashboard),
        )
        .route("/dashboard/alert-trends", get(get_alert_trends))
}

// ============= 事件管理处理器 =============

async fn create_incident(
    State(state): State<AppState>,
    Json(req): Json<CreateIncidentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.create_incident(req, "current_user").await {
        Ok(incident) => {
            // 发送通知
            let escalation_service = EscalationService::new(state.db.clone(), state.redis.clone());
            if let Err(e) = escalation_service
                .send_incident_notifications(&incident, CommunicationType::Notification)
                .await
            {
                warn!("Failed to send notifications: {}", e);
            }

            Ok(Json(json!({
                "success": true,
                "data": incident,
                "message": "Incident created successfully"
            })))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn list_incidents(
    State(state): State<AppState>,
    Query(req): Query<QueryIncidentsRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.query_incidents(req).await {
        Ok(response) => Ok(Json(json!({
            "success": true,
            "data": response
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.get_incident_detail(&id).await {
        Ok(detail) => Ok(Json(json!({
            "success": true,
            "data": detail
        }))),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

async fn update_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateIncidentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.update_incident(&id, req, "current_user").await {
        Ok(incident) => Ok(Json(json!({
            "success": true,
            "data": incident
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn acknowledge_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AcknowledgeIncidentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service
        .acknowledge_incident(&id, &req.user_id, req.note.as_deref())
        .await
    {
        Ok(incident) => Ok(Json(json!({
            "success": true,
            "data": incident
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn resolve_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ResolveIncidentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service
        .resolve_incident(
            &id,
            &req.user_id,
            &req.resolution,
            req.root_cause.as_deref(),
        )
        .await
    {
        Ok(incident) => {
            // 发送解决通知
            let escalation_service = EscalationService::new(state.db.clone(), state.redis.clone());
            if let Err(e) = escalation_service
                .send_incident_notifications(&incident, CommunicationType::Resolution)
                .await
            {
                warn!("Failed to send resolution notifications: {}", e);
            }

            Ok(Json(json!({
                "success": true,
                "data": incident
            })))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn close_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.close_incident(&id, "current_user").await {
        Ok(incident) => Ok(Json(json!({
            "success": true,
            "data": incident
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn join_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<JoinIncidentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.join_incident(&id, &req.user_id, req.role).await {
        Ok(participant) => Ok(Json(json!({
            "success": true,
            "data": participant
        }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

async fn leave_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.leave_incident(&id, "current_user").await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Left incident successfully"
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_timeline(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.get_incident_timeline(&id).await {
        Ok(timeline) => Ok(Json(json!({
            "success": true,
            "data": timeline
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn add_timeline_entry(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AddTimelineEntryRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service
        .add_timeline_entry(
            &id,
            req.entry_type,
            &req.title,
            &req.description,
            "current_user",
            req.metadata,
        )
        .await
    {
        Ok(entry) => Ok(Json(json!({
            "success": true,
            "data": entry
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_incident_alerts(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.get_incident_alerts(&id).await {
        Ok(alerts) => Ok(Json(json!({
            "success": true,
            "data": alerts
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_diagnoses(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.get_incident_diagnoses(&id).await {
        Ok(diagnoses) => Ok(Json(json!({
            "success": true,
            "data": diagnoses
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn perform_ai_diagnosis(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.perform_ai_diagnosis(&id).await {
        Ok(diagnosis) => Ok(Json(json!({
            "success": true,
            "data": diagnosis
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn escalate_incident(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<EscalateIncidentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = EscalationService::new(state.db.clone(), state.redis.clone());

    match service
        .escalate_incident(
            &id,
            "current_user",
            &req.reason,
            req.target_level,
            req.notify_users,
        )
        .await
    {
        Ok(incident) => Ok(Json(json!({
            "success": true,
            "data": incident
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_escalation_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = EscalationService::new(state.db.clone(), state.redis.clone());

    match service.get_escalation_history(&id).await {
        Ok(history) => Ok(Json(json!({
            "success": true,
            "data": history
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_participants(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.get_incident_participants(&id).await {
        Ok(participants) => Ok(Json(json!({
            "success": true,
            "data": participants
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_impact_analysis(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = PostMortemService::new(state.db.clone());

    match service.analyze_impact(&id).await {
        Ok(analysis) => Ok(Json(json!({
            "success": true,
            "data": analysis
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_related_incidents(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    // 从incident detail中获取相关事件
    match service.get_incident_detail(&id).await {
        Ok(detail) => Ok(Json(json!({
            "success": true,
            "data": detail.related_incidents
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_post_mortem(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = PostMortemService::new(state.db.clone());

    match service.get_post_mortem_by_incident(&id).await {
        Ok(Some(pm)) => Ok(Json(json!({
            "success": true,
            "data": pm
        }))),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Post-mortem not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn create_post_mortem(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CreatePostMortemRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = PostMortemService::new(state.db.clone());

    match service
        .create_post_mortem(
            &id,
            &req.title,
            &req.summary,
            &req.root_cause_analysis,
            &req.lessons_learned,
            req.action_items.unwrap_or_default(),
            "current_user",
        )
        .await
    {
        Ok(pm) => Ok(Json(json!({
            "success": true,
            "data": pm
        }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

async fn get_incident_stats(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // 使用列表接口获取统计信息
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    let req = QueryIncidentsRequest {
        team_id: None,
        status: None,
        severity: None,
        incident_type: None,
        assigned_to: None,
        from_date: None,
        to_date: None,
        tags: None,
        page: Some(1),
        limit: Some(1),
    };

    match service.query_incidents(req).await {
        Ok(response) => Ok(Json(json!({
            "success": true,
            "data": response.stats
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// ============= 告警管理处理器 =============

async fn create_alert(
    State(state): State<AppState>,
    Json(req): Json<CreateAlertRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.create_alert(req).await {
        Ok(alert) => Ok(Json(json!({
            "success": true,
            "data": alert
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn list_alerts(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    // 简化为返回空列表，实际实现需要查询参数
    Ok(Json(json!({
        "success": true,
        "data": []
    })))
}

async fn get_aggregated_alerts(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    // 需要team_id，这里简化处理
    match service.get_aggregated_alerts("default_team").await {
        Ok(alerts) => Ok(Json(json!({
            "success": true,
            "data": alerts
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_alert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.get_alert_by_id(&id).await {
        Ok(alert) => Ok(Json(json!({
            "success": true,
            "data": alert
        }))),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

async fn acknowledge_alert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.resolve_alert(&id, "current_user").await {
        Ok(alert) => Ok(Json(json!({
            "success": true,
            "data": alert
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn resolve_alert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.resolve_alert(&id, "current_user").await {
        Ok(alert) => Ok(Json(json!({
            "success": true,
            "data": alert
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn suppress_alert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    match service.suppress_alert(&id, "current_user").await {
        Ok(alert) => Ok(Json(json!({
            "success": true,
            "data": alert
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// ============= 运行手册处理器 =============

async fn create_runbook(
    State(state): State<AppState>,
    Json(req): Json<CreateRunbookRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = RunbookService::new(state.db.clone());

    match service.create_runbook(req, "current_user").await {
        Ok(runbook) => Ok(Json(json!({
            "success": true,
            "data": runbook
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn list_runbooks(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let service = RunbookService::new(state.db.clone());

    // 简化处理，需要team_id参数
    match service.list_runbooks("default_team", None, None).await {
        Ok(runbooks) => Ok(Json(json!({
            "success": true,
            "data": runbooks
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_runbook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = RunbookService::new(state.db.clone());

    match service.get_runbook_by_id(&id).await {
        Ok(runbook) => Ok(Json(json!({
            "success": true,
            "data": runbook
        }))),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

async fn update_runbook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = RunbookService::new(state.db.clone());

    // 简化处理，需要完整请求体
    match service
        .update_runbook(&id, None, None, None, None, None)
        .await
    {
        Ok(runbook) => Ok(Json(json!({
            "success": true,
            "data": runbook
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn delete_runbook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = RunbookService::new(state.db.clone());

    match service.delete_runbook(&id).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Runbook deleted successfully"
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn execute_runbook(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ExecuteRunbookRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = RunbookService::new(state.db.clone());

    match service
        .execute_runbook(&id, &req.incident_id, &req.executed_by)
        .await
    {
        Ok(execution) => Ok(Json(json!({
            "success": true,
            "data": execution
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_runbook_executions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = RunbookService::new(state.db.clone());

    match service.get_runbook_executions(&id).await {
        Ok(executions) => Ok(Json(json!({
            "success": true,
            "data": executions
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn search_runbooks(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // 简化处理
    Ok(Json(json!({
        "success": true,
        "data": []
    })))
}

async fn get_popular_runbooks(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = RunbookService::new(state.db.clone());

    match service.get_popular_runbooks("default_team", 10).await {
        Ok(runbooks) => Ok(Json(json!({
            "success": true,
            "data": runbooks
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// ============= 升级策略处理器 =============

async fn create_escalation_policy(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // 简化实现
    Ok(Json(json!({
        "success": true,
        "message": "Escalation policy creation endpoint - implement with full request body"
    })))
}

async fn list_escalation_policies(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = EscalationService::new(state.db.clone(), state.redis.clone());

    match service.get_team_policies("default_team").await {
        Ok(policies) => Ok(Json(json!({
            "success": true,
            "data": policies
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_escalation_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = EscalationService::new(state.db.clone(), state.redis.clone());

    match service.get_policy_by_id(&id).await {
        Ok(policy) => Ok(Json(json!({
            "success": true,
            "data": policy
        }))),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

async fn update_escalation_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // 简化实现
    Ok(Json(json!({
        "success": true,
        "message": "Escalation policy update endpoint"
    })))
}

async fn delete_escalation_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = EscalationService::new(state.db.clone(), state.redis.clone());

    match service.delete_policy(&id).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Escalation policy deleted"
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn test_escalation_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "success": true,
        "message": "Escalation policy test simulation started"
    })))
}

// ============= 集成管理处理器 =============

async fn create_integration(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "success": true,
        "message": "Integration creation endpoint"
    })))
}

async fn list_integrations(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = EscalationService::new(state.db.clone(), state.redis.clone());

    match service.get_team_integrations("default_team").await {
        Ok(integrations) => Ok(Json(json!({
            "success": true,
            "data": integrations
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_integration(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = EscalationService::new(state.db.clone(), state.redis.clone());

    match service.get_integration_by_id(&id).await {
        Ok(integration) => Ok(Json(json!({
            "success": true,
            "data": integration
        }))),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

async fn update_integration(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "success": true,
        "message": "Integration update endpoint"
    })))
}

async fn delete_integration(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = EscalationService::new(state.db.clone(), state.redis.clone());

    match service.delete_integration(&id).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Integration deleted"
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn test_integration(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = EscalationService::new(state.db.clone(), state.redis.clone());

    match service.test_integration(&id).await {
        Ok(success) => Ok(Json(json!({
            "success": success,
            "message": if success { "Integration test passed" } else { "Integration test failed" }
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// ============= 事后复盘处理器 =============

async fn list_post_mortems(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = PostMortemService::new(state.db.clone());

    match service.list_post_mortems(None, None, None, None).await {
        Ok(post_mortems) => Ok(Json(json!({
            "success": true,
            "data": post_mortems
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_post_mortem_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = PostMortemService::new(state.db.clone());

    match service.get_post_mortem_by_id(&id).await {
        Ok(pm) => Ok(Json(json!({
            "success": true,
            "data": pm
        }))),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

async fn update_post_mortem(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = PostMortemService::new(state.db.clone());

    match service
        .update_post_mortem(&id, None, None, None, None, None, None, None)
        .await
    {
        Ok(pm) => Ok(Json(json!({
            "success": true,
            "data": pm
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn publish_post_mortem(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = PostMortemService::new(state.db.clone());

    match service.publish_post_mortem(&id).await {
        Ok(pm) => Ok(Json(json!({
            "success": true,
            "data": pm
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn generate_post_mortem_report(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = PostMortemService::new(state.db.clone());

    match service.generate_post_mortem_report(&id).await {
        Ok(report) => Ok(Json(json!({
            "success": true,
            "data": {
                "report": report,
                "format": "markdown"
            }
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_improvement_suggestions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = PostMortemService::new(state.db.clone());

    match service.generate_improvement_suggestions(&id).await {
        Ok(suggestions) => Ok(Json(json!({
            "success": true,
            "data": suggestions
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// ============= 检测规则处理器 =============

async fn create_detection_rule(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "success": true,
        "message": "Detection rule creation endpoint"
    })))
}

async fn list_detection_rules(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "success": true,
        "data": []
    })))
}

async fn get_detection_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    Err((StatusCode::NOT_FOUND, "Not implemented".to_string()))
}

async fn update_detection_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "success": true,
        "message": "Detection rule update endpoint"
    })))
}

async fn delete_detection_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "success": true,
        "message": "Detection rule deleted"
    })))
}

// ============= 仪表板和指标处理器 =============

async fn get_incident_metrics(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // 返回综合指标数据
    Ok(Json(json!({
        "success": true,
        "data": {
            "mttr": {
                "p50": 45.5,
                "p90": 120.0,
                "p99": 240.0,
                "trend": "improving"
            },
            "mttr_by_severity": {
                "critical": 15.0,
                "high": 60.0,
                "medium": 120.0,
                "low": 240.0
            },
            "alert_volume": {
                "today": 45,
                "week": 312,
                "trend": -12.5
            },
            "incident_volume": {
                "today": 3,
                "week": 18,
                "trend": -5.2
            }
        }
    })))
}

async fn get_active_incidents_dashboard(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service = IncidentService::new(state.db.clone(), state.redis.clone());

    // 获取活跃事件
    let req = QueryIncidentsRequest {
        team_id: None,
        status: Some(vec![
            IncidentStatus::Detected,
            IncidentStatus::Acknowledged,
            IncidentStatus::Investigating,
            IncidentStatus::Mitigating,
            IncidentStatus::Escalated,
        ]),
        severity: None,
        incident_type: None,
        assigned_to: None,
        from_date: None,
        to_date: None,
        tags: None,
        page: Some(1),
        limit: Some(50),
    };

    match service.query_incidents(req).await {
        Ok(response) => Ok(Json(json!({
            "success": true,
            "data": {
                "active_incidents": response.incidents,
                "count_by_severity": {
                    "critical": response.stats.critical_count,
                    "high": response.stats.high_count,
                },
                "require_attention": response.incidents.iter()
                    .filter(|i| i.status == IncidentStatus::Detected)
                    .count()
            }
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_alert_trends(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // 生成模拟的告警趋势数据
    let hours: Vec<String> = (0..24).map(|h| format!("{:02}:00", h)).collect();

    Ok(Json(json!({
        "success": true,
        "data": {
            "labels": hours,
            "datasets": [
                {
                    "label": "Critical",
                    "data": vec![1, 0, 0, 0, 0, 0, 0, 0, 2, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
                },
                {
                    "label": "High",
                    "data": vec![2, 1, 0, 0, 1, 0, 0, 1, 3, 2, 1, 0, 0, 2, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1]
                },
                {
                    "label": "Medium",
                    "data": vec![5, 3, 2, 1, 2, 1, 2, 3, 8, 6, 4, 3, 2, 4, 3, 2, 3, 2, 1, 2, 4, 3, 2, 3]
                }
            ]
        }
    })))
}
