# EasySSH Lite 功能详解
# EasySSH Lite Features

> **English Version**: [Jump to English Section](#feature-details)

---

## 功能概览 / Feature Overview

EasySSH Lite 是专注于 SSH 配置安全管理的轻量级工具，提供核心而强大的服务器连接管理功能。

```
┌─────────────────────────────────────────────────────────────┐
│                    EasySSH Lite v0.3.0                       │
│                    功能架构图                                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   安全存储    │  │   服务器管理  │  │   分组管理    │      │
│  │  ├─Argon2id  │  │  ├─CRUD      │  │  ├─嵌套分组   │      │
│  │  ├─AES-256   │  │  ├─搜索过滤  │  │  ├─拖拽排序   │      │
│  │  └─Keychain  │  │  ├─快速连接  │  │  └─批量操作   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   认证管理    │  │   终端集成    │  │   数据管理    │      │
│  │  ├─密码认证  │  │  ├─唤起终端  │  │  ├─导入导出   │      │
│  │  ├─密钥认证  │  │  ├─多标签页  │  │  ├─加密备份   │      │
│  │  └─SSH Agent │  │  └─自定义    │  │  └─跨平台同步 │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 一、安全加密存储 / Secure Encrypted Storage

### 1.1 核心安全机制

EasySSH Lite 采用军用级加密方案保护您的 SSH 配置：

```rust
// 加密流程示意 / Encryption Flow
┌─────────────────────────────────────────────────────────┐
│  主密码 (Master Password)                                │
│     ↓                                                    │
│  Argon2id (密码哈希算法)                                  │
│     ↓                                                    │
│  派生密钥 (Derived Key)                                   │
│     ↓                                                    │
│  AES-256-GCM (对称加密)                                   │
│     ↓                                                    │
│  加密配置文件 (Encrypted Config)                         │
└─────────────────────────────────────────────────────────┘
```

| 安全组件 | 技术规格 | 安全等级 |
|----------|----------|----------|
| 密码哈希 | Argon2id | OWASP 推荐 |
| 对称加密 | AES-256-GCM | 军用级 |
| 密钥存储 | 系统 Keychain/Keyring | 硬件级隔离 |
| 内存保护 | SecureString / mlock | 防交换泄露 |

### 1.2 主密码设置

首次启动时强制设置主密码：

```
┌─────────────────────────────────────────────┐
│  🔐 设置主密码                                │
├─────────────────────────────────────────────┤
│                                              │
│  密码强度: [████████░░] 强                   │
│                                              │
│  请输入主密码:     [****************]         │
│  确认密码:         [****************]         │
│                                              │
│  ✅ 至少 8 个字符                             │
│  ✅ 包含大写字母                              │
│  ✅ 包含小写字母                              │
│  ✅ 包含数字                                  │
│  ⬜ 包含特殊字符 (建议)                        │
│                                              │
│  ⚠️  警告: 主密码丢失将导致数据无法恢复！     │
│                                              │
└─────────────────────────────────────────────┘
```

### 1.3 密钥派生参数

```rust
// Argon2id 参数配置 (OWASP 推荐)
pub const ARGON2_MEMORY_COST: u32 = 65536;    // 64 MB
pub const ARGON2_TIME_COST: u32 = 3;          // 3 轮迭代
pub const ARGON2_PARALLELISM: u32 = 4;        // 4 并行线程
pub const ARGON2_HASH_LENGTH: usize = 32;     // 256-bit 密钥
```

### 1.4 自动锁屏保护

```
设置 → 安全 → 自动锁定
├── 从不
├── 5 分钟
├── 15 分钟 (默认)
├── 30 分钟
└── 1 小时

锁定后需要重新输入主密码解锁
```

---

## 二、服务器连接管理 / Server Connection Management

### 2.1 添加服务器

支持两种认证方式：

#### 密码认证
```yaml
服务器配置:
  name: "阿里云生产环境"
  host: "47.123.456.789"
  port: 22
  username: "root"
  auth_type: "password"
  password: "[加密存储于系统钥匙串]"
  description: "主生产服务器"
  tags: ["production", "aliyun", "web"]
```

#### SSH 密钥认证
```yaml
服务器配置:
  name: "GitHub Actions Runner"
  host: "192.168.1.100"
  port: 22
  username: "runner"
  auth_type: "key"
  private_key: "~/.ssh/id_rsa_runner"
  passphrase: "[可选，加密存储]"
  public_key: "~/.ssh/id_rsa_runner.pub"
  key_algorithm: "ed25519"  # 支持 rsa, ed25519, ecdsa
```

### 2.2 高级连接选项

```rust
pub struct ServerConfig {
    // 基础信息
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,

    // 认证信息
    pub auth: AuthMethod,

    // 连接选项
    pub connection: ConnectionOptions,

    // 终端选项
    pub terminal: TerminalOptions,
}

pub struct ConnectionOptions {
    pub timeout_seconds: u64,        // 连接超时 (默认 30s)
    pub keepalive_interval: u64,   // 心跳间隔 (默认 60s)
    pub retry_attempts: u32,         // 重试次数 (默认 3)
    pub compression: bool,           // 启用压缩
}
```

### 2.3 快速连接

双击服务器条目即可连接：

```
连接流程:
1. 检查 SSH 密钥是否已加载到 Agent
2. 唤起系统终端 (Windows Terminal / iTerm2 / GNOME Terminal)
3. 构建 SSH 命令
4. 执行连接

生成的 SSH 命令示例:
ssh -p 22 -i ~/.ssh/id_rsa -o ServerAliveInterval=60 root@192.168.1.100
```

### 2.4 批量操作

```
┌─────────────────────────────────────────────┐
│  批量操作菜单 (选中 3 台服务器)               │
├─────────────────────────────────────────────┤
│                                              │
│  ┌─ 连接 ──────────────────────────────┐    │
│  │  同时连接所有选中服务器              │    │
│  └──────────────────────────────────────┘    │
│                                              │
│  ┌─ 编辑 ──────────────────────────────┐    │
│  │  批量修改标签...                     │    │
│  │  批量移动到分组...                   │    │
│  │  批量更新密钥...                     │    │
│  └──────────────────────────────────────┘    │
│                                              │
│  ┌─ 导出 ──────────────────────────────┐    │
│  │  导出选中服务器配置                  │    │
│  │  复制连接命令到剪贴板                │    │
│  └──────────────────────────────────────┘    │
│                                              │
│  ┌─ 删除 ──────────────────────────────┐    │
│  │  删除选中服务器 (需要确认)           │    │
│  └──────────────────────────────────────┘    │
│                                              │
└─────────────────────────────────────────────┘
```

---

## 三、分组与组织 / Groups & Organization

### 3.1 分组层次结构

```
📁 服务器分组结构
│
├── 📁 开发环境
│   ├── 📁 前端服务器
│   │   ├── 🖥️  nginx-01
│   │   └── 🖥️  nginx-02
│   │
│   ├── 📁 后端服务器
│   │   ├── 🖥️  api-01
│   │   └── 🖥️  api-02
│   │
│   └── 📁 数据库
│       ├── 🖥️  postgres-primary
│       └── 🖥️  postgres-replica
│
├── 📁 测试环境
│   ├── 🖥️  test-web
│   └── 🖥️  test-api
│
├── 📁 生产环境
│   ├── 🖥️  prod-web-01
│   ├── 🖥️  prod-web-02
│   └── 🖥️  prod-db-master
│
└── 📁 个人项目
    ├── 🖥️  home-nas
    └── 🖥️  raspberry-pi
```

### 3.2 分组操作

```rust
pub enum GroupAction {
    Create { name: String, parent_id: Option<Uuid> },
    Rename { id: Uuid, new_name: String },
    Move { id: Uuid, new_parent_id: Option<Uuid> },
    Delete { id: Uuid, cascade: bool },
    Sort { id: Uuid, new_position: usize },
}
```

### 3.3 智能排序

```
排序选项:
├── 手动排序 (拖拽)
├── 名称 A-Z
├── 名称 Z-A
├── 最近连接
├── 连接频率
└── 自定义标签排序
```

---

## 四、搜索与过滤 / Search & Filter

### 4.1 模糊搜索

```
搜索: "p1"

匹配结果:
├── 🖥️  生产服务器-01          ← 匹配 "p" + "1"
├── 🖥️  postgres-01            ← 匹配 "p" + "1"
├── 🖥️  primary-api              ← 匹配 "p"..."1" (模糊)
└── 🖥️  192.168.1.100           ← 不匹配

搜索算法: Fuzzy Matching (近似字符串匹配)
```

### 4.2 高级过滤

```
过滤器组合:
├── 关键词: "prod"
├── 分组: "生产环境"
├── 标签: ["web", "critical"]
├── 认证方式: "密钥"
└── 状态: "最近连接"

结果: 2 台服务器匹配
```

### 4.3 搜索语法

```
# 标签搜索
tag:production

# 主机地址搜索
host:192.168.1

# 用户名搜索
user:root

# 组合搜索
tag:web AND tag:critical NOT tag:deprecated

# 最近连接
connected:within:7d
```

---

## 五、终端集成 / Terminal Integration

### 5.1 支持的原生终端

| 平台 | 终端 | 检测优先级 |
|------|------|------------|
| Windows | Windows Terminal | 1 |
| Windows | PowerShell | 2 |
| Windows | CMD | 3 |
| Linux | GNOME Terminal | 1 |
| Linux | Konsole | 2 |
| Linux | Alacritty | 3 |
| Linux | xterm | 4 |
| macOS | iTerm2 | 1 |
| macOS | Terminal.app | 2 |
| macOS | Alacritty | 3 |

### 5.2 终端配置

```yaml
terminal:
  preferred_terminal: "auto"  # 自动检测
  # 或指定具体终端
  # preferred_terminal: "iterm2"

  new_tab: true          # 在新标签页打开
  # new_window: false    # 或在新窗口打开

  custom_args: "-o StrictHostKeyChecking=no"
  # 额外 SSH 参数
```

### 5.3 自定义终端命令模板

```bash
# 默认模板
ssh -p {port} -i {identity_file} {user}@{host}

# 自定义模板示例 (iTerm2)
osascript -e 'tell application "iTerm" to create window with default profile command "ssh -p {port} {user}@{host}"'

# Windows Terminal 模板
wt.exe new-tab --title "{name}" ssh -p {port} {user}@{host}

# Alacritty 模板
alacritty --title "{name}" -e ssh -p {port} {user}@{host}
```

---

## 六、SSH Agent 集成 / SSH Agent Integration

### 6.1 自动密钥管理

```rust
// Agent 集成流程
pub async fn ensure_key_loaded(key_path: &Path, passphrase: Option<&str>) -> Result<()> {
    // 1. 检查密钥是否已在 agent 中
    if ssh_agent::has_key(key_path).await? {
        return Ok(());
    }

    // 2. 从 keychain 获取 passphrase
    let passphrase = keychain::get_passphrase(key_path).await?;

    // 3. 加载密钥到 agent
    ssh_agent::add_key(key_path, passphrase.as_deref()).await?;

    // 4. 设置自动过期 (可选)
    ssh_agent::set_lifetime(key_path, Duration::from_hours(1)).await?;

    Ok(())
}
```

### 6.2 支持的操作系统钥匙串

| 平台 | 钥匙串服务 | API |
|------|------------|-----|
| Windows | Windows Credential Manager | CredWrite/CredRead |
| Linux | Secret Service API / GNOME Keyring | D-Bus |
| Linux | KWallet | D-Bus |
| macOS | Keychain Services | Security.framework |

---

## 七、数据导入导出 / Data Import & Export

### 7.1 导出格式

```json
{
  "version": "0.3.0",
  "export_date": "2026-04-02T10:30:00Z",
  "encrypted": true,
  "kdf": "argon2id",
  "cipher": "aes-256-gcm",
  "salt": "base64_encoded_salt",
  "nonce": "base64_encoded_nonce",
  "ciphertext": "base64_encoded_encrypted_data"
}
```

### 7.2 从 ~/.ssh/config 导入

```bash
# 自动解析 SSH 配置文件
# 支持以下配置项:
# - Host
# - HostName
# - Port
# - User
# - IdentityFile
# - ProxyJump
# - ServerAliveInterval

导入结果:
✓ 成功导入 12 个主机配置
⚠ 跳过 2 个 (包含通配符 Host 模式)
⚠ 跳过 1 个 (已存在)
```

### 7.3 批量导入工具

```bash
# CSV 格式导入
name,host,port,username,auth_type,password/key_path
server1,192.168.1.1,22,root,password,secret123
server2,192.168.1.2,22,ubuntu,key,~/.ssh/id_rsa

# JSON 格式导入
[
  {
    "name": "server1",
    "host": "192.168.1.1",
    "port": 22,
    "username": "root",
    "auth": {
      "type": "password",
      "password": "secret123"
    }
  }
]
```

---

## 八、系统托盘集成 / System Tray Integration

### 8.1 托盘菜单功能

```
┌─────────────────────────────────────────────┐
│  🖥️  EasySSH Lite              [显示窗口]   │
├─────────────────────────────────────────────┤
│  🔍 快速连接                                 │
│  ├── 🖥️  生产服务器-01                        │
│  ├── 🖥️  生产服务器-02                        │
│  └── 🖥️  测试服务器                          │
├─────────────────────────────────────────────┤
│  📁 最近连接                                  │
│  ├── 🕐  10:30  生产服务器-01                  │
│  └── 🕐  09:15  测试服务器                    │
├─────────────────────────────────────────────┤
│  🔒 锁定应用                                 │
│  ⚙️  设置                                    │
├─────────────────────────────────────────────┤
│  ❌ 退出                                    │
└─────────────────────────────────────────────┘
```

### 8.2 全局快捷键

```
系统级快捷键 (可自定义):
├── Ctrl+Shift+S: 显示/隐藏窗口
├── Ctrl+Shift+Q: 快速连接菜单
├── Ctrl+Shift+L: 锁定应用
└── Ctrl+Shift+1-9: 连接第 N 个服务器
```

---

## Feature Details (English)

### Secure Storage
- **Argon2id**: OWASP-recommended password hashing
- **AES-256-GCM**: Military-grade symmetric encryption
- **System Keychain**: Hardware-level key isolation
- **Memory Protection**: SecureString / mlock to prevent swap leaks

### Server Management
- **Auth Methods**: Password, SSH Key (RSA/Ed25519/ECDSA), SSH Agent
- **Connection Options**: Timeout, keepalive, retry, compression
- **Batch Operations**: Edit multiple, move to group, export, delete

### Groups & Organization
- **Nested Groups**: Unlimited nesting depth
- **Drag & Drop**: Visual organization
- **Smart Sorting**: Manual, alphabetical, by usage, by tags

### Search & Filter
- **Fuzzy Search**: Approximate string matching
- **Advanced Filters**: Keywords, groups, tags, auth method, connection status
- **Search Syntax**: `tag:production`, `host:192.168`, `user:root`

### Terminal Integration
- **Auto-detection**: Windows Terminal, iTerm2, GNOME Terminal, etc.
- **Custom Templates**: Define your own SSH command patterns
- **New Tab/Window**: Configurable behavior

### SSH Agent
- **Auto-key Loading**: Automatically add keys to agent
- **Passphrase Caching**: Secure storage in system keychain
- **Key Lifetime**: Automatic expiration

---

## 截图占位符 / Screenshots

### 服务器列表 / Server List
```
[截图占位符: 主界面服务器列表和分组树]
[Screenshot placeholder: Main interface with server list and group tree]
```

### 添加服务器 / Add Server
```
[截图占位符: 添加服务器对话框，展示密码和密钥选项]
[Screenshot placeholder: Add server dialog with password and key options]
```

### 搜索过滤 / Search & Filter
```
[截图占位符: 搜索栏和过滤结果展示]
[Screenshot placeholder: Search bar and filtered results]
```

### 安全设置 / Security Settings
```
[截图占位符: 主密码设置界面]
[Screenshot placeholder: Master password setup screen]
```

---

**文档版本**: v0.3.0
**最后更新**: 2026-04-02
**适用版本**: EasySSH Lite v0.3.0+
