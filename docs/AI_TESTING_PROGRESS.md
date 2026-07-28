# AI Testing Progress

## P0

- Status: verified
- Completed work:
  - Audited workspace, toolchain, native UI, transfer modules, persistence,
    external dependencies, tests, CI, platform behavior, and automation gap.
  - Implemented the `easyssh-test` allowlisted CLI with JSON result documents,
    workspace-contained artifacts, redaction, Cargo JSON diagnostics, timeout
    termination, and unit tests for compiler-failure diagnostics, redaction,
    timeout termination, and artifact path isolation.
- Commands executed:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features`
  - `cargo build --workspace`
  - `cargo build --workspace --release`
  - `cargo run -p easyssh-test -- fmt`
  - `cargo run -p easyssh-test -- clippy`
  - `cargo run -p easyssh-test -- test`
  - `cargo run -p easyssh-test -- build --profile release`
  - `cargo run -p easyssh-test -- build --profile debug`
- Test results: all five required raw Cargo commands passed after the final
  change. The complete P0 wrapper sequence passed; workspace tests include four
  `easyssh-test` tests for fixture diagnostics, redaction, timeout termination,
  and path isolation.
- Artifacts: `artifacts/*.log` (redacted raw Cargo output).
- Known limitations: UI automation, test-mode application lifecycle, isolated
  SSH, visual regression, and full MCP task management are not P0 deliverables.

## P1

- Status: verified
- Completed work: stdio JSON-RPC server with strict schemas; seven P1 tools;
  queued task state, cancellation, timeout, serialized Cargo execution,
  structured diagnostics, stdout isolation, and graceful EOF cancellation.
- Commands executed: `cargo test -p easyssh-test`; direct `easyssh-mcp.exe`
  stdio tests for discovery, queued formatting, status polling, invalid input,
  cancellation, and shutdown; complete P0 Rust verification sequence.
- Test results: seven `easyssh-test` tests passed. Direct MCP task completion
  and cancellation both passed with empty server stderr.
- Artifacts: redacted task logs under `artifacts/`.
- Known limitations: native UI lifecycle and UI automation begin at P2.

## P2

- Status: verified
- Completed work: `ui-test` feature, isolated run directories, isolated config,
  feature-gated ready metadata, app logs, token validation, graceful stop
  request, MCP application lifecycle tools, and child-only cleanup.
- Commands executed: feature-on and feature-off desktop checks; direct MCP
  launch/status/log/stop session; ten consecutive launch/stop cycles; complete
  P0 Rust verification sequence.
- Test results: all ten native cycles returned `ready/stopped`; ready metadata
  reported `EasySSH [UI Test]` at 1280x800; no MCP stderr output.
- Artifacts: `artifacts/runs/run-*/` with config, data, logs, screenshots, and
  metadata directories/files.
- Known limitations: native UI tree, input, and screenshots are P3 work.

## P3

- Status: not_started

## P4

- Status: not_started

## P5

- Status: not_started

## Blockers

- None. A Windows self-binary locking issue was repaired by using a controlled,
  repository-local target directory for Cargo child processes.

## Next action

- Commit the verified P2 lifecycle, then implement P3 UI Test Bridge.
