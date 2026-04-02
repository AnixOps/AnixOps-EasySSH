# EasySSH

> 现代SSH客户端产品线 - Lite/Standard/Pro三版本

**[English](#english) | [中文](#中文)**

---

# English

## Overview

EasySSH is a product line of modern SSH clients designed for developers and teams. It offers three editions to meet different needs:

| Edition | Positioning | Core Value | Target Users |
|---------|-------------|------------|--------------|
| **Lite** | SSH Configuration Vault | Native terminal + secure storage | Privacy-focused developers |
| **Standard** | Full-Featured Client | Embedded terminal + split-screen + monitoring | Multi-server managers |
| **Pro** | Team Collaboration Platform | Team management + audit + SSO | IT teams / Enterprise |

## Project Status

🟢 Beta Development - Expected Release: 2026-04-15

| Version | Status | Progress |
|---------|--------|----------|
| Lite | 🟡 In Development | 80% |
| Standard | ⚪ Planning | 20% |
| Pro | ⚪ Planning | 10% |

## Quick Links

- [📚 Full Documentation](docs/INDEX.md)
- [🔧 Developer Guide](docs/developers/SETUP.md)
- [🏗️ Architecture](docs/architecture/overall-architecture.md)
- [📝 Changelog](CHANGELOG.md)
- [🤝 Contributing](CONTRIBUTING.md)
- [🇨🇳 中文版文档](#中文)

## Tech Stack

| Component | Technology |
|-----------|------------|
| Windows UI | egui (Pure Rust Native) |
| Linux UI | GTK4 (Pure Native) |
| macOS UI | SwiftUI (Pure Native) |
| Frontend (API Tester) | React 18 + TypeScript |
| State Management | Zustand |
| Terminal (Standard) | xterm.js + xterm-addon-webgl |
| SSH | ssh2 crate / russh |
| Database | SQLite |
| Encryption | Argon2id + AES-256-GCM |
| Keychain | keyring crate |
| Split Screen | golden-layout |

## Quick Start

### Installation

```bash
# Install from crates.io
cargo add easyssh-core

# Or with specific features
cargo add easyssh-core --features "standard sftp"
```

### Basic Usage

```rust
use easyssh_core::{AppState, init_database, get_servers};

// Initialize app state
let state = AppState::new();

// Initialize database
init_database(&state).expect("Failed to initialize database");

// Get all servers
let servers = get_servers(&state).expect("Failed to get servers");
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `lite` | Lite version features (default) |
| `standard` | Standard version features (embedded terminal, SFTP) |
| `pro` | Pro version features (team management, RBAC, SSO) |
| `sftp` | SFTP file transfer support |
| `split-screen` | Terminal split-screen feature |
| `monitoring` | Server monitoring feature |
| `docker` | Docker container management |
| `kubernetes` | Kubernetes cluster management |
| `backup` | Configuration backup system |
| `sync` | Multi-device sync feature |
| `workflow` | Workflow automation |

## Directory Structure

```
.
├── crates/
│   ├── easyssh-core/           # Core library (Rust)
│   ├── easyssh-platforms/      # Native UI implementations
│   │   ├── windows/            # egui native version
│   │   ├── linux/              # GTK4 native version
│   │   └── macos/              # SwiftUI native version
│   ├── easyssh-api-tester/     # API tester (React)
│   └── easyssh-pro-server/     # Pro backend service
├── docs/                       # Full documentation
│   ├── architecture/           # Architecture design
│   ├── developers/             # Developer guides
│   ├── standards/              # Development standards
│   └── archives/               # Historical reports
├── tests/                      # Test suites
├── examples/                   # Example code
└── tools/                      # Development tools
```

## License

MIT License - See [LICENSE](LICENSE) file

---

# 中文

## 项目简介

EasySSH 是一个面向开发者和团队的现代 SSH 客户端产品线，提供三个版本满足不同需求：

| 版本 | 定位 | 核心价值 | 目标用户 |
|------|------|----------|----------|
| **Lite** | SSH配置保险箱 | 原生终端 + 安全存储 | 注重隐私的开发者 |
| **Standard** | 全功能客户端 | 嵌入式终端 + 分屏 + 监控 | 多服务器管理者 |
| **Pro** | 团队协作平台 | 团队管理 + 审计 + SSO | IT团队/企业 |

## 项目状态

🟢 Beta版本开发中 - 预计2026-04-15发布

| 版本 | 状态 | 进度 |
|------|------|------|
| Lite | 🟡 开发中 | 80% |
| Standard | ⚪ 规划中 | 20% |
| Pro | ⚪ 规划中 | 10% |

## 快速链接

- [📚 完整文档](docs/INDEX.md)
- [🔧 开发指南](docs/developers/SETUP.md)
- [🏗️ 架构设计](docs/architecture/overall-architecture.md)
- [📝 变更日志](CHANGELOG.md)
- [🤝 贡献指南](CONTRIBUTING.md)

## 核心技术栈

| 组件 | 技术选型 |
|------|----------|
| Windows UI | egui (纯Rust原生) |
| Linux UI | GTK4 (纯原生) |
| macOS UI | SwiftUI (纯原生) |
| 前端 (API Tester) | React 18 + TypeScript |
| 状态管理 | Zustand |
| 终端 | xterm.js + xterm-addon-webgl |
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

## 功能特性

### Feature Flags

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
| `workflow` | 工作流自动化 |

## 目录结构

```
.
├── crates/
│   ├── easyssh-core/           # 核心库 (Rust)
│   ├── easyssh-platforms/      # 原生UI实现
│   │   ├── windows/            # egui原生版本
│   │   ├── linux/              # GTK4原生版本
│   │   └── macos/              # SwiftUI原生版本
│   ├── easyssh-api-tester/     # API测试器 (React)
│   └── easyssh-pro-server/     # Pro后端服务
├── docs/                       # 完整文档
│   ├── architecture/           # 架构设计
│   ├── developers/             # 开发者指南
│   ├── standards/              # 开发标准
│   └── archives/               # 历史报告归档
├── tests/                      # 测试套件
├── examples/                   # 示例代码
└── tools/                      # 开发工具
```

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件
