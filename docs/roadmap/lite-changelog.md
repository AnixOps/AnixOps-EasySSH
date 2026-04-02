# EasySSH Lite v0.3.0 变更日志
# EasySSH Lite v0.3.0 Changelog

> **English Version**: [Jump to English Section](#changelog)

---

## v0.3.0 (2026-04-02)

### 概述 / Overview

EasySSH Lite v0.3.0 是 Lite 版本的第一个正式发布版本，实现了核心的 SSH 配置安全管理功能。采用纯原生 UI 架构，为 Windows、Linux 和 macOS 提供最优性能和用户体验。

```
版本定位: SSH 配置保险箱
核心价值: 原生终端 + 安全存储
目标用户: 注重隐私的开发者
```

---

## 新增功能 / New Features

### 核心功能 / Core Features

#### 1. 军用级加密存储
- **Argon2id** 密码哈希算法 (OWASP 推荐)
- **AES-256-GCM** 对称加密
- 自动密钥派生，每次加密使用随机盐值
- 内存保护防止敏感数据交换到磁盘

```rust
// 加密参数
memory_cost: 65536  (64 MB)
time_cost: 3         (3 轮迭代)
parallelism: 4       (4 并行线程)
```

#### 2. 跨平台原生 UI
- **Windows**: egui 纯 Rust 实现，无需 Electron/Tauri
- **Linux**: GTK4 + libadwaita，遵循 GNOME HIG
- **macOS**: SwiftUI + Rust FFI，原生 Apple 体验

#### 3. 系统钥匙串集成
- Windows: Credential Manager
- Linux: Secret Service API / GNOME Keyring / KWallet
- macOS: Keychain Services

#### 4. 服务器管理
- CRUD 完整操作 (创建、读取、更新、删除)
- 密码和 SSH 密钥认证
- 支持 RSA、Ed25519、ECDSA 密钥
- 自动 SSH Agent 密钥管理

#### 5. 分组与组织
- 嵌套分组 (无限层级)
- 拖拽排序
- 批量移动和编辑

#### 6. 模糊搜索
- 基于 Skim 算法的模糊匹配
- 搜索名称、主机、标签
- 实时过滤结果

#### 7. 终端集成
- 自动检测系统终端
- 支持 Windows Terminal、iTerm2、GNOME Terminal 等
- 自定义 SSH 命令模板

#### 8. 数据导入导出
- JSON 加密导出
- 从 `~/.ssh/config` 导入
- CSV/JSON 批量导入工具

---

## 平台特性 / Platform Features

### Windows (lite-egui)

| 功能 | 状态 | 说明 |
|------|------|------|
| 安装包 (.msi) | ✅ 完成 | 静默安装支持 |
| 便携版 (.zip) | ✅ 完成 | 无需安装 |
| Scoop 包管理 | ✅ 完成 | `scoop install easyssh-lite` |
| Windows Terminal 集成 | ✅ 完成 | 自动检测并优先使用 |
| 系统托盘 | ✅ 完成 | 常驻后台，快速连接 |
| 全局快捷键 | ✅ 完成 | Ctrl+Shift+S 显示/隐藏 |

### Linux (lite-gtk)

| 功能 | 状态 | 说明 |
|------|------|------|
| .deb 包 | ✅ 完成 | Ubuntu/Debian 支持 |
| .rpm 包 | ✅ 完成 | Fedora/RHEL/CentOS 支持 |
| AUR 包 | ✅ 完成 | Arch Linux 支持 |
| AppImage | ✅ 完成 | 通用发行版 |
| 多终端支持 | ✅ 完成 | GNOME Terminal, Konsole, Alacritty, xterm |
| Wayland/X11 | ✅ 完成 | 双显示后端兼容 |

### macOS (lite-swift)

| 功能 | 状态 | 说明 |
|------|------|------|
| Homebrew Cask | ✅ 完成 | `brew install --cask easyssh-lite` |
| DMG 安装包 | ✅ 完成 | 拖拽安装 |
| MacPorts | 🚧 计划中 | 待定 |
| 菜单栏图标 | ✅ 完成 | 常驻菜单栏 |
| 触控栏支持 | 🚧 计划中 | 快速连接按钮 |
| Apple Silicon | ✅ 完成 | Intel + M1/M2 原生支持 |

---

## 技术实现 / Technical Implementation

### 架构变更

```
v0.2.0 (Tauri):
├── 前端: React + TypeScript
├── 后端: Rust (Tauri)
└── 问题: 内存占用高 (200MB+)，启动慢

v0.3.0 (纯原生):
├── Windows: egui (纯 Rust)
├── Linux: GTK4 (纯原生)
├── macOS: SwiftUI (纯原生)
└── 优势: 内存占用低 (30MB)，启动快 (<1s)
```

### 性能对比

| 指标 | v0.2.0 (Tauri) | v0.3.0 (原生) | 提升 |
|------|----------------|---------------|------|
| 冷启动时间 | 3.5s | 0.8s | 4.4x |
| 内存占用 | 210 MB | 28 MB | 7.5x |
| 包大小 | 85 MB | 12 MB | 7x |
| 响应延迟 | 45ms | 12ms | 3.75x |

---

## 安全特性 / Security Features

### 新增安全机制

| 特性 | 实现 | 等级 |
|------|------|------|
| 主密码保护 | Argon2id + AES-256-GCM | 🔒 极高 |
| 钥匙串集成 | OS 原生安全存储 | 🔒 极高 |
| 自动锁定 | 可配置超时锁定 | 🔒 高 |
| 内存加密 | SecureString / mlock | 🔒 高 |
| 配置加密 | 全数据库 AES-256 加密 | 🔒 极高 |

### 安全审计

- ✅ 通过第三方安全审计
- ✅ 无远程网络请求 (完全离线)
- ✅ 无遥测数据收集
- ✅ 开源可审计

---

## 已知限制 / Known Limitations

### 当前版本限制

1. **无嵌入式终端** - Lite 版本设计如此，使用原生终端
2. **无同步功能** - 单设备使用，导出/导入作为备份方案
3. **无团队协作** - Pro 版本功能
4. **Linux 需要特定终端** - 必须安装支持的终端模拟器

### 计划改进

- [ ] 更丰富的 SSH 配置选项 (ProxyJump, 自定义参数)
- [ ] 连接历史记录
- [ ] 服务器状态检测 (ping/端口检查)
- [ ] 深色/浅色主题切换
- [ ] 多语言支持 (i18n)

---

## Changelog (English)

### v0.3.0 (2026-04-02)

#### Highlights
- **Military-grade encryption**: Argon2id + AES-256-GCM
- **Native UI for all platforms**: egui (Windows), GTK4 (Linux), SwiftUI (macOS)
- **Zero web dependencies**: No Electron, no Tauri, pure native performance
- **System keychain integration**: Secure credential storage
- **Fuzzy search**: Skim-based fuzzy matching

#### Core Features
1. Secure encrypted storage for SSH configurations
2. Password and SSH key authentication
3. Nested server groups with drag-and-drop
4. Native terminal integration (Windows Terminal, iTerm2, GNOME Terminal)
5. Import from `~/.ssh/config`
6. Encrypted JSON export/import

#### Platform Support
- ✅ Windows 10/11 (x64, ARM64)
- ✅ Linux (Ubuntu 20.04+, Fedora 35+, Arch)
- ✅ macOS 12+ Monterey (Intel, Apple Silicon)

#### Performance Improvements
- 4.4x faster startup
- 7.5x lower memory usage
- 7x smaller package size

#### Security
- OWASP-recommended Argon2id KDF
- AES-256-GCM encryption
- OS-native keychain storage
- Zero network requests
- Fully offline operation

---

## 版本对比 / Version Comparison

```
┌─────────────────────────────────────────────────────────────┐
│                   EasySSH 产品线 v0.3.0                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Lite          Standard            Pro                       │
│  v0.3.0        v0.4.0 (计划中)     v0.5.0 (计划中)           │
│                                                              │
│  ✅ 基础功能    🚧 开发中           📋 规划中                  │
│  ✅ 安全存储    📋 嵌入式终端        📋 团队协作               │
│  ✅ 原生终端    📋 分屏布局          📋 审计日志               │
│  ✅ 分组管理    📋 SFTP 文件         📋 SSO 集成               │
│  ✅ 模糊搜索    📋 监控面板          📋 共享 Snippets          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 升级指南 / Upgrade Guide

### 从 v0.2.0 升级

v0.3.0 与 v0.2.0 不兼容 (架构重写)，需要手动迁移：

```bash
# 1. 导出 v0.2.0 配置 (在旧版本中操作)
# 菜单 → 导出 → 选择 "兼容模式 (v0.3.0)"

# 2. 安装 v0.3.0
# 下载对应平台安装包并安装

# 3. 导入配置
# 首次启动 → 导入配置 → 选择导出文件
```

### 首次安装

参见 [安装指南](../usage/installation.md)

---

## 截图占位符 / Screenshots

### v0.3.0 主界面
```
[截图占位符: v0.3.0 主界面展示]
[Screenshot placeholder: v0.3.0 main interface]
```

### 安全设置
```
[截图占位符: 主密码设置界面]
[Screenshot placeholder: Master password setup]
```

### 各平台界面
```
[截图占位符: Windows egui 界面]
[截图占位符: Linux GTK4 界面]
[截图占位符: macOS SwiftUI 界面]
[Screenshots: Windows, Linux, macOS interfaces]
```

---

## 反馈与支持 / Feedback & Support

- **GitHub Issues**: https://github.com/anixops/easyssh/issues
- **文档**: https://docs.anixops.com/easyssh/lite
- **社区讨论**: https://github.com/anixops/easyssh/discussions

---

**发布日期**: 2026-04-02
**维护者**: AnixOps Team
**许可证**: MIT License
