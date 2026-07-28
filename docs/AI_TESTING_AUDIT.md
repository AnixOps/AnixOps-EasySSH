# AI Testing Audit

Audit date: 2026-07-28. This document describes the repository as inspected on
Windows; macOS and Linux behavior has not been executed locally.

## Workspace and toolchain

- Cargo workspace resolver `2`, edition 2021, declared MSRV Rust 1.75.
- Local verification used `rustc 1.89.0` and Cargo 1.89.0.
- Crates: `easyssh-core` library, `easyssh-desktop` (`easyssh` native binary),
  and `easyssh-test` (`easyssh-test` and preliminary `easyssh-mcp` binaries).
- No crate declares Cargo features today. Workspace targets are the three
  libraries/binaries above; desktop and MCP binaries have no doctests.
- No `build.rs`, system library build script, database migration, or checked-in
  external binary was found. Runtime dependencies include system OpenSSH
  (`ssh`, `scp`, `sftp`, optionally `ssh-add`) and Git for sync actions.

## Native application

- `easyssh-desktop` uses `eframe 0.29` and immediate-mode `egui 0.29`, rendered
  with the `glow` backend. Its entry point and native window lifecycle are in
  `crates/easyssh-desktop/src/main.rs`.
- UI modules live in `crates/easyssh-desktop/src/ui/`. Navigation selects the
  `Workspace` enum; there is no web route, DOM, or HTTP layer.
- Transfers are rendered by `EasySshApp::transfers` in `main.rs`. The core SCP
  and batch SFTP invocation builders are in `crates/easyssh-core/src/transfer.rs`.
- The application has no async runtime. Transfers use child processes and
  standard-library threads/channel-style polling; the UI checks child status
  during egui updates.
- There is no accessibility export, system accessibility adapter, widget ID
  registry, screenshot test bridge, or native UI automation interface. P2/P3
  must add an explicitly feature-gated implementation rather than claim it
  exists.

## Data, security, and logging

- Persistent connection metadata is JSON through `ConfigStore` in
  `crates/easyssh-core/src/config.rs`, normally under the OS configuration
  directory via `dirs`. No database is used.
- Passwords and key material are intentionally absent from the persisted domain
  schema. Authentication is delegated to system OpenSSH and its agent.
- Core validation and command construction are in `security.rs` and
  `openssh.rs`. The current desktop application can use the user's configured
  OpenSSH environment; P2/P4 must never do so in test mode.
- There is no centralized structured application log sink. The P0 CLI writes
  redacted command logs only below repository `artifacts/`.

## Tests, scripts, and CI

- Existing unit tests cover configuration migration/schema filtering, OpenSSH
  invocation, sync validation, transfer command construction, and two desktop
  helpers. P0 adds CLI unit tests for Cargo JSON diagnostics, redaction, and
  timeout termination.
- `.github/workflows/ci.yml` targets Windows, macOS, and Ubuntu and invokes the
  P0 CLI for format, Clippy, tests, and release build. No UI test job exists.
- Standard Cargo validation is supported unchanged:
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo test --workspace --all-features`, `cargo build --workspace`, and
  `cargo build --workspace --release`.

## Platform notes

- Windows was verified locally. The CLI builds child Cargo commands in
  `target/easyssh-test-run` because Windows locks the currently executing test
  binary; this does not change application build inputs.
- Linux has `x11` and `wayland` eframe features enabled and launches an external
  terminal via `x-terminal-emulator` where applicable. It was not locally run.
- macOS terminal launch is intentionally unsupported in the current code. It
  was not locally run.
- Native UI capture/input requires platform-specific work and is not available
  at P0.

## P0 test entry point and safety boundary

`easyssh-test` accepts only `inspect`, `fmt`, `clippy`, `test`, and `build`
with controlled `--profile debug|release`, `--json`, `--timeout <seconds>`, and
`--artifact-dir <path>` options. It resolves the compile-time workspace root,
requires artifact directories to remain under that root, invokes only the
literal `cargo` executable with allowlisted argument vectors, and never accepts
a shell command, executable path, host, private key, or user configuration
path. Cargo JSON messages are parsed for diagnostics and raw redacted logs are
saved under the requested repository artifact directory.

Timeout handling terminates the controlled Cargo child. On Windows it uses the
fixed `taskkill /PID <controlled-pid> /T /F` invocation to also terminate that
child's descendants; it is not caller-configurable. P0 does not launch the
desktop application or connect to any host.
