# EasySSH MCP

Build the local tools:

```powershell
cargo build -p easyssh-test --release
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

P1 tools are `project_inspect`, `format_check`, `run_clippy`,
`run_unit_tests`, `build_app`, `get_task_status`, and `cancel_task`.
Build and test operations return a task identifier; poll it with
`get_task_status` and cancel only that task with `cancel_task`. `build_app`
accepts only `debug` or `release`; arbitrary Cargo flags, command paths, host
names, and filesystem paths are not accepted. UI automation, screenshots and
SSH/SFTP test-server tools do not exist until their later phases are backed by
real implementations.

Run the standalone CLI with `cargo run -p easyssh-test -- inspect`, `fmt`, `clippy`, `test`, or `build --profile release`. Logs are written under `artifacts/` and redact lines containing password, private-key or token markers.
