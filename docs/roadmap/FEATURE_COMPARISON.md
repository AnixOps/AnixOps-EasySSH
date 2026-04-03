# EasySSH Feature Comparison Matrix
# EasySSH 功能对比矩阵

> Compare features across Lite, Standard, and Pro editions
> 对比 Lite、Standard 和 Pro 三个版本的功能差异

---

## Table of Contents / 目录

1. [SSH Connection Features / SSH 连接功能](#1-ssh-connection-features--ssh-连接功能)
2. [Terminal Features / 终端功能](#2-terminal-features--终端功能)
3. [Management Features / 管理功能](#3-management-features--管理功能)
4. [Security Features / 安全功能](#4-security-features--安全功能)
5. [Team Features / 团队功能](#5-team-features--团队功能)
6. [Pricing / 定价](#6-pricing--定价)
7. [Recommendations / 版本选择建议](#7-recommendations--版本选择建议)

---

## 1. SSH Connection Features / SSH 连接功能

| Feature / 功能 | Lite | Standard | Pro |
|----------------|------|----------|-----|
| **Password Authentication** | ✓ | ✓ | ✓ |
| 密码认证 | ✓ | ✓ | ✓ |
| **SSH Key Authentication** | ✓ | ✓ | ✓ |
| SSH 密钥认证 | ✓ | ✓ | ✓ |
| **SSH Agent Support** | ✓ | ✓ | ✓ |
| SSH Agent 支持 | ✓ | ✓ | ✓ |
| **Agent Forwarding** | - | ✓ | ✓ |
| Agent 转发 | - | ✓ | ✓ |
| **ProxyJump / Jump Host** | - | ✓ | ✓ |
| 跳板机支持 | - | ✓ | ✓ |
| **Auto-reconnect** | - | ✓ | ✓ |
| 自动重连 | - | ✓ | ✓ |
| **Connection Keep-alive** | ✓ | ✓ | ✓ |
| 连接保活 | ✓ | ✓ | ✓ |
| **Multiple Sessions** | ✓ | ✓ | ✓ |
| 多会话管理 | ✓ | ✓ | ✓ |
| **Connection Proxy** | - | ✓ | ✓ |
| 连接代理 (HTTP/SOCKS) | - | ✓ | ✓ |

### Details / 详细说明

- **Password/Key Auth / 密码/密钥认证**: All editions support standard password and SSH key-based authentication. Keys are securely stored in the system keychain.
  所有版本均支持标准密码和 SSH 密钥认证。密钥安全存储在系统钥匙串中。

- **SSH Agent / SSH 代理**: Full support for SSH agent protocol, allowing secure key management without exposing private keys.
  完整支持 SSH agent 协议，无需暴露私钥即可安全管理密钥。

- **Agent Forwarding / Agent 转发**: (Standard/Pro) Forward authentication requests to remote servers for seamless multi-hop connections.
  (Standard/Pro) 将认证请求转发到远程服务器，实现无缝多跳连接。

- **ProxyJump / 跳板机**: (Standard/Pro) Connect through bastion hosts or jump servers with automatic configuration.
  (Standard/Pro) 通过堡垒机或跳板服务器连接，自动配置连接参数。

- **Auto-reconnect / 自动重连**: (Standard/Pro) Automatically restore dropped connections with configurable retry logic.
  (Standard/Pro) 自动恢复断开的连接，支持可配置的重试逻辑。

---

## 2. Terminal Features / 终端功能

| Feature / 功能 | Lite | Standard | Pro |
|----------------|------|----------|-----|
| **Native Terminal Launch** | ✓ | - | - |
| 原生终端唤起 | ✓ | - | - |
| **Embedded Web Terminal** | - | ✓ | ✓ |
| 嵌入式 Web 终端 | - | ✓ | ✓ |
| **Multi-tab Support** | - | ✓ | ✓ |
| 多标签页支持 | - | ✓ | ✓ |
| **Split Screen / Panes** | - | ✓ | ✓ |
| 分屏 / 面板 | - | ✓ | ✓ |
| **WebGL Acceleration** | - | ✓ | ✓ |
| WebGL 加速 | - | ✓ | ✓ |
| **Terminal Themes** | ✓ | ✓ | ✓ |
| 终端主题 | ✓ | ✓ | ✓ |
| **Custom Fonts** | ✓ | ✓ | ✓ |
| 自定义字体 | ✓ | ✓ | ✓ |
| **Copy/Paste Support** | ✓ | ✓ | ✓ |
| 复制/粘贴支持 | ✓ | ✓ | ✓ |
| **Terminal Search** | ✓ | ✓ | ✓ |
| 终端搜索 | ✓ | ✓ | ✓ |
| **Session Recording** | - | ✓ | ✓ |
| 会话录制 | - | ✓ | ✓ |
| **Keyboard Shortcuts** | ✓ | ✓ | ✓ |
| 键盘快捷键 | ✓ | ✓ | ✓ |

### Details / 详细说明

- **Native Terminal / 原生终端**: (Lite) Opens connections in your preferred system terminal (Terminal.app, iTerm2, Windows Terminal, etc.).
  (Lite) 在您偏好的系统终端中打开连接（Terminal.app、iTerm2、Windows Terminal 等）。

- **Embedded Terminal / 嵌入式终端**: (Standard/Pro) Full-featured terminal emulator built into the application with xterm.js and WebGL acceleration.
  (Standard/Pro) 内置于应用中的全功能终端模拟器，基于 xterm.js 和 WebGL 加速。

- **Split Screen / 分屏**: (Standard/Pro) Divide your workspace into multiple panes for simultaneous server monitoring.
  (Standard/Pro) 将工作区划分为多个面板，同时监控多台服务器。

- **WebGL Acceleration / WebGL 加速**: (Standard/Pro) GPU-accelerated rendering for smooth performance with high-frequency output.
  (Standard/Pro) GPU 加速渲染，高频输出时保持流畅性能。

---

## 3. Management Features / 管理功能

| Feature / 功能 | Lite | Standard | Pro |
|----------------|------|----------|-----|
| **Server Grouping** | ✓ (Single-level) | ✓ (Nested) | ✓ (Team) |
| 服务器分组 | ✓ (单层) | ✓ (嵌套) | ✓ (团队) |
| **Batch Operations** | - | ✓ | ✓ |
| 批量操作 | - | ✓ | ✓ |
| **Import SSH Config** | - | ✓ | ✓ |
| 导入 SSH 配置 | - | ✓ | ✓ |
| **Export SSH Config** | ✓ | ✓ | ✓ |
| 导出 SSH 配置 | ✓ | ✓ | ✓ |
| **Quick Search** | ✓ | ✓ | ✓ |
| 快速搜索 | ✓ | ✓ | ✓ |
| **Tags & Labels** | ✓ | ✓ | ✓ |
| 标签与标记 | ✓ | ✓ | ✓ |
| **Snippets** | - | ✓ | ✓ |
| 代码片段 | - | ✓ | ✓ |
| **Connection Profiles** | ✓ | ✓ | ✓ |
| 连接配置文件 | ✓ | ✓ | ✓ |
| **Favorites** | ✓ | ✓ | ✓ |
| 收藏夹 | ✓ | ✓ | ✓ |

### Details / 详细说明

- **Server Grouping / 服务器分组**:
  - Lite: Simple single-level folders for organizing servers.
    Lite: 简单的单层文件夹组织服务器。
  - Standard: Nested folder structure for complex hierarchies.
    Standard: 嵌套文件夹结构，支持复杂层级。
  - Pro: Team-based organization with shared groups.
    Pro: 基于团队的组织，支持共享分组。

- **Batch Operations / 批量操作**: (Standard/Pro) Execute commands or scripts across multiple servers simultaneously.
  (Standard/Pro) 在多台服务器上同时执行命令或脚本。

- **Import SSH Config / 导入 SSH 配置**: (Standard/Pro) Automatically import existing `~/.ssh/config` with full parsing support.
  (Standard/Pro) 自动导入现有 `~/.ssh/config`，完整解析支持。

- **Snippets / 代码片段**: (Standard/Pro) Store and quickly access frequently used commands and scripts.
  (Standard/Pro) 存储并快速访问常用命令和脚本。

---

## 4. Security Features / 安全功能

| Feature / 功能 | Lite | Standard | Pro |
|----------------|------|----------|-----|
| **Keychain Integration** | ✓ | ✓ | ✓ |
| 系统钥匙串集成 | ✓ | ✓ | ✓ |
| **Master Password** | ✓ | - | - |
| 主密码保护 | ✓ | - | - |
| **Config Encryption (E2EE)** | - | ✓ | ✓ |
| 配置加密 (端到端加密) | - | ✓ | ✓ |
| **Audit Logs** | - | - | ✓ |
| 审计日志 | - | - | ✓ |
| **Session Recording** | - | ✓ | ✓ |
| 会话录制 | - | ✓ | ✓ |
| **Password Policy** | - | - | ✓ |
| 密码策略 | - | - | ✓ |
| **Credential Rotation** | - | - | ✓ |
| 凭证轮换 | - | - | ✓ |
| **Secure Clipboard** | ✓ | ✓ | ✓ |
| 安全剪贴板 | ✓ | ✓ | ✓ |
| **Auto-lock** | ✓ | ✓ | ✓ |
| 自动锁定 | ✓ | ✓ | ✓ |

### Details / 详细说明

- **Keychain Integration / 系统钥匙串集成**: All editions integrate with the native system keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service).
  所有版本均与原生系统钥匙串集成（Windows 凭据管理器、macOS 钥匙串、Linux Secret Service）。

- **Master Password / 主密码**: (Lite) Additional protection layer with Argon2id-derived encryption key.
  (Lite) 使用 Argon2id 派生加密密钥的额外保护层。

- **Config Encryption / 配置加密**: (Standard/Pro) End-to-end encryption for all configuration data synced across devices.
  (Standard/Pro) 所有配置数据的端到端加密，支持跨设备同步。

- **Audit Logs / 审计日志**: (Pro) Comprehensive logging of all access and operations for compliance requirements.
  (Pro) 完整记录所有访问和操作，满足合规要求。

---

## 5. Team Features / 团队功能

| Feature / 功能 | Lite | Standard | Pro |
|----------------|------|----------|-----|
| **Team Management** | - | - | ✓ |
| 团队管理 | - | - | ✓ |
| **Member Invitation** | - | - | ✓ |
| 成员邀请 | - | - | ✓ |
| **RBAC Permissions** | - | - | ✓ |
| RBAC 权限控制 | - | - | ✓ |
| **SSO (SAML/OIDC)** | - | - | ✓ |
| 单点登录 (SAML/OIDC) | - | - | ✓ |
| **Shared Snippets** | - | - | ✓ |
| 共享代码片段 | - | - | ✓ |
| **Shared Servers** | - | - | ✓ |
| 共享服务器 | - | - | ✓ |
| **Team Audit Logs** | - | - | ✓ |
| 团队审计日志 | - | - | ✓ |
| **API Access** | - | - | ✓ |
| API 访问 | - | - | ✓ |
| **Priority Support** | - | - | ✓ |
| 优先支持 | - | - | ✓ |

### Details / 详细说明

- **Team Management / 团队管理**: (Pro) Create and manage teams with full administrative controls.
  (Pro) 创建和管理团队，提供完整的管理控制。

- **RBAC / 基于角色的访问控制**: (Pro) Fine-grained permission system with roles like Admin, Operator, Viewer.
  (Pro) 细粒度权限系统，支持管理员、操作员、查看者等角色。

- **SSO / 单点登录**: (Pro) Enterprise SSO integration with SAML 2.0 and OpenID Connect providers.
  (Pro) 企业级 SSO 集成，支持 SAML 2.0 和 OpenID Connect 提供商。

- **Shared Resources / 共享资源**: (Pro) Share servers, snippets, and configurations across team members.
  (Pro) 在团队成员间共享服务器、代码片段和配置。

---

## 6. Pricing / 定价

| Edition / 版本 | Price / 价格 | Billing / 计费周期 |
|----------------|--------------|-------------------|
| **Lite** | Free / 免费 | - |
| Lite | 免费 | - |
| **Standard** | TBD / 待定 | Monthly / Yearly 月付 / 年付 |
| Standard | 待定 | 月付 / 年付 |
| **Pro** | TBD / 待定 | Per-seat, Monthly / Yearly 按席位, 月付 / 年付 |
| Pro | 待定 | 按席位, 月付 / 年付 |

### Notes / 说明

- Lite is and will remain free for personal use.
  Lite 版本将永久免费供个人使用。

- Standard pricing will be announced closer to launch.
  Standard 版本定价将在发布前公布。

- Pro pricing is per-seat with team management features.
  Pro 版本按席位定价，包含团队管理功能。

- Educational and open-source discounts available.
  提供教育和开源项目折扣。

---

## 7. Recommendations / 版本选择建议

### Choose Lite if / 选择 Lite 版本，如果您：

| Scenario / 场景 | Reason / 原因 |
|------------------|---------------|
| Individual developer / 个人开发者 | No need for team features / 无需团队功能 |
| Privacy-focused / 注重隐私 | Master password + native terminal keeps everything local / 主密码 + 原生终端，数据完全本地 |
| Minimal resource usage / 最小资源占用 | Lightweight, uses system terminal / 轻量级，使用系统终端 |
| Quick SSH access / 快速 SSH 访问 | Simple config management / 简单的配置管理 |
| Learning SSH / 学习 SSH | Free, easy to start / 免费且易于上手 |

**Lite is perfect for**: Solo developers who want secure SSH config management without the complexity of embedded terminals.
**Lite 适合**: 希望安全管理 SSH 配置，但不需要嵌入式终端复杂性的独立开发者。

---

### Choose Standard if / 选择 Standard 版本，如果您：

| Scenario / 场景 | Reason / 原因 |
|------------------|---------------|
| Managing 10+ servers / 管理 10+ 台服务器 | Nested grouping + search / 嵌套分组 + 搜索 |
| Need embedded terminal / 需要嵌入式终端 | No app switching / 无需切换应用 |
| Multi-tasking / 多任务处理 | Split screen + multi-tab / 分屏 + 多标签页 |
| Import existing config / 导入现有配置 | Full `~/.ssh/config` support / 完整 `~/.ssh/config` 支持 |
| Batch operations / 批量操作 | Run commands on multiple servers / 在多台服务器执行命令 |
| Performance-critical / 性能敏感 | WebGL acceleration / WebGL 加速 |

**Standard is perfect for**: DevOps engineers, system administrators, and power users managing multiple servers.
**Standard 适合**: 管理多台服务器的运维工程师、系统管理员和高级用户。

---

### Choose Pro if / 选择 Pro 版本，如果您：

| Scenario / 场景 | Reason / 原因 |
|------------------|---------------|
| IT team (3+ members) / IT 团队 (3+ 人) | Team management + RBAC / 团队管理 + RBAC |
| Enterprise compliance / 企业合规 | Audit logs + SSO / 审计日志 + SSO |
| Onboarding new members / 新成员入职 | Shared servers + snippets / 共享服务器 + 代码片段 |
| Security requirements / 安全要求 | E2EE + audit trail / 端到端加密 + 审计追踪 |
| API automation / API 自动化 | Programmatic access / 编程访问 |
| Need priority support / 需要优先支持 | Dedicated support channel / 专属支持渠道 |

**Pro is perfect for**: IT teams, enterprises, and organizations requiring collaboration and compliance features.
**Pro 适合**: 需要协作和合规功能的 IT 团队、企业和组织。

---

## Quick Decision Flow / 快速决策流程

```
                    Need team features?
                    需要团队功能?
                           │
              ┌────────────┴────────────┐
              │                         │
             Yes                        No
              │                         │
              ▼                         ▼
           ┌──────┐         Need embedded terminal?
           │ PRO  │         需要嵌入式终端?
           └──────┘                │
                      ┌────────────┴────────────┐
                      │                         │
                     Yes                        No
                      │                         │
                      ▼                         ▼
                 ┌──────────┐            ┌──────────┐
                 │ Standard │            │   Lite   │
                 └──────────┘            └──────────┘
```

---

## Feature Roadmap / 功能路线图

| Feature / 功能 | Status / 状态 | Target / 目标版本 |
|----------------|---------------|-------------------|
| Core SSH / 核心 SSH | Planned / 已规划 | Lite |
| Native Terminal / 原生终端 | Planned / 已规划 | Lite |
| Keychain Integration / 钥匙串集成 | Planned / 已规划 | Lite |
| Master Password / 主密码 | Planned / 已规划 | Lite |
| Embedded Terminal / 嵌入式终端 | Planned / 已规划 | Standard |
| Split Screen / 分屏 | Planned / 已规划 | Standard |
| Batch Operations / 批量操作 | Planned / 已规划 | Standard |
| WebGL Acceleration / WebGL 加速 | Planned / 已规划 | Standard |
| Team Management / 团队管理 | Planned / 已规划 | Pro |
| SSO Integration / SSO 集成 | Planned / 已规划 | Pro |
| Audit Logs / 审计日志 | Planned / 已规划 | Pro |

---

## Legend / 图例

| Symbol / 符号 | Meaning / 含义 |
|---------------|----------------|
| ✓ | Fully supported / 完全支持 |
| - | Not available / 不可用 |
| (Partial) | Partial support / 部分支持 |

---

*Last updated: 2026-04-03*
*最后更新: 2026-04-03*