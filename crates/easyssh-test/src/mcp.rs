use easyssh_test::{
    app::{launch_ui_test, ManagedApp},
    inspect, run_with_cancellation, validate_options, workspace_root, CancellationToken,
    Diagnostic, ResultDocument, RunOptions,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum TaskState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Serialize)]
struct TaskSnapshot {
    task_id: String,
    operation: String,
    state: TaskState,
    started_at: Option<String>,
    finished_at: Option<String>,
    progress: String,
    summary: String,
    diagnostics: Vec<Diagnostic>,
    artifact_paths: Vec<String>,
}

struct Task {
    snapshot: TaskSnapshot,
    cancellation: CancellationToken,
}

#[derive(Default)]
struct Server {
    tasks: Mutex<HashMap<String, Task>>,
    app: Mutex<Option<ManagedApp>>,
    build_lock: Mutex<()>,
    sequence: AtomicU64,
}

fn main() {
    let root = match workspace_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("easyssh-mcp startup failed: {error}");
            return;
        }
    };
    let mut options = RunOptions::default_for_workspace(&root);
    if let Err(error) = validate_options(&root, &mut options) {
        eprintln!("easyssh-mcp startup failed: {error}");
        return;
    }
    let server = Arc::new(Server::default());
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle_request(&server, &request),
            Err(_) => error_response(Value::Null, -32700, "invalid JSON-RPC request"),
        };
        if let Ok(text) = serde_json::to_string(&response) {
            let _ = writeln!(writer, "{text}");
            let _ = writer.flush();
        }
    }
    shutdown(&server);
}

fn handle_request(server: &Arc<Server>, request: &Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return error_response(id, -32600, "request method is required");
    };
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion":"2024-11-05",
            "capabilities":{"tools":{}},
            "serverInfo":{"name":"easyssh-mcp","version":"0.4.0"}
        })),
        "notifications/initialized" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools()})),
        "tools/call" => call_tool(server, request.pointer("/params")),
        _ => Err((-32601, "method not allowed".into())),
    };
    match result {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err((code, message)) => error_response(id, code, &message),
    }
}

fn error_response(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

fn tools() -> Vec<Value> {
    vec![
        tool(
            "project_inspect",
            "Inspect the validated EasySSH workspace",
            json!({"type":"object","additionalProperties":false}),
        ),
        tool(
            "format_check",
            "Queue the allowlisted Cargo format check",
            json!({"type":"object","properties":{"timeout_seconds":{"type":"integer","minimum":1,"maximum":3600}},"additionalProperties":false}),
        ),
        tool(
            "run_clippy",
            "Queue allowlisted Cargo Clippy",
            json!({"type":"object","properties":{"timeout_seconds":{"type":"integer","minimum":1,"maximum":3600}},"additionalProperties":false}),
        ),
        tool(
            "run_unit_tests",
            "Queue allowlisted Cargo unit tests",
            json!({"type":"object","properties":{"timeout_seconds":{"type":"integer","minimum":1,"maximum":3600}},"additionalProperties":false}),
        ),
        tool(
            "build_app",
            "Queue an EasySSH build",
            json!({"type":"object","properties":{"profile":{"enum":["debug","release"]},"features":{"type":"array","items":{"type":"string","enum":[]},"maxItems":0},"timeout_seconds":{"type":"integer","minimum":1,"maximum":3600}},"required":["profile"],"additionalProperties":false}),
        ),
        tool(
            "get_task_status",
            "Get the summary and structured diagnostics for a queued task",
            json!({"type":"object","properties":{"task_id":{"type":"string","minLength":1}},"required":["task_id"],"additionalProperties":false}),
        ),
        tool(
            "cancel_task",
            "Cancel a task started by this MCP server",
            json!({"type":"object","properties":{"task_id":{"type":"string","minLength":1}},"required":["task_id"],"additionalProperties":false}),
        ),
        tool(
            "launch_app",
            "Launch an isolated feature-gated EasySSH UI test instance",
            json!({"type":"object","properties":{"timeout_seconds":{"type":"integer","minimum":1,"maximum":600}},"additionalProperties":false}),
        ),
        tool(
            "get_app_status",
            "Get status for the app launched by this MCP server",
            json!({"type":"object","additionalProperties":false}),
        ),
        tool(
            "get_app_logs",
            "Get the redacted UI test application log",
            json!({"type":"object","additionalProperties":false}),
        ),
        tool(
            "stop_app",
            "Gracefully stop the app launched by this MCP server",
            json!({"type":"object","properties":{"timeout_seconds":{"type":"integer","minimum":1,"maximum":60}},"additionalProperties":false}),
        ),
        tool(
            "get_ui_tree",
            "Get the stable-ID UI tree",
            json!({"type":"object","additionalProperties":false}),
        ),
        tool(
            "find_ui_element",
            "Find a UI element by stable ID",
            json!({"type":"object","properties":{"element_id":{"type":"string","minLength":1}},"required":["element_id"],"additionalProperties":false}),
        ),
        tool(
            "wait_for_ui_condition",
            "Wait for an ID-based UI condition",
            json!({"type":"object","properties":{"element_id":{"type":"string","minLength":1},"condition":{"enum":["exists","not_exists","visible","hidden","enabled","disabled","text_equals","text_contains","value_equals"]},"value":{"type":"string"},"timeout_seconds":{"type":"integer","minimum":1,"maximum":60}},"required":["element_id","condition"],"additionalProperties":false}),
        ),
        tool(
            "click_ui_element",
            "Click an enabled element by stable ID",
            json!({"type":"object","properties":{"element_id":{"type":"string","minLength":1}},"required":["element_id"],"additionalProperties":false}),
        ),
        tool(
            "type_into_ui_element",
            "Replace text through a stable element ID",
            json!({"type":"object","properties":{"element_id":{"type":"string","minLength":1},"text":{"type":"string"}},"required":["element_id","text"],"additionalProperties":false}),
        ),
        tool(
            "resize_app_window",
            "Resize the running test window",
            json!({"type":"object","properties":{"width":{"type":"integer","minimum":320},"height":{"type":"integer","minimum":320}},"required":["width","height"],"additionalProperties":false}),
        ),
        tool(
            "take_app_screenshot",
            "Capture only the EasySSH test window",
            json!({"type":"object","properties":{"name":{"type":"string","minLength":1,"maxLength":64}},"additionalProperties":false}),
        ),
    ]
}

fn tool(name: &str, description: &str, schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":schema})
}

fn call_tool(server: &Arc<Server>, params: Option<&Value>) -> Result<Value, (i32, String)> {
    let params = params.ok_or((-32602, "tool parameters are required".into()))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "tool name is required".into()))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !arguments.is_object() {
        return Err((-32602, "tool arguments must be an object".into()));
    }
    match name {
        "project_inspect" => {
            reject_unknown(&arguments, &[])?;
            let root = workspace_root().map_err(internal)?;
            let mut options = RunOptions::default_for_workspace(&root);
            validate_options(&root, &mut options).map_err(internal)?;
            Ok(tool_result(inspect(&options).map_err(internal)?))
        }
        "format_check" | "run_clippy" | "run_unit_tests" | "build_app" => {
            let operation = match name {
                "format_check" => "format_check",
                "run_clippy" => "clippy",
                "run_unit_tests" => "unit_tests",
                _ => match arguments.get("profile").and_then(Value::as_str) {
                    Some("debug") => "build_debug",
                    Some("release") => "build_release",
                    _ => return Err((-32602, "build_app.profile must be debug or release".into())),
                },
            };
            let allowed = if name == "build_app" {
                ["profile", "features", "timeout_seconds"].as_slice()
            } else {
                ["timeout_seconds"].as_slice()
            };
            reject_unknown(&arguments, allowed)?;
            if arguments.get("features").is_some_and(|value| {
                value
                    .as_array()
                    .map(|items| !items.is_empty())
                    .unwrap_or(true)
            }) {
                return Err((
                    -32602,
                    "features are not supported by this workspace".into(),
                ));
            }
            let timeout = parse_timeout(&arguments)?;
            Ok(task_result(enqueue(server, operation, timeout)))
        }
        "get_task_status" => {
            reject_unknown(&arguments, &["task_id"])?;
            let task_id = arguments
                .get("task_id")
                .and_then(Value::as_str)
                .ok_or((-32602, "task_id is required".into()))?;
            let tasks = server
                .tasks
                .lock()
                .map_err(|_| (-32603, "task manager lock failed".into()))?;
            let task = tasks
                .get(task_id)
                .ok_or((-32602, "unknown task_id".into()))?;
            Ok(task_result(task.snapshot.clone()))
        }
        "cancel_task" => {
            reject_unknown(&arguments, &["task_id"])?;
            let task_id = arguments
                .get("task_id")
                .and_then(Value::as_str)
                .ok_or((-32602, "task_id is required".into()))?;
            let mut tasks = server
                .tasks
                .lock()
                .map_err(|_| (-32603, "task manager lock failed".into()))?;
            let task = tasks
                .get_mut(task_id)
                .ok_or((-32602, "unknown task_id".into()))?;
            match task.snapshot.state {
                TaskState::Queued | TaskState::Running => {
                    task.cancellation.cancel();
                    task.snapshot.progress = "cancellation requested".into();
                }
                _ => return Err((-32602, "task is already finished".into())),
            }
            Ok(task_result(task.snapshot.clone()))
        }
        "launch_app" => {
            reject_unknown(&arguments, &["timeout_seconds"])?;
            let root = workspace_root().map_err(internal)?;
            let timeout = parse_timeout(&arguments)?.min(Duration::from_secs(600));
            let mut app = server
                .app
                .lock()
                .map_err(|_| (-32603, "app manager lock failed".into()))?;
            if let Some(existing) = app.as_mut() {
                if matches!(
                    existing.get_status().map_err(internal)?.state.as_str(),
                    "starting" | "ready"
                ) {
                    return Err((
                        -32602,
                        "an app is already managed by this MCP server".into(),
                    ));
                }
                *app = None;
            }
            let launched = launch_ui_test(&root, timeout).map_err(internal)?;
            let result = launched.status.clone();
            *app = Some(launched);
            Ok(
                json!({"isError":!result.success,"content":[{"type":"text","text":result.summary}],"structuredContent":result}),
            )
        }
        "get_app_status" => {
            reject_unknown(&arguments, &[])?;
            let mut app = server
                .app
                .lock()
                .map_err(|_| (-32603, "app manager lock failed".into()))?;
            let result = app
                .as_mut()
                .ok_or((-32602, "no app launched by this MCP server".into()))?
                .get_status()
                .map_err(internal)?;
            Ok(
                json!({"isError":!result.success,"content":[{"type":"text","text":result.summary}],"structuredContent":result}),
            )
        }
        "get_app_logs" => {
            reject_unknown(&arguments, &[])?;
            let app = server
                .app
                .lock()
                .map_err(|_| (-32603, "app manager lock failed".into()))?;
            let status = app
                .as_ref()
                .ok_or((-32602, "no app launched by this MCP server".into()))?
                .status
                .clone();
            let root = workspace_root().map_err(internal)?;
            let log = root.join(status.log_file.replace('/', "\\"));
            let text = std::fs::read_to_string(log).unwrap_or_default();
            Ok(
                json!({"content":[{"type":"text","text":easyssh_test::redact(&text)}],"structuredContent":{"log_file":status.log_file}}),
            )
        }
        "stop_app" => {
            reject_unknown(&arguments, &["timeout_seconds"])?;
            let timeout = parse_timeout(&arguments)?.min(Duration::from_secs(60));
            let mut app = server
                .app
                .lock()
                .map_err(|_| (-32603, "app manager lock failed".into()))?;
            let result = app
                .as_mut()
                .ok_or((-32602, "no app launched by this MCP server".into()))?
                .stop(timeout)
                .map_err(internal)?;
            Ok(
                json!({"isError":!result.success,"content":[{"type":"text","text":result.summary}],"structuredContent":result}),
            )
        }
        "get_ui_tree" | "click_ui_element" | "type_into_ui_element" | "resize_app_window" => {
            let allowed = match name {
                "get_ui_tree" => &[][..],
                "click_ui_element" => &["element_id"][..],
                "type_into_ui_element" => &["element_id", "text"][..],
                _ => &["width", "height"][..],
            };
            reject_unknown(&arguments, allowed)?;
            let mut app = server
                .app
                .lock()
                .map_err(|_| (-32603, "app manager lock failed".into()))?;
            let app = app
                .as_mut()
                .ok_or((-32602, "no app launched by this MCP server".into()))?;
            let request = match name {
                "get_ui_tree" => json!({"operation":"get_ui_tree"}),
                "click_ui_element" => {
                    json!({"operation":"click","element_id":arguments["element_id"]})
                }
                "type_into_ui_element" => {
                    json!({"operation":"type","element_id":arguments["element_id"],"text":arguments["text"]})
                }
                _ => {
                    json!({"operation":"resize","width":arguments["width"],"height":arguments["height"]})
                }
            };
            let result = app
                .bridge(request, Duration::from_secs(10))
                .map_err(internal)?;
            Ok(
                json!({"isError":!result["success"].as_bool().unwrap_or(false),"content":[{"type":"text","text":result["error"].as_str().unwrap_or("UI bridge completed")}],"structuredContent":result}),
            )
        }
        "find_ui_element" => {
            reject_unknown(&arguments, &["element_id"])?;
            let id = arguments["element_id"]
                .as_str()
                .ok_or((-32602, "element_id is required".into()))?;
            let tree = request_ui_tree(server)?;
            let element = find_element(&tree, id).ok_or((-32602, "UI element not found".into()))?;
            Ok(
                json!({"content":[{"type":"text","text":"UI element found"}],"structuredContent":element}),
            )
        }
        "wait_for_ui_condition" => {
            reject_unknown(
                &arguments,
                &["element_id", "condition", "value", "timeout_seconds"],
            )?;
            let id = arguments["element_id"]
                .as_str()
                .ok_or((-32602, "element_id is required".into()))?;
            let condition = arguments["condition"]
                .as_str()
                .ok_or((-32602, "condition is required".into()))?;
            let value = arguments["value"].as_str();
            let timeout = parse_timeout(&arguments)?.min(Duration::from_secs(60));
            let started = std::time::Instant::now();
            loop {
                let tree = request_ui_tree(server)?;
                let element = find_element(&tree, id);
                if ui_condition(element, condition, value) {
                    return Ok(
                        json!({"content":[{"type":"text","text":"UI condition met"}],"structuredContent":element}),
                    );
                }
                if started.elapsed() >= timeout {
                    return Err((-32602, "UI condition timed out".into()));
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
        "take_app_screenshot" => {
            reject_unknown(&arguments, &["name"])?;
            let mut app = server
                .app
                .lock()
                .map_err(|_| (-32603, "app manager lock failed".into()))?;
            let name = arguments["name"].as_str().unwrap_or("window");
            if !name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
            {
                return Err((
                    -32602,
                    "screenshot name contains unsupported characters".into(),
                ));
            }
            let path = app
                .as_mut()
                .ok_or((-32602, "no app launched by this MCP server".into()))?
                .take_screenshot(name, Duration::from_secs(10))
                .map_err(internal)?;
            Ok(
                json!({"content":[{"type":"text","text":"window screenshot captured"}],"structuredContent":{"path":path,"format":"png"}}),
            )
        }
        _ => Err((-32601, "tool not allowed".into())),
    }
}

fn enqueue(server: &Arc<Server>, operation: &str, timeout: Duration) -> TaskSnapshot {
    let operation = operation.to_owned();
    let task_id = format!(
        "task-{}",
        server.sequence.fetch_add(1, Ordering::SeqCst) + 1
    );
    let snapshot = TaskSnapshot {
        task_id: task_id.clone(),
        operation: operation.clone(),
        state: TaskState::Queued,
        started_at: None,
        finished_at: None,
        progress: "queued".into(),
        summary: String::new(),
        diagnostics: vec![],
        artifact_paths: vec![],
    };
    let cancellation = CancellationToken::default();
    server.tasks.lock().expect("task manager lock").insert(
        task_id.clone(),
        Task {
            snapshot: snapshot.clone(),
            cancellation: cancellation.clone(),
        },
    );
    let server = Arc::clone(server);
    thread::spawn(move || {
        let _build_guard = server.build_lock.lock().ok();
        if cancellation.is_cancelled() {
            update_task(&server, &task_id, |task| {
                task.snapshot.state = TaskState::Cancelled;
                task.snapshot.finished_at = Some(chrono::Utc::now().to_rfc3339());
                task.snapshot.progress = "finished".into();
                task.snapshot.summary = "task cancelled before execution".into();
            });
            return;
        }
        update_task(&server, &task_id, |task| {
            task.snapshot.state = TaskState::Running;
            task.snapshot.started_at = Some(chrono::Utc::now().to_rfc3339());
            task.snapshot.progress = "running".into();
        });
        let root = workspace_root();
        let result = root.and_then(|root| {
            let mut options = RunOptions::default_for_workspace(&root);
            options.timeout = timeout;
            validate_options(&root, &mut options)?;
            run_with_cancellation(&operation, &options, Some(&cancellation))
        });
        update_task(&server, &task_id, |task| {
            task.snapshot.finished_at = Some(chrono::Utc::now().to_rfc3339());
            match result {
                Ok(document) => finish_from_document(task, document),
                Err(error) => {
                    task.snapshot.state = TaskState::Failed;
                    task.snapshot.progress = "finished".into();
                    task.snapshot.summary = error.to_string();
                }
            }
        });
    });
    snapshot
}

fn request_ui_tree(server: &Arc<Server>) -> Result<Value, (i32, String)> {
    let mut app = server
        .app
        .lock()
        .map_err(|_| (-32603, "app manager lock failed".into()))?;
    let result = app
        .as_mut()
        .ok_or((-32602, "no app launched by this MCP server".into()))?
        .bridge(json!({"operation":"get_ui_tree"}), Duration::from_secs(10))
        .map_err(internal)?;
    result["tree"]
        .clone()
        .as_object()
        .map(|_| result["tree"].clone())
        .ok_or((-32603, "UI bridge returned no tree".into()))
}

fn find_element<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
    if node["id"].as_str() == Some(id) {
        return Some(node);
    }
    node["children"]
        .as_array()?
        .iter()
        .find_map(|child| find_element(child, id))
}

fn ui_condition(element: Option<&Value>, condition: &str, value: Option<&str>) -> bool {
    match condition {
        "exists" => element.is_some(),
        "not_exists" => element.is_none(),
        "visible" => element.and_then(|item| item["visible"].as_bool()) == Some(true),
        "hidden" => element.and_then(|item| item["visible"].as_bool()) == Some(false),
        "enabled" => element.and_then(|item| item["enabled"].as_bool()) == Some(true),
        "disabled" => element.and_then(|item| item["enabled"].as_bool()) == Some(false),
        "text_equals" => element.and_then(|item| item["text"].as_str()) == value,
        "text_contains" => element
            .and_then(|item| item["text"].as_str())
            .zip(value)
            .is_some_and(|(text, expected)| text.contains(expected)),
        "value_equals" => element.and_then(|item| item["value"].as_str()) == value,
        _ => false,
    }
}

fn shutdown(server: &Server) {
    if let Ok(tasks) = server.tasks.lock() {
        for task in tasks.values() {
            task.cancellation.cancel();
        }
    }
    for _ in 0..100 {
        let active = server
            .tasks
            .lock()
            .map(|tasks| {
                tasks.values().any(|task| {
                    matches!(task.snapshot.state, TaskState::Queued | TaskState::Running)
                })
            })
            .unwrap_or(false);
        if !active {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    if let Ok(mut app) = server.app.lock() {
        if let Some(app) = app.as_mut() {
            let _ = app.stop(Duration::from_secs(2));
        }
    }
}

fn finish_from_document(task: &mut Task, document: ResultDocument) {
    task.snapshot.state = match document.exit_code {
        0 => TaskState::Completed,
        124 => TaskState::TimedOut,
        130 => TaskState::Cancelled,
        _ => TaskState::Failed,
    };
    task.snapshot.progress = "finished".into();
    task.snapshot.summary = document.summary;
    task.snapshot.diagnostics = document.diagnostics;
    task.snapshot.artifact_paths = document.artifacts;
}

fn update_task(server: &Server, task_id: &str, update: impl FnOnce(&mut Task)) {
    if let Ok(mut tasks) = server.tasks.lock() {
        if let Some(task) = tasks.get_mut(task_id) {
            update(task);
        }
    }
}

fn reject_unknown(arguments: &Value, allowed: &[&str]) -> Result<(), (i32, String)> {
    for key in arguments.as_object().expect("arguments object").keys() {
        if !allowed.contains(&key.as_str()) {
            return Err((-32602, format!("unsupported argument: {key}")));
        }
    }
    Ok(())
}

fn parse_timeout(arguments: &Value) -> Result<Duration, (i32, String)> {
    match arguments.get("timeout_seconds").and_then(Value::as_u64) {
        Some(seconds) if (1..=3600).contains(&seconds) => Ok(Duration::from_secs(seconds)),
        Some(_) => Err((-32602, "timeout_seconds must be between 1 and 3600".into())),
        None => Ok(Duration::from_secs(600)),
    }
}

fn tool_result(value: ResultDocument) -> Value {
    json!({"isError":!value.success,"content":[{"type":"text","text":value.summary}],"structuredContent":value})
}
fn task_result(value: TaskSnapshot) -> Value {
    json!({"content":[{"type":"text","text":value.summary}],"structuredContent":value})
}
fn internal(error: anyhow::Error) -> (i32, String) {
    (-32603, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_discovery_exposes_p1_to_p3_surface() {
        let listed_tools = tools();
        let names = listed_tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "project_inspect",
                "format_check",
                "run_clippy",
                "run_unit_tests",
                "build_app",
                "get_task_status",
                "cancel_task",
                "launch_app",
                "get_app_status",
                "get_app_logs",
                "stop_app",
                "get_ui_tree",
                "find_ui_element",
                "wait_for_ui_condition",
                "click_ui_element",
                "type_into_ui_element",
                "resize_app_window",
                "take_app_screenshot"
            ]
        );
    }

    #[test]
    fn rejects_unsafe_build_arguments() {
        let server = Arc::new(Server::default());
        let response = call_tool(
            &server,
            Some(
                &json!({"name":"build_app","arguments":{"profile":"debug","cargo_args":"--evil"}}),
            ),
        );
        assert!(response.is_err());
    }

    #[test]
    fn unknown_task_cannot_be_cancelled() {
        let server = Arc::new(Server::default());
        let response = call_tool(
            &server,
            Some(&json!({"name":"cancel_task","arguments":{"task_id":"not-created"}})),
        );
        assert!(response.is_err());
    }
}
