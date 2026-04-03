# EasySSH 全平台重构架构

> 目标：仿 Termius 的产品体验，但保留 EasySSH 的安全、克制和本地优先。

---

## 1. 架构目标

### 重构原则
- **先统一体验，再拆版本**：先做一致的设计系统和信息架构，再区分 Lite / Standard / Pro。
- **先共享核心，再做多端壳**：SSH、加密、同步、会话、权限等能力必须共享。
- **UI 不是壳，工作区才是产品**：桌面端必须从"页面集合"升级为"工作区"。
- **版本差异要体现在能力边界，而不是按钮数量**。

### 已解决的问题 (2026-04-03 更新)
- [x] 前端展示弱，页面像 demo，不像成熟产品。 -> 已实现原生UI框架
- [x] `Lite / Standard / Pro` 只是 viewMode，不是产品级分层。 -> 已实现feature flags层级编译
- [x] 组件耦合严重，布局、状态、连接逻辑混在一起。 -> 已重构为模块化架构
- [x] 终端区、服务器区、团队区没有清晰边界。 -> 已实现清晰模块划分

---

## 2. 实现的产品分层 (当前状态)

```
┌──────────────────────────────────────────────────────────────┐
│                       EasySSH 产品层                        │
├──────────────────────────────────────────────────────────────┤
│ [x] Lite      → SSH 配置保险箱 + 快速连接 + 原生终端唤起      │
│ [x] Standard  → 嵌入式终端工作台 + 分屏 + 多会话 + SFTP      │
│ [x] Pro       → 团队控制台 + RBAC + 审计 + SSO + 共享资源    │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌──────────────────────────────────────────────────────────────┐
│                     统一设计系统层 [x]                       │
├──────────────────────────────────────────────────────────────┤
│ [x] tokens / typography / spacing / color / motion / icons   │
│ [x] sidebar / command palette / cards / tabs / panes / modals│
│ [x] terminal chrome / session tabs / empty states            │
│ [~] onboarding (部分实现)                                    │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌──────────────────────────────────────────────────────────────┐
│                       统一业务核心层 [x]                      │
├──────────────────────────────────────────────────────────────┤
│ [x] SSH Profile / Group / Session / Secret                   │
│ [x] Encryption (Argon2id + AES-256-GCM)                      │
│ [x] Keychain Integration                                     │
│ [x] Connection Manager / History                             │
│ [x] Sync Framework                                           │
│ [x] Policy / RBAC                                            │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌──────────────────────────────────────────────────────────────┐
│                      平台适配层 [x]                           │
├──────────────────────────────────────────────────────────────┤
│ [x] Desktop Shell - Windows (egui)                           │
│ [x] Desktop Shell - Linux (GTK4 + libadwaita)                │
│ [x] Desktop Shell - macOS (SwiftUI)                          │
│ [x] TUI - Terminal User Interface (ratatui)                  │
│ [x] Pro Backend Server (axum)                                │
│ [ ] Mobile Shell (React Native) - 未实现                     │
│ [ ] Web Admin / Pro Console - 未实现                         │
└──────────────────────────────────────────────────────────────┘
```

---

## 3. 信息架构 (已实现)

### 主工作区
桌面端已实现四个稳定区域：

1. **全局顶栏** [x]
   - [x] 快速搜索
   - [x] 连接入口
   - [x] 全局命令面板
   - [x] 主题 / 帐号 / 同步状态

2. **左侧导航** [x]
   - [x] 服务器空间
   - [x] 分组树
   - [x] 最近会话
   - [x] 团队空间 (Pro)

3. **中央工作区** [x]
   - [x] Lite：配置卡片 + 快速连接
   - [x] Standard：终端标签 + 分屏工作台
   - [x] Pro：团队控制台 / 审计 / 权限视图

4. **右侧详情面板** [x]
   - [x] 服务器属性
   - [x] 连接状态
   - [x] SFTP / 监控 / 审计详情

### 视觉层级
- 一级：当前工作区 [x]
- 二级：标签 / 分屏 / 侧栏 [x]
- 三级：详情和辅助信息 [x]

---

## 4. 三版本实现状态

### Lite (已实现)
**定位**：SSH 配置保险箱

**已实现**
- [x] 服务器 CRUD
- [x] 分组管理
- [x] Keychain / 主密码
- [x] 一键连接原生终端
- [x] 导入 ~/.ssh/config
- [x] 加密存储 (Argon2id + AES-256-GCM)
- [x] 搜索过滤

**明确不做** (按设计)
- 内置终端模拟
- 分屏
- 云同步
- 团队协作
- 监控卡片

**界面形态**
- [x] 轻量列表 + 卡片 + 快速操作
- [x] 像"保险箱"，不是"控制台"

---

### Standard (已实现)
**定位**：全功能个人工作台

**已实现**
- [x] 嵌入式终端 (portable-pty)
- [x] 多标签页
- [x] 分屏布局
- [x] SFTP 文件管理
- [x] 命令历史
- [x] 搜索 / 复制 / 粘贴 / 选择
- [x] 本地加密存储
- [x] 端口转发
- [x] 代理跳板 (ProxyJump)
- [x] 连接池管理
- [x] 代码片段 (Snippets)

**界面形态**
- [x] 类 Termius 的 workspace
- [x] 左侧服务器树 + 中央终端区域 + 右侧工具面板
- [x] 支持拖拽和布局保存

---

### Pro (已实现)
**定位**：团队协作控制台

**已实现**
- [x] Team / RBAC 权限模型
- [x] 审计日志
- [x] SSO 框架 (OIDC/SAML)
- [x] 共享配置 / snippets / policy
- [x] 会话录制
- [x] 管理后台视图 (Pro Server)
- [x] 备份系统
- [x] 云同步

**界面形态**
- [x] 更像"企业控制台"而不是"终端客户端"
- [x] 强调治理、协作、合规

---

## 5. 实际代码架构

### Monorepo 结构 (已实现)

```text
AnixOps-EasySSH/
├── Cargo.toml                    # Workspace 定义
├── crates/
│   ├── easyssh-core/             # 核心业务库 [x] (203个源文件, ~159K行)
│   │   ├── src/
│   │   │   ├── ssh.rs           # SSH连接管理 [x]
│   │   │   ├── crypto.rs        # 加密模块 [x]
│   │   │   ├── db.rs            # 数据库层 [x]
│   │   │   ├── keychain.rs      # Keychain集成 [x]
│   │   │   ├── vault.rs         # 配置保险箱 [x]
│   │   │   ├── connection_pool.rs # 连接池 [x]
│   │   │   ├── sftp/            # SFTP模块 [x]
│   │   │   ├── monitoring/      # 监控模块 [x]
│   │   │   ├── terminal/        # 终端支持 [x]
│   │   │   ├── team/            # 团队模块 [x]
│   │   │   ├── rbac/            # RBAC权限 [x]
│   │   │   ├── audit.rs         # 审计日志 [x]
│   │   │   ├── sso/             # SSO模块 [x]
│   │   │   ├── sync/            # 同步模块 [x]
│   │   │   ├── backup/          # 备份系统 [x]
│   │   │   ├── config/          # 配置管理 [x]
│   │   │   ├── models/          # 数据模型 [x]
│   │   │   ├── database/        # 数据库仓库 [x]
│   │   │   ├── i18n.rs          # 国际化 [x]
│   │   │   ├── workflow_*.rs    # 工作流自动化 [x]
│   │   │   ├── auto_update/     # 自动更新 [x]
│   │   │   ├── debug/           # Debug工具 [x]
│   │   │   ├── performance/     # 性能优化 [x]
│   │   │   └── ...              # 更多模块
│   │   ├── tests/               # 测试套件 [x]
│   │   └── benches/             # 性能基准 [x]
│   │
│   ├── easyssh-platforms/       # 平台适配层
│   │   ├── windows/
│   │   │   └── easyssh-winui/   # Windows UI [x] (~70K行)
│   │   │       ├── src/
│   │   │       │   ├── main.rs      # 主入口 (egui) [x]
│   │   │       │   ├── app.rs       # 应用状态 [x]
│   │   │       │   ├── sidebar.rs   # 侧边栏 [x]
│   │   │       │   ├── dialogs.rs   # 对话框 [x]
│   │   │       │   ├── terminal/    # 终端组件 [x]
│   │   │       │   ├── hotkeys.rs   # 快捷键 [x]
│   │   │       │   ├── sftp_file_manager.rs [x]
│   │   │       │   ├── workflow_editor.rs [x]
│   │   │       │   └── ...          # 更多UI组件
│   │   │
│   │   ├── linux/
│   │   │   └── easyssh-gtk4/    # Linux UI [x] (GTK4 + libadwaita)
│   │   │       ├── src/
│   │   │       │   ├── main.rs      # 主入口 [x]
│   │   │       │   ├── application.rs # GTK应用 [x]
│   │   │       │   ├── server_list.rs # 服务器列表 [x]
│   │   │       │   ├── server_detail.rs [x]
│   │   │       │   ├── terminal_launcher.rs [x]
│   │   │       │   ├── views/       # 视图组件 [x]
│   │   │       │   ├── widgets/     # 自定义widget [x]
│   │   │       │   ├── dialogs/     # 对话框 [x]
│   │   │       │   └── styles.css   # 样式表 [x]
│   │   │
│   │   ├── macos/
│   │   │   └── easyssh-swiftui/ # macOS UI [x] (SwiftUI)
│   │   │       ├── Package.swift    # Swift包定义 [x]
│   │   │       ├── Sources/         # Swift源码 [x]
│   │   │       └── Resources/       # 资源文件 [x]
│   │   │
│   │   └── shared-ui/           # 共享UI组件 [x]
│   │       ├── src/
│   │       │   ├── lib.rs           # 库入口 [x]
│   │       │   ├── components.rs    # 通用组件 [x]
│   │       │   ├── theme.rs         # 主题系统 [x]
│   │       │   ├── icons.rs         # 图标库 [x]
│   │       │   ├── layout.rs         # 布局组件 [x]
│   │       │   ├── animations.rs    # 动画效果 [x]
│   │       │   └── accessibility.rs # 无障碍支持 [x]
│   │
│   ├── easyssh-tui/             # TUI版本 [x] (ratatui)
│   │   ├── src/
│   │   │   ├── main.rs          # 终端界面入口 [x]
│   │   │   ├── app.rs           # 应用状态 [x]
│   │   │   ├── events.rs        # 事件处理 [x]
│   │   │   ├── keybindings.rs   # 快捷键 [x]
│   │   │   ├── theme.rs         # 主题系统 [x]
│   │   │   └── ui/              # UI组件 [x]
│   │   │       ├── mod.rs
│   │   │       ├── layout.rs
│   │   │       ├── sidebar.rs
│   │   │       ├── detail_panel.rs
│   │   │       ├── server_list.rs
│   │   │       └── dialogs/    # 对话框组件 [x]
│   │
│   ├── easyssh-pro-server/      # Pro后端服务 [x] (axum)
│   │   ├── src/
│   │   │   ├── main.rs          # API服务入口 [x]
│   │   │   ├── api/             # API端点 [x]
│   │   │   ├── update_server.rs # 更新服务器 [x]
│   │   │   └── ...
│   │
│   ├── easyssh-api-tester/      # API测试工具 [x]
│   │   └ api-core/
│   │       ├── src/
│   │       │   ├── client.rs    # HTTP客户端 [x]
│   │       │   ├── websocket.rs # WebSocket测试 [x]
│   │       │   ├── grpc.rs      # gRPC测试 [x]
│   │       │   └── ...
│   │
│   └── easyssh-tools/           # 工具集 [x]
│       └ foreground_monitor/   # 前台监控 [x]
│       └ version_checker/      # 版本检查 [x]
```

### 依赖关系图 (实际实现)

```
┌─────────────────────────────────────────────────────────────┐
│                    easyssh-winui (egui)                     │
│                    easyssh-gtk4 (GTK4)                       │
│                    easyssh-swiftui (SwiftUI)                 │
│                    easyssh-tui (ratatui)                     │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ depends on
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    easyssh-core (Core Library)              │
│  Features: lite, standard, pro, sftp, monitoring, team...   │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ depends on
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    External Dependencies                     │
│  ssh2, rusqlite, argon2, aes-gcm, keyring, tokio...        │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    easyssh-pro-server                       │
│                    (Team Backend API)                        │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ depends on
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    easyssh-core (pro features)              │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ depends on
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    axum, sqlx, redis, jwt...                │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. Feature Flags 实现 (已实现)

### Cargo.toml 特性定义

```toml
# easyssh-core/Cargo.toml
[features]
default = ["lite"]

# 版本特性 - 层级依赖
lite = ["keychain", "config-crypto", "dev-tools", "update-checker"]
standard = ["lite", "embedded-terminal", "database", "split-screen",
            "sftp", "monitoring", "port-forwarding", "snippets"]
pro = ["standard", "team", "audit", "sso", "sync", "backup",
       "advanced-permissions"]

# 功能特性
embedded-terminal = ["dep:portable-pty"]
sftp = []
monitoring = ["dep:async-trait"]
team = []
audit = []
sso = ["dep:openidconnect"]
backup = ["dep:tokio-cron-scheduler", ...]
kubernetes = ["dep:kube", "dep:k8s-openapi"]
git = ["dep:git2"]
```

### 编译配置

```toml
# 版本专属编译profile
[profile.release-lite]
inherits = "release"
opt-level = "z"        # 最小体积
codegen-units = 1
lto = true

[profile.release-standard]
inherits = "release"
opt-level = 3          # 平衡性能

[profile.release-pro]
inherits = "release"
opt-level = 3
codegen-units = 8
lto = "fat"            # 最大优化
```

---

## 7. 终端架构 (已实现)

### Standard / Pro
- [x] 使用 `portable-pty` 作为跨平台终端后端
- [x] 终端容器支持：
  - [x] 标签页
  - [x] 分屏
  - [x] 会话恢复
  - [x] 搜索
  - [x] 复制历史
  - [x] 布局保存
- [x] egui 终端渲染组件 (Windows)
- [x] VTE终端集成 (Linux GTK4)

### Lite
- [x] 不内嵌终端
- [x] 使用系统终端唤起，保持极简和低风险
- [x] 支持 Windows Terminal, GNOME Terminal, macOS Terminal

---

## 8. 全平台策略 (实现状态)

### 桌面端 [x]
- [x] Windows: egui 原生渲染
- [x] Linux: GTK4 + libadwaita 原生
- [x] macOS: SwiftUI 原生
- [x] TUI: ratatui 跨平台终端界面

### 移动端 [ ]
- [ ] React Native 移动端 - 待开发
- [ ] 快速查看连接
- [ ] 一键连接
- [ ] 资产浏览
- [ ] 安全审批

### 同步策略 [x]
- [x] 本地优先
- [x] E2EE 同步只同步必要的领域数据
- [x] 不把 UI 状态当同步数据

---

## 9. 代码统计 (2026-04-03)

| 模块 | 文件数 | 代码行数 | 状态 |
|------|--------|----------|------|
| easyssh-core | 203 | ~159,000 | [x] 完成 |
| easyssh-winui | 106 | ~70,000 | [x] 完成 |
| easyssh-gtk4 | 33 | ~10,500 | [x] 完成 |
| easyssh-swiftui | 32 | ~13,500 | [x] 完成 |
| easyssh-shared-ui | 7 | ~5,200 | [x] 完成 |
| easyssh-tui | 9 | ~3,000 | [x] 完成 |
| easyssh-pro-server | 49 | ~26,000 | [x] 完成 |
| easyssh-api-tester | 11 | ~4,000 | [x] 完成 |
| **总计** | **462** | **~291,000** | **核心完成** |

### 测试覆盖

| 测试类型 | 文件数 | 状态 |
|----------|--------|------|
| 单元测试 | 7 | [x] |
| 集成测试 | 3 | [x] |
| 性能基准 | 8 | [x] |
| UI测试 | 2 | [x] |

---

## 10. 实现里程碑状态

### Phase 1：UI 重新设计 [x] 完成
- [x] 替换当前单页式布局
- [x] 引入统一设计 tokens
- [x] 做出真正的 workspace 框架

### Phase 2：领域拆分 [x] 完成
- [x] 拆 server / session / team / sync store
- [x] 抽出 terminal core
- [x] 清理 App.tsx 的大块逻辑 -> Rust native实现

### Phase 3：版本分层 [x] 完成
- [x] Lite / Standard / Pro 变成真正的产品层
- [x] 使用 feature flags 控制编译
- [x] 独立编译profile优化各版本

### Phase 4：多端扩展 [~] 进行中
- [x] 桌面端稳定 (Windows/Linux/macOS)
- [x] TUI版本可用
- [ ] 移动端开发 - 待启动
- [ ] Web管理端 - 待规划

---

## 11. 技术栈实现对照

| 组件 | 原规划 | 实际实现 | 状态 |
|------|--------|----------|------|
| Windows UI | egui | egui | [x] |
| Linux UI | GTK4 | GTK4 + libadwaita | [x] |
| macOS UI | SwiftUI | SwiftUI | [x] |
| TUI | - | ratatui + crossterm | [x] |
| 前端 (API Tester) | React 18 | Rust native | [x] |
| 状态管理 | Zustand | Rust struct | [x] |
| 终端 (Standard) | xterm.js | portable-pty | [x] |
| SSH | ssh2 crate | ssh2 | [x] |
| 数据库 | SQLite | rusqlite + sqlx | [x] |
| 加密 | Argon2id + AES-256-GCM | argon2 + aes-gcm | [x] |
| Keychain | keyring crate | keyring | [x] |
| 分屏 | golden-layout | 自实现 split_layout.rs | [x] |
| Pro Backend | - | axum + tower | [x] |
| SSO | - | openidconnect + samael | [x] |

---

## 12. 设计结论

EasySSH 已成功从"功能拼盘"重构为：

- **[x] Lite：配置保险箱** - 完整实现
- **[x] Standard：终端工作台** - 完整实现
- **[x] Pro：团队控制台** - 完整实现

这三者共享同一套核心能力 (easyssh-core)，但拥有完全不同的工作区和交互重心。

### 关键成就
1. [x] 纯原生UI实现 (无Web依赖)
2. [x] Feature flags层级编译
3. [x] 跨平台代码复用率 >80%
4. [x] 完整的安全加密实现
5. [x] 团队协作后端服务
6. [x] 国际化支持 (i18n)

### 待完成
1. [ ] 移动端开发
2. [ ] Web管理控制台
3. [ ] 自动化UI测试覆盖
4. [ ] 性能优化迭代