use crate::ai_programming;
use futures_util::{sink::Sink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::{net::TcpListener, net::TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};


#[derive(Debug, Deserialize)]
struct WsRequest {
    #[serde(default, rename = "type")]
    kind: Option<String>,
    id: Option<String>,
    op: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct WsError {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

#[derive(Debug, Serialize)]
struct WsResponse {
    #[serde(rename = "type")]
    kind: &'static str,
    id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<WsError>,
}

pub async fn run_server(host: &str, port: u16) -> Result<(), String> {
    let bind_addr = resolve_loopback_addr(host, port)?;
    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|e| format!("启动 WebSocket Debug 服务失败: {e}"))?;

    log::info!("WebSocket Debug 服务已启动: ws://{}", bind_addr);
    log::warn!("Debug 服务已开启，AI 具备完整控制权限");

    loop {
        let (stream, peer_addr) = listener
            .accept()
            .await
            .map_err(|e| format!("接受连接失败: {e}"))?;

        if !peer_addr.ip().is_loopback() {
            log::warn!("拒绝非 loopback 连接: {}", peer_addr);
            continue;
        }

        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, peer_addr).await {
                log::warn!("WebSocket 连接结束: {}", err);
            }
        });
    }
}

fn resolve_loopback_addr(host: &str, port: u16) -> Result<SocketAddr, String> {
    let candidates = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("解析监听地址失败: {e}"))?
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Err(format!("无法解析监听地址: {host}:{port}"));
    }

    let addr = candidates[0];
    if !addr.ip().is_loopback() {
        return Err("WebSocket Debug 服务只能绑定到 loopback 地址".to_string());
    }

    Ok(addr)
}

async fn handle_connection(stream: TcpStream, peer_addr: SocketAddr) -> Result<(), String> {
    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| format!("WebSocket 握手失败 ({peer_addr}): {e}"))?;

    let (mut writer, mut reader) = ws_stream.split();
    let welcome = json!({
        "type": "event",
        "event": "ready",
        "payload": {
            "peer": peer_addr.to_string(),
            "capabilities": capabilities_list(),
        }
    });

    writer
        .send(Message::Text(welcome.to_string().into()))
        .await
        .map_err(|e| format!("发送欢迎消息失败: {e}"))?;

    while let Some(msg) = reader.next().await {
        let msg = msg.map_err(|e| format!("接收消息失败: {e}"))?;
        if !msg.is_text() {
            continue;
        }

        let text = msg.to_text().map_err(|e| format!("解析消息失败: {e}"))?;
        let request: WsRequest = match serde_json::from_str(text) {
            Ok(req) => req,
            Err(err) => {
                send_response(
                    &mut writer,
                    WsResponse {
                        kind: "response",
                        id: String::new(),
                        ok: false,
                        result: None,
                        error: Some(WsError {
                            code: "invalid_json".to_string(),
                            message: err.to_string(),
                            details: Some(Value::String(text.to_string())),
                        }),
                    },
                )
                .await?;
                continue;
            }
        };

        if let Some(kind) = &request.kind {
            if kind != "request" {
                send_response(
                    &mut writer,
                    WsResponse {
                        kind: "response",
                        id: request.id.unwrap_or_default(),
                        ok: false,
                        result: None,
                        error: Some(WsError {
                            code: "invalid_message".to_string(),
                            message: format!("不支持的消息类型: {kind}"),
                            details: None,
                        }),
                    },
                )
                .await?;
                continue;
            }
        }

        let req_id = request.id.unwrap_or_default();
        let response = match dispatch(&request.op, request.params).await {
            Ok(result) => WsResponse {
                kind: "response",
                id: req_id,
                ok: true,
                result: Some(result),
                error: None,
            },
            Err(error) => WsResponse {
                kind: "response",
                id: req_id,
                ok: false,
                result: None,
                error: Some(error),
            },
        };

        send_response(&mut writer, response).await?;
    }

    Ok(())
}

async fn send_response<S>(writer: &mut S, response: WsResponse) -> Result<(), String>
where
    S: Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    writer
        .send(Message::Text(
            serde_json::to_string(&response)
                .map_err(|e| format!("序列化响应失败: {e}"))?
                .into(),
        ))
        .await
        .map_err(|e| format!("发送响应失败: {e}"))
}

async fn dispatch(op: &str, params: Value) -> Result<Value, WsError> {
    match op {
        "health.check" => serde_json::to_value(ai_programming::ai_health_check().map_err(|e| WsError {
            code: "health_error".to_string(),
            message: e,
            details: None,
        })?)
        .map_err(|e| serde_json_error("health_error", e)),

        "capabilities.list" => Ok(json!({"ops": capabilities_list()})),

        "fs.read" => {
            let path = param_string(&params, "path")?;
            let content = ai_programming::ai_read_code(path.clone()).await.map_err(|e| WsError {
                code: "fs_read_error".to_string(),
                message: e,
                details: Some(json!({"path": path})),
            })?;
            Ok(json!({"path": path, "content": content}))
        }

        "fs.list" => {
            let dir = param_string(&params, "dir")?;
            let pattern = params.get("pattern").and_then(Value::as_str).map(ToOwned::to_owned);
            let files = ai_programming::ai_list_files(dir.clone(), pattern.clone()).await.map_err(|e| WsError {
                code: "fs_list_error".to_string(),
                message: e,
                details: Some(json!({"dir": dir, "pattern": pattern})),
            })?;
            Ok(json!({"files": files}))
        }

        "code.search" => {
            let query = param_string(&params, "query")?;
            let path = params.get("path").and_then(Value::as_str).map(ToOwned::to_owned);
            let results = ai_programming::ai_search_code(query.clone(), path.clone()).await.map_err(|e| WsError {
                code: "search_error".to_string(),
                message: e,
                details: Some(json!({"query": query, "path": path})),
            })?;
            Ok(serde_json::to_value(results).map_err(|e| serde_json_error("search_error", e))?)
        }

        "rust.check" => serde_json::to_value(ai_programming::ai_check_rust().await.map_err(|e| WsError {
            code: "rust_check_error".to_string(),
            message: e,
            details: None,
        })?)
        .map_err(|e| serde_json_error("rust_check_error", e)),

        "test.run" => serde_json::to_value(ai_programming::ai_run_tests().await.map_err(|e| WsError {
            code: "test_run_error".to_string(),
            message: e,
            details: None,
        })?)
        .map_err(|e| serde_json_error("test_run_error", e)),

        "build.run" => serde_json::to_value(ai_programming::ai_build().await.map_err(|e| WsError {
            code: "build_run_error".to_string(),
            message: e,
            details: None,
        })?)
        .map_err(|e| serde_json_error("build_run_error", e)),

        "debug.quick_check" => serde_json::to_value(ai_programming::debug_quick_check().map_err(|e| WsError {
            code: "debug_error".to_string(),
            message: e,
            details: Some(json!({"op": op})),
        })?)
        .map_err(|e| serde_json_error("debug_error", e)),

        "debug.test_all" => serde_json::to_value(ai_programming::debug_test_all().map_err(|e| WsError {
            code: "debug_error".to_string(),
            message: e,
            details: Some(json!({"op": op})),
        })?)
        .map_err(|e| serde_json_error("debug_error", e)),

        "debug.test_db" => serde_json::to_value(ai_programming::debug_test_db().map_err(|e| WsError {
            code: "debug_error".to_string(),
            message: e,
            details: Some(json!({"op": op})),
        })?)
        .map_err(|e| serde_json_error("debug_error", e)),

        "debug.test_crypto" => serde_json::to_value(ai_programming::debug_test_crypto().map_err(|e| WsError {
            code: "debug_error".to_string(),
            message: e,
            details: Some(json!({"op": op})),
        })?)
        .map_err(|e| serde_json_error("debug_error", e)),

        "debug.test_ssh" => serde_json::to_value(ai_programming::debug_test_ssh().map_err(|e| WsError {
            code: "debug_error".to_string(),
            message: e,
            details: Some(json!({"op": op})),
        })?)
        .map_err(|e| serde_json_error("debug_error", e)),

        "debug.test_terminal" => serde_json::to_value(ai_programming::debug_test_terminal().map_err(|e| WsError {
            code: "debug_error".to_string(),
            message: e,
            details: Some(json!({"op": op})),
        })?)
        .map_err(|e| serde_json_error("debug_error", e)),

        "debug.test_pro" => serde_json::to_value(ai_programming::debug_test_pro().map_err(|e| WsError {
            code: "debug_error".to_string(),
            message: e,
            details: Some(json!({"op": op})),
        })?)
        .map_err(|e| serde_json_error("debug_error", e)),

        "git.status" => serde_json::to_value(ai_programming::git_status().await.map_err(|e| WsError {
            code: "git_error".to_string(),
            message: e,
            details: Some(json!({"op": op})),
        })?)
        .map_err(|e| serde_json_error("git_error", e)),

        "git.diff" => {
            let path = params.get("path").and_then(Value::as_str).map(ToOwned::to_owned);
            let diff = ai_programming::git_diff(path.clone()).await.map_err(|e| WsError {
                code: "git_error".to_string(),
                message: e,
                details: Some(json!({"path": path})),
            })?;
            Ok(json!({"diff": diff}))
        }

        "git.log" => {
            let count = params.get("count").and_then(Value::as_u64).unwrap_or(10) as usize;
            let log = ai_programming::git_log(count).await.map_err(|e| WsError {
                code: "git_error".to_string(),
                message: e,
                details: Some(json!({"count": count})),
            })?;
            Ok(serde_json::to_value(log).map_err(|e| serde_json_error("git_error", e))?)
        }

        "git.branch" => serde_json::to_value(ai_programming::git_branch().await.map_err(|e| WsError {
            code: "git_error".to_string(),
            message: e,
            details: None,
        })?)
        .map_err(|e| serde_json_error("git_error", e)),

        "fs.write" => {
            let path = param_string(&params, "path")?;
            let content = param_string(&params, "content")?;
            ai_programming::write_file(path.clone(), content.clone()).await.map_err(|e| WsError {
                code: "fs_write_error".to_string(),
                message: e,
                details: Some(json!({"path": path})),
            })?;
            Ok(json!({"written": true}))
        }

        "fs.edit" => {
            let path = param_string(&params, "path")?;
            let old_string = param_string(&params, "old_string")?;
            let new_string = param_string(&params, "new_string")?;
            let result = ai_programming::edit_file(path.clone(), old_string, new_string).await.map_err(|e| WsError {
                code: "fs_edit_error".to_string(),
                message: e,
                details: Some(json!({"path": path})),
            })?;
            Ok(serde_json::to_value(result).map_err(|e| serde_json_error("fs_edit_error", e))?)
        }

        "context.set" => {
            let key = param_string(&params, "key")?;
            let value = param_string(&params, "value")?;
            ai_programming::set_context(key.clone(), value.clone()).map_err(|e| WsError {
                code: "context_error".to_string(),
                message: e,
                details: Some(json!({"key": key})),
            })?;
            Ok(json!({"ok": true}))
        }

        "context.get" => {
            let key = param_string(&params, "key")?;
            let value = ai_programming::get_context(key.clone()).map_err(|e| WsError {
                code: "context_error".to_string(),
                message: e,
                details: Some(json!({"key": key})),
            })?;
            Ok(json!({"value": value}))
        }

        "context.clear" => {
            ai_programming::clear_context().map_err(|e| WsError {
                code: "context_error".to_string(),
                message: e,
                details: None,
            })?;
            Ok(json!({"cleared": true}))
        }

        _ => Err(WsError {
            code: "unknown_op".to_string(),
            message: format!("未知操作: {op}"),
            details: Some(json!({"op": op})),
        }),
    }
}

fn capabilities_list() -> Vec<&'static str> {
    vec![
        "health.check",
        "capabilities.list",
        "fs.read",
        "fs.list",
        "code.search",
        "rust.check",
        "test.run",
        "build.run",
        "debug.quick_check",
        "debug.test_all",
        "debug.test_db",
        "debug.test_crypto",
        "debug.test_ssh",
        "debug.test_terminal",
        "debug.test_pro",
        "git.status",
        "git.diff",
        "git.log",
        "git.branch",
        "fs.write",
        "fs.edit",
        "context.set",
        "context.get",
        "context.clear",
    ]
}

fn param_string(params: &Value, key: &str) -> Result<String, WsError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| WsError {
            code: "invalid_params".to_string(),
            message: format!("缺少参数: {key}"),
            details: Some(params.clone()),
        })
}

fn serde_json_error(code: &str, err: serde_json::Error) -> WsError {
    WsError {
        code: code.to_string(),
        message: err.to_string(),
        details: None,
    }
}
