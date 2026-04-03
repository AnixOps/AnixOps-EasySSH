use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{auth::decode_token, models::*, AppState};

pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/", get(websocket_handler))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    // Authenticate connection
    let token = params.get("token").cloned();

    ws.on_upgrade(move |socket| handle_socket(socket, state, token))
}

async fn handle_socket(socket: WebSocket, state: AppState, token: Option<String>) {
    let connection_id = Uuid::new_v4().to_string();
    let mut user_id: Option<String> = None;
    let mut team_id: Option<String> = None;

    // Authenticate
    if let Some(t) = token {
        match decode_token(&t, &state.config.jwt_secret) {
            Ok(claims) => {
                user_id = Some(claims.sub.clone());
                // Store connection in Redis
                if let Err(e) = state
                    .redis
                    .store_ws_connection(
                        &claims.sub,
                        &connection_id,
                        std::time::Duration::from_secs(3600),
                    )
                    .await
                {
                    error!("Failed to store WebSocket connection: {}", e);
                }
                info!(
                    "WebSocket connection {} authenticated for user {}",
                    connection_id, claims.sub
                );
            }
            Err(e) => {
                warn!("WebSocket authentication failed: {}", e);
                return;
            }
        }
    } else {
        warn!("WebSocket connection without token rejected");
        return;
    }

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

    // Spawn task to forward messages from channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Subscribe to broadcast channel for team events
    let broadcast_tx = state.redis.clone(); // In a real implementation, use a proper broadcast mechanism

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => match serde_json::from_str::<WebSocketMessage>(&text) {
                Ok(ws_msg) => {
                    handle_message(ws_msg, &tx, &user_id, &team_id, &state).await;
                }
                Err(e) => {
                    let error_msg = serde_json::json!({
                        "type": "error",
                        "message": format!("Invalid message format: {}", e)
                    })
                    .to_string();
                    let _ = tx.send(error_msg).await;
                }
            },
            Ok(Message::Binary(_)) => {
                // Handle binary messages if needed
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection {} closed", connection_id);
                break;
            }
            Ok(Message::Ping(_)) => {
                // Automatic pong response
            }
            Ok(Message::Pong(_)) => {
                // Received pong
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    // Cleanup
    if let Some(ref uid) = user_id {
        if let Err(e) = state.redis.remove_ws_connection(uid, &connection_id).await {
            error!("Failed to remove WebSocket connection: {}", e);
        }
    }

    send_task.abort();
}

async fn handle_message(
    msg: WebSocketMessage,
    tx: &tokio::sync::mpsc::Sender<String>,
    user_id: &Option<String>,
    team_id: &Option<String>,
    state: &AppState,
) {
    match msg {
        WebSocketMessage::Ping { timestamp } => {
            let pong = WebSocketMessage::Pong { timestamp };
            if let Ok(json) = serde_json::to_string(&pong) {
                let _ = tx.send(json).await;
            }
        }
        WebSocketMessage::Subscribe { channels } => {
            info!("User {:?} subscribed to channels: {:?}", user_id, channels);
            // Subscribe logic would go here
            let response = serde_json::json!({
                "type": "subscribed",
                "channels": channels
            });
            let _ = tx.send(response.to_string()).await;
        }
        WebSocketMessage::Unsubscribe { channels } => {
            info!(
                "User {:?} unsubscribed from channels: {:?}",
                user_id, channels
            );
            let response = serde_json::json!({
                "type": "unsubscribed",
                "channels": channels
            });
            let _ = tx.send(response.to_string()).await;
        }
        WebSocketMessage::CollaborationUpdate {
            resource_type,
            resource_id,
            action,
            data,
            user_id,
            timestamp,
        } => {
            // Broadcast collaboration update to team members
            if let Some(tid) = team_id {
                broadcast_collaboration_update(
                    state,
                    tid,
                    &resource_type,
                    &resource_id,
                    &action,
                    &data,
                    &user_id,
                    timestamp,
                )
                .await;
            }
        }
        _ => {
            // Handle other message types
        }
    }
}

async fn broadcast_collaboration_update(
    state: &AppState,
    team_id: &str,
    resource_type: &str,
    resource_id: &str,
    action: &str,
    data: &serde_json::Value,
    user_id: &str,
    timestamp: chrono::DateTime<chrono::Utc>,
) {
    // Get all connected users for this team
    // This is a simplified implementation - in production, use a proper pub/sub mechanism
    let message = WebSocketMessage::CollaborationUpdate {
        resource_type: resource_type.to_string(),
        resource_id: resource_id.to_string(),
        action: action.to_string(),
        data: data.clone(),
        user_id: user_id.to_string(),
        timestamp,
    };

    if let Ok(json) = serde_json::to_string(&message) {
        // Broadcast to all team members via Redis pub/sub
        // In a full implementation, this would use Redis Pub/Sub
        tracing::info!("Broadcasting to team {}: {}", team_id, json);
    }
}

// Public function to send notifications
pub async fn send_notification(
    state: &AppState,
    user_id: &str,
    notification: WebSocketMessage,
) -> anyhow::Result<()> {
    // Get user's WebSocket connections
    let connections = state.redis.get_user_ws_connections(user_id).await?;

    if let Ok(json) = serde_json::to_string(&notification) {
        for conn_id in connections {
            // Send to connection - in production, this would use a proper connection manager
            tracing::info!("Sending notification to connection {}: {}", conn_id, json);
        }
    }

    Ok(())
}

// Broadcast to all members of a team
pub async fn broadcast_to_team(
    state: &AppState,
    team_id: &str,
    message: WebSocketMessage,
) -> anyhow::Result<()> {
    // Get all team members
    let members = sqlx::query_scalar::<_, String>(
        "SELECT user_id FROM team_members WHERE team_id = ? AND is_active = TRUE",
    )
    .bind(team_id)
    .fetch_all(state.db.pool())
    .await?;

    for user_id in members {
        send_notification(state, &user_id, message.clone())
            .await
            .ok();
    }

    Ok(())
}
