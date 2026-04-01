# EasySSH 故障排除指南

> 常见问题诊断和解决方案速查手册

---

## 目录

1. [编译问题](#1-编译问题)
2. [运行时错误](#2-运行时错误)
3. [连接问题](#3-连接问题)
4. [性能问题](#4-性能问题)
5. [平台特定问题](#5-平台特定问题)
6. [数据库问题](#6-数据库问题)
7. [安全问题](#7-安全问题)
8. [调试工具](#8-调试工具)

---

## 1. 编译问题

### 1.1 Rust 编译错误

#### 错误：找不到 `link.exe` (Windows)

```powershell
# 症状
error: linker `link.exe` not found

# 解决方案
# 1. 安装 Visual Studio Build Tools
winget install Microsoft.VisualStudio.2022.BuildTools

# 2. 或使用 rustup 安装 MSVC 工具链
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

# Windows 解决方案
# 使用 vendored 版本 (推荐)
# 在 Cargo.toml 中:
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

### 1.2 依赖冲突

```bash
# 症状
error: failed to select a version for the requirement

# 解决方案 1: 更新依赖
cargo update

# 解决方案 2: 检查冲突
cargo tree -d  # 查看重复依赖

# 解决方案 3: 使用统一版本
# 在 workspace Cargo.toml 中定义统一版本
[workspace.dependencies]
serde = "1.0"
tokio = "1.0"
```

### 1.3 特征冲突

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

## 2. 运行时错误

### 2.1 启动失败

#### GTK4 主题加载失败

```bash
# 症状
Gtk-WARNING **: Theme parsing error

# 解决方案 1: 设置默认主题
export GTK_THEME=Adwaita:dark

# 解决方案 2: 安装主题
sudo apt-get install gnome-themes-extra

# 解决方案 3: 检查主题文件
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

# 解决方案 1: 启动 secret service
# GNOME
eval $(gnome-keyring-daemon --start)
export SSH_AUTH_SOCK

# 解决方案 2: 使用 keyutils
sudo apt-get install keyutils

# 解决方案 3: 配置备用 keyring
# 在配置文件中设置:
# keyring_backend = "file"
```

### 2.2 崩溃和 Panic

#### 段错误 (Segmentation Fault)

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

#### 内存不足 (OOM)

```bash
# 症状
fatal runtime error: memory allocation failed

# 解决方案 1: 限制并发编译
cargo build -j 1

# 解决方案 2: 增加交换空间
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# 解决方案 3: 使用 release 模式 (更少的 debug 信息)
cargo run --release
```

---

## 3. 连接问题

### 3.1 SSH 连接失败

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
echo "password" | ssh user@host  # 测试

// 2. 密钥权限问题 (Unix)
chmod 600 ~/.ssh/id_rsa
chmod 700 ~/.ssh

// 3. 密钥格式问题
// 转换密钥格式
ssh-keygen -p -m PEM -f ~/.ssh/id_rsa

// 4. SSH Agent 问题
echo $SSH_AUTH_SOCK
ssh-add -l  # 列出已加载密钥
ssh-add ~/.ssh/id_rsa  # 添加密钥
```

#### 主机密钥验证失败

```bash
# 症状
Error: Host key verification failed

# 解决方案 1: 更新 known_hosts
ssh-keygen -R <hostname>
ssh-keygen -R <ip_address>

# 解决方案 2: 在开发环境禁用 (不推荐用于生产)
let config = SshConfig {
    strict_host_key_checking: false,  // 仅开发使用!
    ..Default::default()
};

// 解决方案 3: 自动接受新密钥
let config = SshConfig {
    auto_accept_new_keys: true,  // 首次连接自动添加
    ..Default::default()
};
```

### 3.2 SFTP 传输问题

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

# 使用正确的权限掩码
let sftp_config = SftpConfig {
    umask: 0o022,
    ..Default::default()
};
```

---

## 4. 性能问题

### 4.1 高 CPU 占用

```bash
# 诊断
# 1. 找到占用线程
htop -p $(pgrep easyssh)

# 2. 获取 Rust backtrace
cargo install perf
sudo perf record -g ./target/release/easyssh-gtk4
sudo perf report

# 常见原因和解决方案

# 1. 轮询循环
tokio::time::sleep(Duration::from_millis(10)).await;  // 添加延迟

// 2. 无限递归
// 检查递归函数，添加深度限制

// 3. 渲染循环
// 使用 requestAnimationFrame
// 限制更新频率
```

### 4.2 内存泄漏

```bash
# 诊断工具
# 1. Valgrind
valgrind --leak-check=full --show-leak-kinds=all ./target/debug/easyssh-gtk4

# 2. Heaptrack
heaptrack ./target/debug/easyssh-gtk4
heaptrack_gui heaptrack.easyssh-gtk4.*.gz

# 常见内存泄漏原因

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

### 4.3 UI 卡顿

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

## 5. 平台特定问题

### 5.1 Windows 问题

#### 控制台窗口闪烁

```rust
// 症状: 启动时控制台窗口闪烁

// 解决方案: 设置 Windows 子系统
// Cargo.toml
[package]
edition = "2021"

# Windows 特定配置
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

# 解决方案 1: 添加到排除项
Add-MpPreference -ExclusionPath "C:\path\to\easyssh"

# 解决方案 2: 提交给 Microsoft 进行扫描
# https://www.microsoft.com/en-us/wdsi/filesubmission
```

### 5.2 macOS 问题

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

### 5.3 Linux 问题

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

# 解决方案 1: 强制使用 X11
export GDK_BACKEND=x11
./easyssh-gtk4

# 解决方案 2: 检查 Wayland 支持
# 确保系统支持 required 的 Wayland 协议
```

---

## 6. 数据库问题

### 6.1 数据库锁定

```rust
// 症状
Error: database is locked

// 解决方案 1: 启用 WAL 模式
conn.execute_batch("PRAGMA journal_mode = WAL;")?;

// 解决方案 2: 增加超时
conn.execute_batch("PRAGMA busy_timeout = 5000;")?;  // 5秒

// 解决方案 3: 减少事务持有时间
// 尽快提交事务
```

### 6.2 迁移失败

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

// 重置并重新迁移
// (仅在开发环境)
```

### 6.3 数据损坏

```bash
# 症状
Error: database disk image is malformed

# 解决方案
# 1. 尝试修复
sqlite3 corrupt.db ".mode insert" ".output dump.sql" ".dump"
sqlite3 new.db < dump.sql

# 2. 使用 PRAGMA integrity_check
sqlite3 data.db "PRAGMA integrity_check;"

# 3. 从备份恢复
# EasySSH 自动创建备份
ls ~/.local/share/easyssh/backups/
```

---

## 7. 安全问题

### 7.1 密钥泄露

```rust
// 症状: 密钥意外提交到 git

// 解决方案
// 1. 立即轮换密钥
ssh-keygen -t ed25519 -C "new-key"

// 2. 从 git 历史移除
git filter-branch --force --index-filter \
  'git rm --cached --ignore-unmatch path/to/key' \
  HEAD

// 3. 启用密钥扫描保护
cargo audit
```

### 7.2 加密失败

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

### 7.3 审计日志问题

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

## 8. 调试工具

### 8.1 日志收集

```bash
# 启用详细日志
RUST_LOG=trace ./easyssh-gtk4 2>&1 | tee easyssh.log

# 按模块过滤
RUST_LOG=easyssh_core=debug,ssh2=info,gtk4=warn ./easyssh-gtk4

# 保存到文件
export RUST_LOG=debug
./easyssh-gtk4 > app.log 2>&1
```

### 8.2 网络调试

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

### 8.3 系统诊断

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

### 8.4 问题报告模板

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

## 9. 快速修复清单

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

## 10. 相关文档

- [设置指南](./SETUP.md) - 环境配置
- [调试指南](./DEBUGGING.md) - 调试技术
- [测试指南](./TESTING.md) - 测试策略
- [性能分析指南](./PROFILING.md) - 性能优化

---

## 11. 获取帮助

### 11.1 社区支持

- **GitHub Issues**: https://github.com/anixops/easyssh/issues
- **Discussions**: https://github.com/anixops/easyssh/discussions
- **Discord**: https://discord.gg/easyssh

### 11.2 商业支持

- **企业支持**: support@anixops.com
- **安全报告**: security@anixops.com

---

*最后更新: 2026-04-01*
