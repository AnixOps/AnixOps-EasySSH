# EasySSH

现代SSH客户端产品线 - Lite/Standard/Pro三版本

## 快速链接
- [完整文档](docs/INDEX.md)
- [开发指南](docs/developers/SETUP.md)
- [架构设计](docs/architecture/overall-architecture.md)
- [变更日志](CHANGELOG.md)
- [贡献指南](CONTRIBUTING.md)

## 项目状态

🟢 Beta版本开发中 - 预计2026-04-15发布

| 版本 | 状态 | 进度 |
|------|------|------|
| Lite | 🟡 开发中 | 80% |
| Standard | ⚪ 规划中 | 20% |
| Pro | ⚪ 规划中 | 10% |

## 目录结构

```
.
├── core/                   # 核心库 (Rust)
├── platforms/              # 原生UI实现
│   ├── windows/            # egui原生版本
│   ├── linux/              # GTK4原生版本
│   └── macos/              # SwiftUI原生版本
├── docs/                   # 完整文档
│   ├── architecture/       # 架构设计
│   ├── developers/         # 开发者指南
│   ├── standards/          # 开发标准
│   └── archives/           # 历史报告归档
├── tests/                  # 测试套件
├── examples/               # 示例代码
└── tools/                  # 开发工具
```

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

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件
