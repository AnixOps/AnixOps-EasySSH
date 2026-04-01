# EasySSH

[![Crates.io](https://img.shields.io/crates/v/easyssh-core)](https://crates.io/crates/easyssh-core)
[![Docs.rs](https://docs.rs/easyssh-core/badge.svg)](https://docs.rs/easyssh-core)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://rust-lang.org)

> 企业级跨平台 SSH 客户端核心库，支持 Lite、Standard、Pro 三版本战略

## 产品定位

| 版本 | 定位 | 核心价值 | 目标用户 |
|------|------|----------|----------|
| **Lite** | SSH配置保险箱 | 原生终端 + 安全存储 | 注重隐私的开发者 |
| **Standard** | 全功能客户端 | 嵌入式终端 + 分屏 + 监控 | 多服务器管理者 |
| **Pro** | 团队协作平台 | 团队管理 + 审计 + SSO | IT团队/企业 |

## 功能矩阵

### SSH连接

| 功能 | Lite | Standard | Pro |
|------|------|----------|-----|
| 密码/密钥认证 | ✓ | ✓ | ✓ |
| SSH Agent | ✓ | ✓ | ✓ |
| Agent转发 | - | ✓ | ✓ |
| ProxyJump | - | ✓ | ✓ |
| 自动重连 | - | ✓ | ✓ |

### 终端

| 功能 | Lite | Standard | Pro |
|------|------|----------|-----|
| 原生终端唤起 | ✓ | - | - |
| 嵌入式Web终端 | - | ✓ | ✓ |
| 多标签页 | - | ✓ | ✓ |
| 分屏 | - | ✓ | ✓ |
| WebGL加速 | - | ✓ | ✓ |

### 管理

| 功能 | Lite | Standard | Pro |
|------|------|----------|-----|
| 服务器分组 | ✓ (单层) | ✓ (嵌套) | ✓ (团队) |
| 批量操作 | - | ✓ | ✓ |
| 导入~/.ssh/config | - | ✓ | ✓ |

### 安全

| 功能 | Lite | Standard | Pro |
|------|------|----------|-----|
| Keychain集成 | ✓ | ✓ | ✓ |
| 主密码保护 | ✓ | - | - |
| 配置加密(E2EE) | - | ✓ | ✓ |
| 审计日志 | - | - | ✓ |

## 核心技术栈

| 组件 | 技术选型 |
|------|----------|
| 框架 | Tauri 2.x / GTK4 / WinUI3 |
| 前端 | React 18 + TypeScript + Vite |
| 状态管理 | Zustand |
| 终端 (Standard) | xterm.js + xterm-addon-webgl |
| SSH | ssh2 crate / russh |
| 数据库 | SQLite |
| 加密 | Argon2id + AES-256-GCM |
| Keychain | keyring crate |
| 分屏 | golden-layout |

## 快速开始

### 安装

```bash
# 从 crates.io 安装核心库
cargo add easyssh-core

# 或使用特定功能
cargo add easyssh-core --features "standard sftp"
```

### 基本使用

```rust
use easyssh_core::{AppState, init_database, get_servers};

// 初始化应用状态
let state = AppState::new();

// 初始化数据库
init_database(&state).expect("Failed to initialize database");

// 获取所有服务器
let servers = get_servers(&state).expect("Failed to get servers");
```

### SSH 连接

```rust
use easyssh_core::{AppState, ssh_connect, init_database};

async fn connect_example() {
    let state = AppState::new();
    init_database(&state).unwrap();

    // 连接到服务器
    let metadata = ssh_connect(&state, "server-1", None).await.unwrap();
    println!("Connected: {}", metadata.id);
}
```

## 功能特性

### Feature Flags

```toml
[dependencies]
easyssh-core = { version = "0.3", features = ["standard", "sftp"] }
```

| Feature | 描述 |
|---------|------|
| `lite` | Lite 版本功能 (默认) |
| `standard` | Standard 版本功能 (嵌入式终端、SFTP) |
| `pro` | Pro 版本功能 (团队管理、RBAC、SSO) |
| `sftp` | SFTP 文件传输支持 |
| `split-screen` | 终端分屏功能 |
| `monitoring` | 服务器监控功能 |
| `docker` | Docker 容器管理 |
| `kubernetes` | Kubernetes 集群管理 |
| `backup` | 配置备份系统 |
| `sync` | 多端同步功能 |
| `auto-update` | 自动更新功能 |
| `workflow` | 工作流自动化 |
| `telemetry` | 遥测分析 |

## 架构设计

```
应用层 (Tauri/GTK4/WinUI)
    │
    ▼
核心服务层 (SSH, Crypto, Database)
    │
    ▼
平台适配层 (Keychain, Terminal, OS)
```

## 文档

- [API 文档](https://docs.rs/easyssh-core)
- [架构设计](docs/architecture/overall-architecture.md)
- [Lite 版本规划](docs/easyssh-lite-planning.md)
- [Standard 版本规划](docs/easyssh-standard-planning.md)
- [Pro 版本规划](docs/easyssh-pro-planning.md)
- [竞品分析](docs/competitor-analysis.md)

## 开发指南

### 构建

```bash
# 构建 Lite 版本
cargo build --package easyssh-core --features lite

# 构建 Standard 版本
cargo build --package easyssh-core --features standard

# 构建 Pro 版本
cargo build --package easyssh-core --features pro
```

### 测试

```bash
# 运行测试
cargo test --package easyssh-core

# 运行特定功能测试
cargo test --package easyssh-core --features standard
```

## 贡献指南

我们欢迎所有形式的贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解如何参与项目。

## 安全

- 使用 Argon2id 进行密钥派生
- AES-256-GCM 加密存储
- 操作系统 Keychain 集成
- 完整的审计日志 (Pro)

报告安全问题请发送邮件至 security@easyssh.dev

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 致谢

感谢所有贡献者为 EasySSH 项目付出的努力！
