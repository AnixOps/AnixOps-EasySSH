use crate::types::*;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub struct WebSocketClient {
    url: String,
    tx: Option<mpsc::Sender<String>>,
    messages: Arc<RwLock<Vec<WebSocketMessage>>>,
    is_connected: Arc<RwLock<bool>>,
}

impl WebSocketClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            tx: None,
            messages: Arc::new(RwLock::new(Vec::new())),
            is_connected: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn connect(&mut self, headers: Option<Vec<KeyValue>>) -> ApiResult<()> {
        let request = http::Request::builder()
            .uri(&self.url)
            .header("Host", extract_host(&self.url)?);

        // Add custom headers
        let request = if let Some(h) = headers {
            let mut req = request;
            for header in h {
                if header.enabled && !header.key.is_empty() {
                    req = req.header(&header.key, &header.value);
                }
            }
            req
        } else {
            request
        };

        let request = request
            .body(())
            .map_err(|e| ApiError::WebSocket(e.to_string()))?;

        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|e| ApiError::WebSocket(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<String>(100);
        self.tx = Some(tx);

        let messages = self.messages.clone();
        let is_connected = self.is_connected.clone();

        // Set connected flag
        *is_connected.write().await = true;

        // Spawn write task
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(_) = write.send(WsMessage::Text(msg)).await {
                    break;
                }
            }
        });

        // Spawn read task
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        let msg = WebSocketMessage {
                            timestamp: chrono::Utc::now(),
                            direction: MessageDirection::Received,
                            content: text,
                            message_type: "text".to_string(),
                        };
                        messages.write().await.push(msg);
                    }
                    Ok(WsMessage::Binary(data)) => {
                        let text = String::from_utf8_lossy(&data).to_string();
                        let msg = WebSocketMessage {
                            timestamp: chrono::Utc::now(),
                            direction: MessageDirection::Received,
                            content: text,
                            message_type: "binary".to_string(),
                        };
                        messages.write().await.push(msg);
                    }
                    Ok(WsMessage::Close(_)) => {
                        *is_connected.write().await = false;
                        break;
                    }
                    Err(_) => {
                        *is_connected.write().await = false;
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub async fn send(&self, message: impl Into<String>) -> ApiResult<()> {
        let msg = message.into();

        if let Some(tx) = &self.tx {
            tx.send(msg.clone())
                .await
                .map_err(|e| ApiError::WebSocket(e.to_string()))?;

            // Record sent message
            let sent_msg = WebSocketMessage {
                timestamp: chrono::Utc::now(),
                direction: MessageDirection::Sent,
                content: msg,
                message_type: "text".to_string(),
            };
            self.messages.write().await.push(sent_msg);

            Ok(())
        } else {
            Err(ApiError::WebSocket("Not connected".to_string()))
        }
    }

    pub async fn disconnect(&mut self) -> ApiResult<()> {
        self.tx = None;
        *self.is_connected.write().await = false;
        Ok(())
    }

    pub async fn get_messages(&self) -> Vec<WebSocketMessage> {
        self.messages.read().await.clone()
    }

    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }

    pub async fn clear_messages(&self) {
        self.messages.write().await.clear();
    }
}

fn extract_host(url: &str) -> ApiResult<String> {
    let parsed = url::Url::parse(url)
        .map_err(|e| ApiError::InvalidUrl(e.to_string()))?;

    parsed.host_str()
        .map(|h| h.to_string())
        .ok_or_else(|| ApiError::InvalidUrl("No host found".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_host() {
        assert_eq!(
            extract_host("ws://example.com:8080/socket").unwrap(),
            "example.com:8080"
        );
        assert_eq!(
            extract_host("wss://echo.websocket.org/").unwrap(),
            "echo.websocket.org"
        );
    }
}
