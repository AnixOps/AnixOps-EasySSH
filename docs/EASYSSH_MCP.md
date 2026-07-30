# EasySSH MCP

Build the local tools:

```powershell
$env:CARGO_TARGET_DIR = "target/standard"
cargo build -p easyssh-test --release --bin easyssh-mcp
```

The server uses stdio and exposes only allowlisted project operations. It has no arbitrary command, path deletion, or arbitrary-host connection tool.

Codex configuration:

```json
{"mcp_servers":{"easyssh":{"command":"C:\\path\\to\\AnixOps-EasySSH\\target\\standard\\release\\easyssh-mcp.exe","args":[]}}}
```

Claude Code configuration:

```json
{"mcpServers":{"easyssh":{"command":"C:\\path\\to\\AnixOps-EasySSH\\target\\standard\\release\\easyssh-mcp.exe","args":[]}}}
```

The release binary is `target/standard/release/easyssh-mcp.exe` when the
repository `.cargo/config.toml` is used. The debug binary is
`target/standard/debug/easyssh-mcp.exe`. The project verification wrapper uses
its own child target directory, so a wrapper-built release MCP binary is at
`target/easyssh-test-run/release/easyssh-mcp.exe`; use the direct build command
above for the configuration paths in this document.

P1 tools are `project_inspect`, `format_check`, `run_clippy`,
`run_unit_tests`, `build_app`, `get_task_status`, and `cancel_task`.
Build and test operations return a task identifier; poll it with
`get_task_status` and cancel only that task with `cancel_task`. `build_app`
accepts only `debug` or `release`; arbitrary Cargo flags, command paths, host
names, and filesystem paths are not accepted. The P2/P3 UI tools are
`launch_app`, `get_app_status`, `get_app_logs`, `stop_app`, `get_ui_tree`,
`find_ui_element`, `wait_for_ui_condition`, `click_ui_element`,
`type_into_ui_element`, `resize_app_window`, `take_app_screenshot`,
`set_ui_locale`, `set_ui_workspace`, `show_ui_toast`, and
`dismiss_ui_toast`. They operate only on an isolated `ui-test` instance.

Run the standalone CLI with `cargo run -p easyssh-test -- inspect`, `fmt`, `clippy`, `test`, or `build --profile release`. Logs are written under `artifacts/` and redact lines containing password, private-key or token markers.
