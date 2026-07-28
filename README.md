# EasySSH

EasySSH is a modern cross-platform session manager for system OpenSSH and the
system SSH Agent, including the 1Password SSH Agent. It opens sessions in the
platform terminal instead of embedding a terminal emulator.

It launches `ssh` for terminals and `scp` for file transfers. EasySSH does not
implement an SSH protocol, keep passwords, import private keys, invoke the
1Password CLI, or access credential vaults.

## Build

```powershell
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo run -p easyssh-desktop
```

Configure OpenSSH and your SSH Agent before opening EasySSH. On Windows the
app opens Windows Terminal when available, with PowerShell as the system
terminal fallback. Prefer an
OpenSSH config alias such as `production`; EasySSH will run `ssh production`.

## Security Boundary

- Connection metadata is stored in a local JSON file in the OS configuration directory.
- Child processes inherit the existing `PATH`, `HOME`, `USERPROFILE`, and `SSH_AUTH_SOCK` environment.
- The SSH Agent status page performs only `ssh -V`, `scp -V`, `ssh-add -l`, and optional `ssh -G` diagnostics.
- Agent fingerprints are transient UI output and are never written to configuration or logs.
