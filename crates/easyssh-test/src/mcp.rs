use easyssh_test::{inspect, run, validate_options, workspace_root, ResultDocument, RunOptions};
use serde_json::{json, Value};
use std::io::{self, BufRead};

fn main() {
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let result = match method {
            "initialize" => {
                json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"easyssh-mcp","version":"0.4.0"}})
            }
            "tools/list" => json!({"tools":[
                tool("project_inspect","Inspect the EasySSH workspace",json!({})),
                tool("format_check","Run cargo fmt --check",json!({})),
                tool("run_clippy","Run allowlisted cargo clippy",json!({})),
                tool("run_unit_tests","Run allowlisted cargo test",json!({})),
                tool("build_app","Build debug or release application",json!({"type":"object","properties":{"profile":{"enum":["debug","release"]}},"additionalProperties":false}))
            ]}),
            "tools/call" => call(&request),
            _ => json!({"error":{"code":-32601,"message":"method not allowed"}}),
        };
        println!("{}", json!({"jsonrpc":"2.0","id":id,"result":result}));
    }
}

fn tool(name: &str, description: &str, schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":schema})
}
fn call(request: &Value) -> Value {
    let name = request
        .pointer("/params/name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let document: anyhow::Result<ResultDocument> = match name {
        "project_inspect" => with_default_options(inspect),
        "format_check" => with_default_options(|options| run("format_check", options)),
        "run_clippy" => with_default_options(|options| run("clippy", options)),
        "run_unit_tests" => with_default_options(|options| run("unit_tests", options)),
        "build_app" => {
            if request
                .pointer("/params/arguments/profile")
                .and_then(Value::as_str)
                == Some("release")
            {
                with_default_options(|options| run("build_release", options))
            } else {
                with_default_options(|options| run("build_debug", options))
            }
        }
        _ => {
            return json!({"isError":true,"content":[{"type":"text","text":"configuration_error: tool not allowed"}]})
        }
    };
    match document {
        Ok(value) => {
            json!({"isError":!value.success,"content":[{"type":"text","text":serde_json::to_string(&value).unwrap()}],"structuredContent":value})
        }
        Err(error) => {
            json!({"isError":true,"content":[{"type":"text","text":format!("internal_error: {error}")}]})
        }
    }
}

fn with_default_options(
    operation: impl FnOnce(&RunOptions) -> anyhow::Result<ResultDocument>,
) -> anyhow::Result<ResultDocument> {
    let root = workspace_root()?;
    let mut options = RunOptions::default_for_workspace(&root);
    validate_options(&root, &mut options)?;
    operation(&options)
}
