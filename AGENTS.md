# EasySSH Agent Verification

Do not connect to production hosts, use user configuration, run arbitrary shell commands through MCP, or update visual baselines automatically.

After Rust changes run, in order:

1. `cargo run -p easyssh-test -- fmt`
2. `cargo run -p easyssh-test -- clippy`
3. `cargo run -p easyssh-test -- test`
4. `cargo run -p easyssh-test -- build --profile release`

After native UI changes, additionally use the future UI-test bridge to launch an isolated instance, inspect its UI tree, execute affected interactions, capture required theme/window screenshots, inspect logs, and stop the test instance. Do not claim UI automation passed while the bridge is unavailable.

On failure, fix the root cause and rerun the failed operation, then run the complete sequence. Stop after five automatic repair attempts and report the evidence.
