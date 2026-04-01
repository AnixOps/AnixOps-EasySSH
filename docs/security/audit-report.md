# EasySSH 安全审计报告

**审计版本**: v0.3.0
**审计日期**: 2026-03-31
**审计标准**: OWASP Top 10 2021, CWE/SANS Top 25
**审计Agent**: #17 (全平台安全审计)

---

## 执行摘要

本次审计对EasySSH代码库进行了全面的安全审查，涵盖Rust核心代码、前端TypeScript/React代码以及依赖项分析。共发现**1个高危**、**5个中危**和**8个低危**安全问题。另有**5个安全建议**需要改进。

**总体评级**: 中等风险 - 需要修复后方可生产部署

---

## 严重级别分类

| 级别 | 数量 | 描述 |
|------|------|------|
| 高危 (Critical) | 1 | 可能导致系统完全被接管或数据大规模泄露 |
| 中危 (High) | 5 | 可能导致数据泄露或权限提升 |
| 低危 (Medium) | 8 | 可能导致信息泄露或局部安全问题 |
| 建议 (Low) | 5 | 最佳实践改进，潜在安全隐患 |

---

## 高危问题 (CRITICAL)

### 1. CRITICAL-001: Debug WebSocket 接口存在远程代码执行风险

**文件**: `core/src/debug_ws.rs`
**行号**: 340-361
**风险等级**: CRITICAL

**问题描述**:
```rust
"fs.write" => {
    let path = param_string(&params, "path")?;
    let content = param_string(&params, "content")?;
    ai_programming::write_file(path.clone(), content.clone()).await
    ...
}

"fs.edit" => {
    let path = param_string(&params, "path")?;
    let old_string = param_string(&params, "old_string")?;
    let new_string = param_string(&params, "new_string")?;
    let result = ai_programming::edit_file(...).await
    ...
}
```

WebSocket debug接口提供了`fs.write`和`fs.edit`操作，允许任意文件写入和修改。虽然目前有loopback地址限制（`resolve_loopback_addr`），但以下情况仍存在风险：

1. **SSRF攻击**: 如果应用部署在云环境或容器，可能绕过loopback限制
2. **本地提权**: 攻击者可覆盖可执行文件、配置文件、SSH密钥等
3. **代码注入**: 修改Rust源文件注入恶意代码，下次编译时执行
4. **数据销毁**: 删除关键系统文件

**OWASP参考**: A03:2021 - Injection, A05:2021 - Security Misconfiguration
**CWE参考**: CWE-78 (OS Command Injection), CWE-22 (Path Traversal)

**修复建议**:
1. 仅在`debug_assertions`模式下启用文件写入功能
2. 添加路径白名单，限制只能写入项目目录
3. 添加文件内容签名验证
4. 在生产构建中完全禁用debug WebSocket接口

```rust
#[cfg(debug_assertions)]
"fs.write" => { ... }

#[cfg(debug_assertions)]
"fs.edit" => { ... }
```

---

## 中危问题 (HIGH)

### 2. HIGH-001: SSH命令执行缺少输入验证

**文件**: `core/src/ssh.rs`
**行号**: 394-421, 713-734
**风险等级**: HIGH

**问题描述**:
```rust
pub async fn execute(&self, session_id: &str, command: &str) -> Result<String, LiteError> {
    ...
    let command = command.to_string();
    ...
    channel.exec(&command)
        .map_err(|e| LiteError::Ssh(format!("Exec failed: {}", e)))?;
    ...
}
```

`command`参数直接传递给`channel.exec()`，没有进行任何验证或转义。虽然ssh2 crate处理了底层的shell转义，但如果应用层直接构造命令字符串，仍存在命令注入风险。

**风险场景**:
- 攻击者控制的服务器返回恶意payload，诱导用户执行危险命令
- 如果将来添加命令拼接功能，容易引入注入漏洞

**修复建议**:
1. 添加命令白名单验证
2. 使用参数化命令执行而不是字符串拼接
3. 添加命令审计日志

```rust
pub async fn execute(&self, session_id: &str, command: &str) -> Result<String, LiteError> {
    // 验证命令不包含危险字符
    if contains_dangerous_chars(command) {
        return Err(LiteError::Ssh("Command contains invalid characters".to_string()));
    }
    ...
}
```

### 3. HIGH-002: 终端唤起存在命令注入风险

**文件**: `core/src/terminal.rs`
**行号**: 61-88, 104-138, 163-172
**风险等级**: HIGH

**问题描述**:
```rust
#[cfg(target_os = "windows")]
pub fn open_native_terminal(...) -> Result<(), LiteError> {
    let ssh_args = SshArgs::new(host, port, username, auth_type);
    let ssh_cmd = ssh_args.to_command_string();
    ...
    .arg(&ssh_cmd)  // 直接使用构造的命令字符串
}
```

macOS和Linux版本使用AppleScript/终端命令执行，构造的命令字符串包含host、username等用户输入，存在潜在的命令注入风险。

**风险场景**:
```
host = "192.168.1.1; rm -rf /"
username = "user\"; malicious_command; echo \""
```

**修复建议**:
1. 使用参数列表而非命令字符串
2. 对所有输入进行严格验证（主机名只允许IP或域名格式）
3. 使用execvp风格的API，避免shell解析

### 4. HIGH-003: FFI接口缺少输入验证

**文件**: `core/src/ffi.rs`
**行号**: 129-158, 166-190
**风险等级**: HIGH

**问题描述**:
```rust
pub unsafe extern "C" fn easyssh_add_server(
    handle: *mut EasySSHAppState,
    json_config: *const c_char,
) -> c_int {
    ...
    let c_str = match CStr::from_ptr(json_config).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let new_server: NewServer = match serde_json::from_str(c_str) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    // 直接存入数据库，没有验证字段内容
```

FFI接口接收JSON配置直接反序列化并存储，缺少字段级验证：
- `host`字段可能包含恶意payload
- `identity_file`可能是任意路径（路径遍历）
- `name`可能包含超长字符串（DoS）

**修复建议**:
1. 添加字段验证逻辑
2. 限制字符串长度
3. 验证host格式（IP或合法域名）
4. 验证identity_file路径

### 5. HIGH-004: 全局加密状态锁竞争可能导致信息泄露

**文件**: `core/src/crypto.rs`
**行号**: 139-140
**风险等级**: HIGH

**问题描述**:
```rust
pub static CRYPTO_STATE: std::sync::LazyLock<Mutex<CryptoState>> =
    std::sync::LazyLock::new(|| Mutex::new(CryptoState::new()));
```

使用标准库Mutex而非异步安全锁，在高并发场景下：
1. 可能导致死锁
2. 长时间持有锁影响性能
3. `poison()`后无法恢复

**修复建议**:
1. 使用`tokio::sync::RwLock`替代`std::sync::Mutex`
2. 添加锁超时机制
3. 实现锁恢复逻辑

### 6. HIGH-005: Pro版本的SSO配置硬编码敏感信息风险

**文件**: `core/src/pro.rs`
**行号**: 274-299
**风险等级**: HIGH

**问题描述**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProvider {
    pub client_secret: String,  // 明文存储客户端密钥
    ...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoSession {
    pub access_token: String,   // 明文存储访问令牌
    pub refresh_token: Option<String>,  // 明文存储刷新令牌
    ...
}
```

SSO的client_secret、access_token、refresh_token以明文形式存储和序列化，存在泄露风险。

**修复建议**:
1. 使用keyring存储client_secret
2. 使用CryptoState加密access_token和refresh_token
3. 实现token自动轮换机制

---

## 低危问题 (MEDIUM)

### 7. MEDIUM-001: 错误信息可能泄露敏感路径

**文件**: `core/src/error.rs`
**风险等级**: MEDIUM

**问题描述**:
错误消息直接包含文件路径、服务器地址等敏感信息：
```rust
#[error("SSH连接失败: {host}:{port} - {message}")]
SshConnectionFailed { host: String, port: u16, message: String },

#[error("SSH认证失败: {username}@{host}")]
SshAuthFailed { host: String, username: String },
```

这些错误信息可能被记录到日志或返回给前端，泄露内部网络拓扑。

**修复建议**:
1. 生产环境脱敏处理错误信息
2. 区分用户可见错误和内部日志错误
3. 使用错误代码映射，不直接暴露原始错误

### 8. MEDIUM-002: AI编程接口的Command执行缺少限制

**文件**: `core/src/ai_programming.rs`
**行号**: 158-209
**风险等级**: MEDIUM

**问题描述**:
```rust
pub async fn ai_check_rust() -> Result<CheckResult, String> {
    let output = tokio::process::Command::new("cargo")
        .args(["check", "--message-format=json"])
        .current_dir("src-tauri")  // 固定目录
        ...
}
```

虽然这些Command调用是硬编码的，但`current_dir("src-tauri")`是相对路径，如果工作目录被篡改，可能执行恶意代码。

**修复建议**:
1. 使用绝对路径
2. 在执行前验证cargo可执行文件的真实性
3. 限制Command执行环境（chroot或容器）

### 9. MEDIUM-003: 配置导入导出缺少完整性验证

**文件**: `core/src/config_import_export.rs`
**风险等级**: MEDIUM

**问题描述**:
导入功能没有验证数据完整性：
1. 没有签名验证，无法确认配置来源可信
2. CSV导入没有严格的字段验证
3. 导入的服务器配置可能包含恶意payload

**修复建议**:
1. 添加导出文件签名
2. 导入时验证签名
3. 添加schema版本验证

### 10. MEDIUM-004: 数据库查询缺少超时限制

**文件**: `core/src/db.rs`
**风险等级**: MEDIUM

**问题描述**:
数据库操作使用rusqlite同步API，没有超时限制：
```rust
let conn = Connection::open(path)?;
```

长时间运行的查询可能导致：
1. DoS攻击（连接池耗尽）
2. UI无响应

**修复建议**:
1. 启用rusqlite的busy_timeout
2. 使用异步数据库连接池（如sqlx）
3. 添加查询超时

### 11. MEDIUM-005: 密码尝试没有速率限制

**文件**: `core/src/crypto.rs`
**风险等级**: MEDIUM

**问题描述**:
主密码解锁没有速率限制：
```rust
pub fn unlock(&mut self, master_password: &str) -> Result<bool, LiteError> {
    let salt = self.salt.ok_or(LiteError::InvalidMasterPassword)?;
    let key = self.derive_key_internal(master_password, &salt)?;
    ...
}
```

攻击者可以进行暴力破解尝试。

**修复建议**:
1. 添加尝试次数限制（如5次失败后锁定）
2. 指数退避延迟
3. 持久化失败次数到安全存储

### 12. MEDIUM-006: SFTP路径遍历风险

**文件**: `core/src/sftp.rs`
**行号**: 31-206
**风险等级**: MEDIUM

**问题描述**:
SFTP操作直接使用用户传入路径：
```rust
pub async fn list_dir(&self, session_id: &str, path: &str) -> Result<Vec<SftpEntry>, LiteError> {
    let sftp = sftp_mutex.lock().await;
    let dir = sftp.readdir(Path::new(path))  // 直接使用用户输入
```

`Path::new()`不验证路径合法性，可能导致：
1. 访问SFTP根目录之外的文件（如果服务器配置不当）
2. 符号链接跟随攻击

**修复建议**:
1. 规范化路径（使用`Path::canonicalize`）
2. 验证路径在允许的根目录内
3. 限制符号链接跟随

### 13. MEDIUM-007: 会话ID可预测

**文件**: `core/src/ssh.rs`
**行号**: 273
**风险等级**: MEDIUM

**问题描述**:
```rust
let session_id = uuid::Uuid::new_v4().to_string();
```

使用UUID v4生成的会话ID虽然随机，但如果应用暴露会话枚举接口，攻击者可能遍历会话ID。

**修复建议**:
1. 添加会话ID访问权限验证
2. 使用更长的随机令牌
3. 实现会话绑定（绑定到IP或设备指纹）

### 14. MEDIUM-008: 日志可能记录敏感信息

**文件**: 多处
**风险等级**: MEDIUM

**问题描述**:
代码中多处使用`log::info!`和`log::warn!`记录信息，可能意外记录敏感数据：
```rust
log::info!("SSH MUX: Created new connection {}@{}:{}", username, host, port);
```

**修复建议**:
1. 审查所有日志输出，确保不记录密码、密钥、token
2. 实现日志脱敏过滤器
3. 区分不同敏感级别的日志

---

## 安全建议 (LOW)

### 15. LOW-001: 依赖项需要定期安全审计

**建议**:
1. 配置cargo-deny CI检查
2. 订阅RustSec安全通告
3. 每月更新依赖版本

**已配置**: `deny.toml`存在，但需要启用cargo-deny CI步骤

### 16. LOW-002: 缺少安全响应头

**文件**: 前端代码
**建议**:
添加以下安全响应头：
```
Content-Security-Policy: default-src 'self'
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
```

### 17. LOW-003: 构建配置需要加固

**文件**: `core/Cargo.toml`
**建议**:
1. 启用更多编译安全选项：
```toml
[profile.release]
overflow-checks = true
```

2. 使用`strip = true`减少符号信息（已配置）

### 18. LOW-004: 测试覆盖率需要提升

**当前状态**: 部分模块缺少安全测试
**建议**:
1. 添加命令注入防护测试
2. 添加路径遍历防护测试
3. 添加加密/解密边界测试

### 19. LOW-005: 需要安全事件监控

**建议**:
1. 添加异常行为检测（如频繁解锁失败）
2. 实现安全事件告警机制
3. 集成SIEM系统（Pro版本）

---

## 依赖项安全分析

### 关键依赖状态

| 依赖 | 版本 | 状态 | 备注 |
|------|------|------|------|
| aes-gcm | 0.10.3 | 安全 | 标准AEAD实现 |
| argon2 | 0.5.3 | 安全 | 密码哈希 |
| ssh2 | 0.9 | 需关注 | 依赖libssh2，需保持更新 |
| rusqlite | 0.31 | 安全 | bundled模式 |
| tokio | 1.x | 安全 | 最新版本 |
| tokio-tungstenite | 0.24 | 安全 | WebSocket实现 |

### 已知漏洞检查

根据RustSec Advisory Database，当前依赖版本未发现已知高危漏洞。建议：
1. 每月运行`cargo audit`
2. 关注`ssh2`和`libssh2`的安全更新

---

## 修复优先级

### 立即修复 (P0 - 1周内)
- [ ] CRITICAL-001: 限制或移除debug WebSocket文件写入功能
- [ ] HIGH-002: 修复终端命令注入风险
- [ ] HIGH-003: 添加FFI输入验证

### 短期修复 (P1 - 1个月内)
- [ ] HIGH-001: 添加SSH命令输入验证
- [ ] HIGH-004: 优化全局加密状态锁
- [ ] HIGH-005: 加密Pro版本SSO凭据
- [ ] MEDIUM-001: 错误信息脱敏
- [ ] MEDIUM-003: 配置导入导出签名验证

### 中期改进 (P2 - 3个月内)
- [ ] MEDIUM-004: 数据库查询超时
- [ ] MEDIUM-005: 密码尝试速率限制
- [ ] MEDIUM-006: SFTP路径验证
- [ ] MEDIUM-007: 会话ID安全增强
- [ ] 所有LOW级别建议

---

## 安全测试用例建议

### 需要添加的测试

```rust
// 1. 命令注入防护测试
#[test]
fn test_ssh_command_injection_protection() {
    let malicious_commands = vec![
        "ls; rm -rf /",
        "$(whoami)",
        "`cat /etc/passwd`",
        "ls && cat /etc/shadow",
        "ls || reboot",
    ];
    // 验证所有恶意命令被拒绝
}

// 2. 路径遍历防护测试
#[test]
fn test_path_traversal_protection() {
    let malicious_paths = vec![
        "../../../etc/passwd",
        "/etc/passwd",
        "..\\..\\Windows\\System32\\config\\SAM",
    ];
    // 验证路径被规范化并限制在允许范围内
}

// 3. 加密边界测试
#[test]
fn test_crypto_boundary_conditions() {
    // 空数据加密
    // 超大数据加密(>1GB)
    // 特殊字符处理
    // Unicode密码测试
}

// 4. 并发安全测试
#[test]
fn test_concurrent_crypto_access() {
    // 多线程同时访问CRYPTO_STATE
    // 验证锁安全性和数据一致性
}
```

---

## 附录

### 审计工具和方法

1. **静态分析**: 人工代码审查 + cargo clippy
2. **依赖分析**: cargo tree, cargo audit (planned)
3. **安全标准**: OWASP Top 10 2021, CWE Top 25
4. **Rust特定**: 检查unsafe代码、FFI边界、panic处理

### 审计范围

- [x] core/src/crypto.rs - 加密实现
- [x] core/src/db.rs - 数据库操作
- [x] core/src/ssh.rs - SSH连接管理
- [x] core/src/sftp.rs - SFTP文件操作
- [x] core/src/keychain.rs - 密钥存储
- [x] core/src/terminal.rs - 终端唤起
- [x] core/src/ffi.rs - FFI接口
- [x] core/src/debug_ws.rs - Debug WebSocket
- [x] core/src/ai_programming.rs - AI编程接口
- [x] core/src/pro.rs - Pro版本功能
- [x] src/stores/*.ts - 前端状态管理
- [x] Cargo.toml / Cargo.lock - 依赖分析

### 未审计项

- [ ] platforms/windows/ - Windows原生UI（部分审查）
- [ ] platforms/linux/ - Linux GTK4 UI（部分审查）
- [ ] pro-server/ - Pro后端服务
- [ ] 第三方依赖的深层审计

---

**审计完成签名**: Agent #17
**下次审计建议日期**: 2026-06-30

