# Troubleshooting Guide / 故障排查指南

> Common issues and solutions quick reference / 常见问题诊断和解决方案速查手册

**[English](#english) | [中文](#中文)**

---

# English

## Table of Contents

1. [Build Issues](#build-issues)
2. [Runtime Errors](#runtime-errors)
3. [Connection Issues](#connection-issues)
4. [Performance Issues](#performance-issues)
5. [Platform-Specific Issues](#platform-specific-issues)
6. [Database Issues](#database-issues)
7. [Security Issues](#security-issues)
8. [Debug Tools](#debug-tools)
9. [Quick Fix Checklist](#quick-fix-checklist)

---

## Build Issues

### Rust Compilation Errors

#### Error: `link.exe` not found (Windows)

```powershell
# Symptom
error: linker `link.exe` not found

# Solutions
# 1. Install Visual Studio Build Tools
winget install Microsoft.VisualStudio.2022.BuildTools

# 2. Use rustup MSVC toolchain
rustup default stable-x86_64-pc-windows-msvc

# 3. Run in VS Developer Command Prompt
cargo build
```

#### Error: OpenSSL link failure

```bash
# Symptom
error: failed to run custom build command for `openssl-sys`

# Linux solution
sudo apt-get install libssl-dev pkg-config

# macOS solution
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)

# Windows solution (use vendored)
# Add to Cargo.toml:
# [dependencies]
# openssl = { version = "0.10", features = ["vendored"] }
```

#### Error: SQLite binding failure

```bash
# Symptom
error: failed to run custom build command for `libsqlite3-sys`

# Solution 1: Use bundled SQLite
cargo build --features bundled-sqlite

# Solution 2: Install system SQLite
# Ubuntu/Debian
sudo apt-get install libsqlite3-dev

# Fedora
sudo dnf install sqlite-devel

# macOS
brew install sqlite3
export SQLITE3_DIR=$(brew --prefix sqlite)
```

#### Error: GTK4 compilation failure (Linux)

```bash
# Symptom
error: failed to run custom build command for `gtk4-sys`

# Install GTK4 dependencies
# Ubuntu 22.04+
sudo apt-get install libgtk-4-dev libadwaita-1-dev

# Fedora
sudo dnf install gtk4-devel libadwaita-devel

# Arch
sudo pacman -S gtk4 libadwaita

# Verify installation
pkg-config --modversion gtk4
pkg-config --modversion libadwaita-1
```

### Dependency Conflicts

```bash
# Symptom
error: failed to select a version for the requirement

# Solutions
# 1: Update dependencies
cargo update

# 2: Check conflicts
cargo tree -d  # View duplicate dependencies

# 3: Use unified versions
# In workspace Cargo.toml:
[workspace.dependencies]
serde = "1.0"
tokio = "1.0"
```

### Feature Conflicts

```bash
# Symptom
error: the feature `X` is not enabled

# Solutions
cargo build --features "feature-a feature-b"

# Or enable default features in Cargo.toml
[features]
default = ["feature-a", "feature-b"]
```

---

## Runtime Errors

### Startup Failures

#### GTK4 theme loading failure

```bash
# Symptom
Gtk-WARNING **: Theme parsing error

# Solutions
# 1: Set default theme
export GTK_THEME=Adwaita:dark

# 2: Install theme
sudo apt-get install gnome-themes-extra

# 3: Check theme files
ls /usr/share/themes/
```

#### Database initialization failure

```rust
// Symptom
Error: Failed to initialize database

// Solution: Check database directory permissions
// 1. Create directory
mkdir -p ~/.local/share/easyssh

// 2. Check permissions
ls -la ~/.local/share/easyssh/

// 3. Fix permissions
chmod 700 ~/.local/share/easyssh
```

#### Keychain access failure

```bash
# Linux symptom
Error: Platform secure storage failure

# Solutions
# 1: Start secret service
# GNOME
eval $(gnome-keyring-daemon --start)
export SSH_AUTH_SOCK

# 2: Use keyutils
sudo apt-get install keyutils

# 3: Configure alternative keyring
# In config file:
# keyring_backend = "file"
```

### Crashes and Panics

#### Segmentation Fault

```bash
# Collect crash info
RUST_BACKTRACE=1 cargo run 2>&1 | tee crash.log

# Use debugger
gdb ./target/debug/easyssh-gtk4
(gdb) run
(gdb) bt  # Get backtrace

# Use AddressSanitizer
RUSTFLAGS="-Zsanitizer=address" cargo run
```

#### Out of Memory (OOM)

```bash
# Symptom
fatal runtime error: memory allocation failed

# Solutions
# 1: Limit concurrent compilation
cargo build -j 1

# 2: Increase swap
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# 3: Use release mode (less debug info)
cargo run --release
```

---

## Connection Issues

### SSH Connection Failures

#### Connection timeout

```rust
// Symptom
Error: Connection timed out

// Diagnostic steps
// 1. Check network connectivity
ping <host>
nc -zv <host> <port>

// 2. Check firewall
sudo iptables -L | grep <port>

// 3. Increase timeout
let config = SshConfig {
    connection_timeout: Duration::from_secs(30),
    ..Default::default()
};
```

#### Authentication failure

```rust
// Symptom
Error: Authentication failed

// Common causes and solutions

// 1. Wrong password
// Verify password
echo "password" | ssh user@host  // Test

// 2. Key permissions (Unix)
chmod 600 ~/.ssh/id_rsa
chmod 700 ~/.ssh

// 3. Key format issue
// Convert key format
ssh-keygen -p -m PEM -f ~/.ssh/id_rsa

// 4. SSH Agent issue
echo $SSH_AUTH_SOCK
ssh-add -l  // List loaded keys
ssh-add ~/.ssh/id_rsa  // Add key
```

#### Host key verification failure

```bash
# Symptom
Error: Host key verification failed

# Solutions
# 1: Update known_hosts
ssh-keygen -R <hostname>
ssh-keygen -R <ip_address>

# 2: Disable in dev (NOT for production)
let config = SshConfig {
    strict_host_key_checking: false,  // Dev only!
    ..Default::default()
};

// 3: Auto-accept new keys
let config = SshConfig {
    auto_accept_new_keys: true,  // Auto-add on first connect
    ..Default::default()
};
```

### SFTP Transfer Issues

#### Transfer interruption

```rust
// Symptom
Error: SFTP transfer interrupted

// Solution: Enable resume
let sftp_config = SftpConfig {
    resume_enabled: true,
    chunk_size: 1024 * 1024,  // 1MB chunks
    retry_attempts: 3,
    retry_delay: Duration::from_secs(1),
};
```

#### Permission denied

```bash
# Symptom
Error: Permission denied during SFTP

# Check remote permissions
ls -la /remote/path

# Check local permissions
ls -la /local/path

// Use correct permission mask
let sftp_config = SftpConfig {
    umask: 0o022,
    ..Default::default()
};
```

---

## Performance Issues

### High CPU Usage

```bash
# Diagnose
# 1. Find consuming thread
htop -p $(pgrep easyssh)

# 2. Get Rust backtrace
cargo install perf
sudo perf record -g ./target/release/easyssh-gtk4
sudo perf report

// Common causes and solutions

// 1. Polling loop
tokio::time::sleep(Duration::from_millis(10)).await;  // Add delay

// 2. Infinite recursion
// Check recursive functions, add depth limit

// 3. Render loop
// Limit update frequency
```

### Memory Leaks

```bash
# Diagnostic tools
# 1. Valgrind
valgrind --leak-check=full --show-leak-kinds=all ./target/debug/easyssh-gtk4

# 2. Heaptrack
heaptrack ./target/debug/easyssh-gtk4
heaptrack_gui heaptrack.easyssh-gtk4.*.gz

// Common memory leak causes

// 1. Forgot to release resources
let session = create_session();
// Use drop or ensure RAII
drop(session);

// 2. Circular references
// Use Weak references
use std::sync::Weak;

// 3. Unlimited cache
// Use LRU cache
use lru::LruCache;
let mut cache = LruCache::new(100);
```

### UI Lag

```typescript
// Symptom: Interface response delay

// Solution 1: Use Web Worker
// Move computation to Worker
const worker = new Worker('./crypto-worker.ts');
worker.postMessage({ data, operation: 'encrypt' });

// Solution 2: Virtual list
// Use virtual scrolling for large datasets
import { FixedSizeList } from 'react-window';

// Solution 3: Debounce and throttle
import { debounce, throttle } from 'lodash';

const debouncedSearch = debounce((query) => {
  performSearch(query);
}, 300);

// Solution 4: Use useMemo and useCallback
const memoizedData = useMemo(() => processData(data), [data]);
const stableCallback = useCallback(() => {}, []);
```

---

## Platform-Specific Issues

### Windows Issues

#### Console window flashes

```rust
// Symptom: Console window flashes on startup

// Solution: Set Windows subsystem
// Cargo.toml
[package]
edition = "2021"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winuser", "processthreadsapi"] }

// main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

#### Path issues

```rust
// Symptom: Path separator errors

// Solution: Use PathBuf
use std::path::PathBuf;

let path = PathBuf::new()
    .join("config")
    .join("servers.json");

// Or use Path methods
let path = Path::new("config").join("servers.json");
```

#### Windows Defender blocking

```powershell
# Symptom: Windows Defender marks as virus

# Solutions
# 1: Add to exclusions
Add-MpPreference -ExclusionPath "C:\path\to\easyssh"

# 2: Submit to Microsoft
# https://www.microsoft.com/en-us/wdsi/filesubmission
```

### macOS Issues

#### Gatekeeper blocking

```bash
# Symptom: "Cannot open because developer cannot be verified"

# Solution
xattr -dr com.apple.quarantine /Applications/EasySSH.app

# Or hold Control and click to open
```

#### Permission issues

```bash
# Symptom: Cannot access Keychain

# Solution: Add keychain access permissions
# 1. Open "Keychain Access"
# 2. Right-click login keychain
# 3. Select "Get Info"
# 4. Add EasySSH in "Access Control" tab
```

#### Notification permissions

```rust
// Request notification permission
#[cfg(target_os = "macos")]
pub fn request_notification_permission() {
    use mac_notification_sys::*;

    set_application("com.anixops.easyssh");
    get_bundle_identifier_or_default("EasySSH");
}
```

### Linux Issues

#### Desktop environment integration

```bash
# Symptom: Icon not showing

# Solution: Install .desktop file
cat > ~/.local/share/applications/easyssh.desktop << EOF
[Desktop Entry]
Name=EasySSH
Exec=/usr/bin/easyssh
Icon=easyssh
Type=Application
Categories=Network;RemoteAccess;
EOF

# Update desktop database
update-desktop-database ~/.local/share/applications/
```

#### Wayland issues

```bash
# Symptom: GTK4 issues on Wayland

# Solutions
# 1: Force X11
export GDK_BACKEND=x11
./easyssh-gtk4

# 2: Check Wayland support
# Ensure system supports required Wayland protocols
```

---

## Database Issues

### Database Lock

```rust
// Symptom
Error: database is locked

// Solutions
// 1: Enable WAL mode
conn.execute_batch("PRAGMA journal_mode = WAL;")?;

// 2: Increase timeout
conn.execute_batch("PRAGMA busy_timeout = 5000;")?;  // 5 seconds

// 3: Reduce transaction hold time
// Commit transactions quickly
```

### Migration Failure

```rust
// Symptom
Error: migration failed

// Diagnose
// 1. Check current version
SELECT user_version FROM pragma_user_version;

// 2. Check schema
.schema

// 3. Manual fix
// Backup data
.backup main backup.db

// Reset and re-migrate (dev only)
```

### Data Corruption

```bash
# Symptom
Error: database disk image is malformed

# Solutions
# 1: Try repair
sqlite3 corrupt.db ".mode insert" ".output dump.sql" ".dump"
sqlite3 new.db < dump.sql

# 2: Use PRAGMA integrity_check
sqlite3 data.db "PRAGMA integrity_check;"

# 3: Restore from backup
# EasySSH auto-creates backups
ls ~/.local/share/easyssh/backups/
```

---

## Security Issues

### Key Leak

```rust
// Symptom: Keys accidentally committed to git

// Solutions
// 1: Rotate keys immediately
ssh-keygen -t ed25519 -C "new-key"

// 2: Remove from git history
git filter-branch --force --index-filter \
  'git rm --cached --ignore-unmatch path/to/key' \
  HEAD

// 3: Enable key scanning protection
cargo audit
```

### Encryption Failure

```rust
// Symptom
Error: encryption failed

// Common causes
// 1. Out of memory
// 2. Wrong key derivation parameters
// 3. Data too large

// Solutions
let crypto_config = CryptoConfig {
    algorithm: EncryptionAlgorithm::Aes256Gcm,
    kdf: KdfAlgorithm::Argon2id {
        memory: 65536,  // 64MB
        iterations: 3,
        parallelism: 4,
    },
};
```

### Audit Log Issues

```rust
// Symptom: Audit logs not recording

// Check
// 1. Log level
RUST_LOG=easyssh_audit=trace

// 2. Log path permissions
ls -la /var/log/easyssh/

// 3. Config enabled
let config = AuditConfig {
    enabled: true,
    log_path: "/var/log/easyssh/audit.log",
    max_size: 100 * 1024 * 1024,  // 100MB
    rotation: Rotation::Daily,
};
```

---

## Debug Tools

### Log Collection

```bash
# Enable verbose logging
RUST_LOG=trace ./easyssh-gtk4 2>&1 | tee easyssh.log

# Filter by module
RUST_LOG=easyssh_core=debug,ssh2=info,gtk4=warn ./easyssh-gtk4

# Save to file
export RUST_LOG=debug
./easyssh-gtk4 > app.log 2>&1
```

### Network Debugging

```bash
# Capture SSH traffic
tcpdump -i any -w ssh_traffic.pcap port 22

# Analyze
wireshark ssh_traffic.pcap

# SSH verbose output
ssh -vvv user@host

# Check certificates
openssl s_client -connect host:22 -showcerts
```

### System Diagnostics

```bash
# System info
uname -a
lsb_release -a  # Linux
cargo --version
rustc --version

# Dependency check
ldd ./target/release/easyssh-gtk4  # Linux
otool -L ./target/release/easyssh-gtk4  # macOS

# Resource usage
# CPU
perf stat ./easyssh-gtk4

# Memory
valgrind --tool=massif ./easyssh-gtk4

# Disk
iostat -x 1
```

### Issue Report Template

When submitting issues, please include:

```markdown
## Problem Description
Brief description of the issue

## Environment
- OS: (e.g., Windows 11, macOS 14, Ubuntu 22.04)
- EasySSH Version: (e.g., 0.3.0)
- Installation: (e.g., compiled, package)

## Reproduction Steps
1. Open app
2. Click '...'
3. Scroll to '...'
4. Error occurs

## Expected Behavior
What should happen

## Actual Behavior
What actually happened

## Error Message
```
Paste complete error or logs
```

## Additional Info
- Screenshots
- Config files (sanitized)
- Related logs
```

---

## Quick Fix Checklist

| Issue | Quick Fix Command |
|-------|-------------------|
| Build failure | `cargo clean && cargo update && cargo build` |
| Database lock | `rm ~/.local/share/easyssh/*.db-journal` |
| Corrupted config | `mv ~/.config/easyssh ~/.config/easyssh.bak` |
| Cache issues | `cargo clean` |
| GTK theme | `GTK_THEME=Adwaita ./easyssh-gtk4` |
| Wayland issues | `GDK_BACKEND=x11 ./easyssh-gtk4` |
| Permission issues | `chmod 700 ~/.local/share/easyssh` |
| Key permissions | `chmod 600 ~/.ssh/id_*` |

---

## Related Documentation

- [Setup Guide](SETUP.md) - Environment configuration
- [Debugging Guide](DEBUGGING.md) - Debugging techniques
- [Testing Guide](TESTING.md) - Testing strategies
- [Profiling Guide](PROFILING.md) - Performance optimization

---

## Getting Help

### Community Support

- **GitHub Issues**: https://github.com/anixops/easyssh/issues
- **Discussions**: https://github.com/anixops/easyssh/discussions
- **Discord**: https://discord.gg/easyssh

### Commercial Support

- **Enterprise Support**: support@anixops.com
- **Security Reports**: security@anixops.com

---

*Last Updated: 2026-04-02*

---

# 中文

## 目录

1. [编译问题](#编译问题)
2. [运行时错误](#运行时错误)
3. [连接问题](#连接问题)
4. [性能问题](#性能问题)
5. [平台特定问题](#平台特定问题)
6. [数据库问题](#数据库问题)
7. [安全问题](#安全问题)
8. [调试工具](#调试工具)
9. [快速修复清单](#快速修复清单)

---

## 编译问题

### Rust 编译错误

#### 错误：找不到 `link.exe` (Windows)

```powershell
# 症状
error: linker `link.exe` not found

# 解决方案
# 1. 安装 Visual Studio Build Tools
winget install Microsoft.VisualStudio.2022.BuildTools

# 2. 使用 rustup MSVC 工具链
rustup default stable-x86_64-pc-windows-msvc

# 3. 在 VS Developer Command Prompt 中运行
cargo build
```

#### 错误：OpenSSL 链接失败

```bash
# 症状
error: failed to run custom build command for `openssl-sys`

# Linux 解决方案
sudo apt-get install libssl-dev pkg-config

# macOS 解决方案
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)

# Windows 解决方案 (使用 vendored)
# 添加到 Cargo.toml:
# [dependencies]
# openssl = { version = "0.10", features = ["vendored"] }
```

#### 错误：SQLite 绑定失败

```bash
# 症状
error: failed to run custom build command for `libsqlite3-sys`

# 解决方案 1: 使用 bundled SQLite
cargo build --features bundled-sqlite

# 解决方案 2: 安装系统 SQLite
# Ubuntu/Debian
sudo apt-get install libsqlite3-dev

# Fedora
sudo dnf install sqlite-devel

# macOS
brew install sqlite3
export SQLITE3_DIR=$(brew --prefix sqlite)
```

#### 错误：GTK4 编译失败 (Linux)

```bash
# 症状
error: failed to run custom build command for `gtk4-sys`

# 安装 GTK4 依赖
# Ubuntu 22.04+
sudo apt-get install libgtk-4-dev libadwaita-1-dev

# Fedora
sudo dnf install gtk4-devel libadwaita-devel

# Arch
sudo pacman -S gtk4 libadwaita

# 验证安装
pkg-config --modversion gtk4
pkg-config --modversion libadwaita-1
```

### 依赖冲突

```bash
# 症状
error: failed to select a version for the requirement

# 解决方案
# 1: 更新依赖
cargo update

# 2: 检查冲突
cargo tree -d  # 查看重复依赖

# 3: 使用统一版本
# 在 workspace Cargo.toml 中:
[workspace.dependencies]
serde = "1.0"
tokio = "1.0"
```

### 特征冲突

```bash
# 症状
error: the feature `X` is not enabled

# 解决方案
cargo build --features "feature-a feature-b"

# 或在 Cargo.toml 中启用默认特征
[features]
default = ["feature-a", "feature-b"]
```

---

## 运行时错误

### 启动失败

#### GTK4 主题加载失败

```bash
# 症状
Gtk-WARNING **: Theme parsing error

# 解决方案
# 1: 设置默认主题
export GTK_THEME=Adwaita:dark

# 2: 安装主题
sudo apt-get install gnome-themes-extra

# 3: 检查主题文件
ls /usr/share/themes/
```

#### 数据库初始化失败

```rust
// 症状
Error: Failed to initialize database

// 解决方案: 检查数据库目录权限
// 1. 创建目录
mkdir -p ~/.local/share/easyssh

// 2. 检查权限
ls -la ~/.local/share/easyssh/

// 3. 修复权限
chmod 700 ~/.local/share/easyssh
```

#### Keychain 访问失败

```bash
# Linux 症状
Error: Platform secure storage failure

# 解决方案
# 1: 启动 secret service
# GNOME
eval $(gnome-keyring-daemon --start)
export SSH_AUTH_SOCK

# 2: 使用 keyutils
sudo apt-get install keyutils

# 3: 配置备用 keyring
// 在配置文件中:
# keyring_backend = "file"
```

### 崩溃和 Panic

#### 段错误

```bash
# 收集崩溃信息
RUST_BACKTRACE=1 cargo run 2>&1 | tee crash.log

# 使用调试器
gdb ./target/debug/easyssh-gtk4
(gdb) run
(gdb) bt  # 获取 backtrace

# 使用 AddressSanitizer
RUSTFLAGS="-Zsanitizer=address" cargo run
```

#### 内存不足

```bash
# 症状
fatal runtime error: memory allocation failed

# 解决方案
# 1: 限制并发编译
cargo build -j 1

# 2: 增加交换空间
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# 3: 使用 release 模式
cargo run --release
```

---

## 连接问题

### SSH 连接失败

#### 连接超时

```rust
// 症状
Error: Connection timed out

// 诊断步骤
// 1. 检查网络连通性
ping <host>
nc -zv <host> <port>

// 2. 检查防火墙
sudo iptables -L | grep <port>

// 3. 增加超时时间
let config = SshConfig {
    connection_timeout: Duration::from_secs(30),
    ..Default::default()
};
```

#### 认证失败

```rust
// 症状
Error: Authentication failed

// 常见原因和解决方案

// 1. 密码错误
// 验证密码
echo "password" | ssh user@host  // 测试

// 2. 密钥权限问题 (Unix)
chmod 600 ~/.ssh/id_rsa
chmod 700 ~/.ssh

// 3. 密钥格式问题
// 转换密钥格式
ssh-keygen -p -m PEM -f ~/.ssh/id_rsa

// 4. SSH Agent 问题
echo $SSH_AUTH_SOCK
ssh-add -l  // 列出已加载密钥
ssh-add ~/.ssh/id_rsa  // 添加密钥
```

#### 主机密钥验证失败

```bash
# 症状
Error: Host key verification failed

# 解决方案
# 1: 更新 known_hosts
ssh-keygen -R <hostname>
ssh-keygen -R <ip_address>

// 2: 在开发环境禁用 (不推荐用于生产)
let config = SshConfig {
    strict_host_key_checking: false,  // 仅开发使用!
    ..Default::default()
};

// 3: 自动接受新密钥
let config = SshConfig {
    auto_accept_new_keys: true,  // 首次连接自动添加
    ..Default::default()
};
```

### SFTP 传输问题

#### 传输中断

```rust
// 症状
Error: SFTP transfer interrupted

// 解决方案: 启用断点续传
let sftp_config = SftpConfig {
    resume_enabled: true,
    chunk_size: 1024 * 1024,  // 1MB chunks
    retry_attempts: 3,
    retry_delay: Duration::from_secs(1),
};
```

#### 权限拒绝

```bash
# 症状
Error: Permission denied during SFTP

# 检查远程权限
ls -la /remote/path

# 检查本地权限
ls -la /local/path

// 使用正确的权限掩码
let sftp_config = SftpConfig {
    umask: 0o022,
    ..Default::default()
};
```

---

## 性能问题

### 高 CPU 占用

```bash
# 诊断
# 1. 找到占用线程
htop -p $(pgrep easyssh)

# 2. 获取 Rust backtrace
cargo install perf
sudo perf record -g ./target/release/easyssh-gtk4
sudo perf report

// 常见原因和解决方案

// 1. 轮询循环
tokio::time::sleep(Duration::from_millis(10)).await;  // 添加延迟

// 2. 无限递归
// 检查递归函数，添加深度限制

// 3. 渲染循环
// 限制更新频率
```

### 内存泄漏

```bash
# 诊断工具
# 1. Valgrind
valgrind --leak-check=full --show-leak-kinds=all ./target/debug/easyssh-gtk4

# 2. Heaptrack
heaptrack ./target/debug/easyssh-gtk4
heaptrack_gui heaptrack.easyssh-gtk4.*.gz

// 常见内存泄漏原因

// 1. 忘记释放资源
let session = create_session();
// 使用 drop 或确保 RAII
drop(session);

// 2. 循环引用
// 使用 Weak 引用
use std::sync::Weak;

// 3. 缓存无限制
// 使用 LRU 缓存
use lru::LruCache;
let mut cache = LruCache::new(100);
```

### UI 卡顿

```typescript
// 症状: 界面响应延迟

// 解决方案 1: 使用 Web Worker
// 将计算移到 Worker
const worker = new Worker('./crypto-worker.ts');
worker.postMessage({ data, operation: 'encrypt' });

// 解决方案 2: 虚拟列表
// 大量数据时使用虚拟滚动
import { FixedSizeList } from 'react-window';

// 解决方案 3: 防抖和节流
import { debounce, throttle } from 'lodash';

const debouncedSearch = debounce((query) => {
  performSearch(query);
}, 300);

// 解决方案 4: 使用 useMemo 和 useCallback
const memoizedData = useMemo(() => processData(data), [data]);
const stableCallback = useCallback(() => {}, []);
```

---

## 平台特定问题

### Windows 问题

#### 控制台窗口闪烁

```rust
// 症状: 启动时控制台窗口闪烁

// 解决方案: 设置 Windows 子系统
// Cargo.toml
[package]
edition = "2021"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winuser", "processthreadsapi"] }

// main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

#### 路径问题

```rust
// 症状: 路径分隔符错误

// 解决方案: 使用 PathBuf
use std::path::PathBuf;

let path = PathBuf::new()
    .join("config")
    .join("servers.json");

// 或使用 Path 方法
let path = Path::new("config").join("servers.json");
```

#### Windows Defender 拦截

```powershell
# 症状: 被 Windows Defender 标记为病毒

# 解决方案
# 1: 添加到排除项
Add-MpPreference -ExclusionPath "C:\path\to\easyssh"

# 2: 提交给 Microsoft
# https://www.microsoft.com/en-us/wdsi/filesubmission
```

### macOS 问题

#### Gatekeeper 阻止

```bash
# 症状: "无法打开，因为无法验证开发者"

# 解决方案
xattr -dr com.apple.quarantine /Applications/EasySSH.app

# 或按住 Control 键点击打开
```

#### 权限问题

```bash
# 症状: 无法访问 Keychain

# 解决方案: 添加钥匙串访问权限
# 1. 打开 "钥匙串访问"
# 2. 右键点击登录钥匙串
# 3. 选择 "获取信息"
# 4. 在 "访问控制" 标签中添加 EasySSH
```

#### 通知权限

```rust
// 请求通知权限
#[cfg(target_os = "macos")]
pub fn request_notification_permission() {
    use mac_notification_sys::*;

    set_application("com.anixops.easyssh");
    get_bundle_identifier_or_default("EasySSH");
}
```

### Linux 问题

#### 桌面环境集成

```bash
# 症状: 图标不显示

# 解决方案: 安装 .desktop 文件
cat > ~/.local/share/applications/easyssh.desktop << EOF
[Desktop Entry]
Name=EasySSH
Exec=/usr/bin/easyssh
Icon=easyssh
Type=Application
Categories=Network;RemoteAccess;
EOF

# 更新桌面数据库
update-desktop-database ~/.local/share/applications/
```

#### Wayland 问题

```bash
# 症状: GTK4 在 Wayland 下异常

# 解决方案
# 1: 强制使用 X11
export GDK_BACKEND=x11
./easyssh-gtk4

# 2: 检查 Wayland 支持
# 确保系统支持 required 的 Wayland 协议
```

---

## 数据库问题

### 数据库锁定

```rust
// 症状
Error: database is locked

// 解决方案
// 1: 启用 WAL 模式
conn.execute_batch("PRAGMA journal_mode = WAL;")?;

// 2: 增加超时
conn.execute_batch("PRAGMA busy_timeout = 5000;")?;  // 5秒

// 3: 减少事务持有时间
// 尽快提交事务
```

### 迁移失败

```rust
// 症状
Error: migration failed

// 诊断
// 1. 检查当前版本
SELECT user_version FROM pragma_user_version;

// 2. 检查 schema
.schema

// 3. 手动修复
// 备份数据
.backup main backup.db

// 重置并重新迁移 (仅开发环境)
```

### 数据损坏

```bash
# 症状
Error: database disk image is malformed

# 解决方案
# 1: 尝试修复
sqlite3 corrupt.db ".mode insert" ".output dump.sql" ".dump"
sqlite3 new.db < dump.sql

# 2: 使用 PRAGMA integrity_check
sqlite3 data.db "PRAGMA integrity_check;"

# 3: 从备份恢复
# EasySSH 自动创建备份
ls ~/.local/share/easyssh/backups/
```

---

## 安全问题

### 密钥泄露

```rust
// 症状: 密钥意外提交到 git

// 解决方案
// 1: 立即轮换密钥
ssh-keygen -t ed25519 -C "new-key"

// 2: 从 git 历史移除
git filter-branch --force --index-filter \
  'git rm --cached --ignore-unmatch path/to/key' \
  HEAD

// 3: 启用密钥扫描保护
cargo audit
```

### 加密失败

```rust
// 症状
Error: encryption failed

// 常见原因
// 1. 内存不足
// 2. 密钥派生参数错误
// 3. 数据过大

// 解决方案
let crypto_config = CryptoConfig {
    algorithm: EncryptionAlgorithm::Aes256Gcm,
    kdf: KdfAlgorithm::Argon2id {
        memory: 65536,  // 64MB
        iterations: 3,
        parallelism: 4,
    },
};
```

### 审计日志问题

```rust
// 症状: 审计日志未记录

// 检查
// 1. 日志级别
RUST_LOG=easyssh_audit=trace

// 2. 日志路径权限
ls -la /var/log/easyssh/

// 3. 配置启用
let config = AuditConfig {
    enabled: true,
    log_path: "/var/log/easyssh/audit.log",
    max_size: 100 * 1024 * 1024,  // 100MB
    rotation: Rotation::Daily,
};
```

---

## 调试工具

### 日志收集

```bash
# 启用详细日志
RUST_LOG=trace ./easyssh-gtk4 2>&1 | tee easyssh.log

# 按模块过滤
RUST_LOG=easyssh_core=debug,ssh2=info,gtk4=warn ./easyssh-gtk4

# 保存到文件
export RUST_LOG=debug
./easyssh-gtk4 > app.log 2>&1
```

### 网络调试

```bash
# 捕获 SSH 流量
tcpdump -i any -w ssh_traffic.pcap port 22

# 分析
wireshark ssh_traffic.pcap

# SSH 详细输出
ssh -vvv user@host

# 检查证书
openssl s_client -connect host:22 -showcerts
```

### 系统诊断

```bash
# 系统信息
uname -a
lsb_release -a  # Linux
cargo --version
rustc --version

# 依赖检查
ldd ./target/release/easyssh-gtk4  # Linux
otool -L ./target/release/easyssh-gtk4  # macOS

# 资源使用
# CPU
perf stat ./easyssh-gtk4

# 内存
valgrind --tool=massif ./easyssh-gtk4

# 磁盘
iostat -x 1
```

### 问题报告模板

提交问题时，请包含以下信息:

```markdown
## 问题描述
简要描述遇到的问题

## 环境信息
- 操作系统: (例如: Windows 11, macOS 14, Ubuntu 22.04)
- EasySSH 版本: (例如: 0.3.0)
- 安装方式: (例如: 编译安装, 安装包)

## 复现步骤
1. 打开应用
2. 点击 '...'
3. 滚动到 '...'
4. 出现错误

## 预期行为
描述应该发生什么

## 实际行为
描述实际发生了什么

## 错误信息
```
粘贴完整的错误信息或日志
```

## 附加信息
- 截图
- 配置文件 (脱敏后)
- 相关日志
```

---

## 快速修复清单

| 问题 | 快速修复命令 |
|------|-------------|
| 编译失败 | `cargo clean && cargo update && cargo build` |
| 数据库锁定 | `rm ~/.local/share/easyssh/*.db-journal` |
| 配置损坏 | 重置配置: `mv ~/.config/easyssh ~/.config/easyssh.bak` |
| 缓存问题 | `cargo clean` |
| GTK 主题 | `GTK_THEME=Adwaita ./easyssh-gtk4` |
| Wayland 问题 | `GDK_BACKEND=x11 ./easyssh-gtk4` |
| 权限问题 | `chmod 700 ~/.local/share/easyssh` |
| 密钥权限 | `chmod 600 ~/.ssh/id_*` |

---

## 相关文档

- [设置指南](./SETUP.md) - 环境配置
- [调试指南](./DEBUGGING.md) - 调试技术
- [测试指南](./TESTING.md) - 测试策略
- [性能分析指南](./PROFILING.md) - 性能优化

---

## 获取帮助

### 社区支持

- **GitHub Issues**: https://github.com/anixops/easyssh/issues
- **Discussions**: https://github.com/anixops/easyssh/discussions
- **Discord**: https://discord.gg/easyssh

### 商业支持

- **企业支持**: support@anixops.com
- **安全报告**: security@anixops.com

---

*最后更新: 2026-04-02*
