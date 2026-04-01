# Changelog

> 所有 notable 变更都将记录在此文件中。
>
> 格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)，
> 并且本项目遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### Added
- Working Add Server dialog for Windows native UI
- Working Connect dialog with SSH connection support
- Clean Windows dependencies and build configuration

### Changed
- Improved CI/CD workflow for cross-platform builds
- Enhanced Windows native UI with egui

### Fixed
- Windows platform dependency issues
- Build configuration for release profile

## [0.3.0] - 2024-04-01
- 新增工作流自动化系统
- 新增企业级密码保险箱 (Enterprise Vault)
- 新增完整的多语言国际化支持 (i18n)
- 新增遥测分析系统 (Telemetry)
- 新增自动更新功能 (Auto-update)
- 新增配置备份系统 (Backup)
- 新增日志监控中心 (Log Monitor)
- 新增远程桌面连接管理 (RDP/VNC)

### Changed
- 重构为多平台原生架构 (Tauri/GTK4/WinUI)
- 优化 SSH 连接池性能
- 改进错误处理国际化支持
- 升级依赖库到最新版本

### Fixed
- 修复 Windows 平台编译问题
- 修复数据库并发访问冲突
- 修复 SFTP 大文件传输问题

### Security
- 修复密码学实现中的时序攻击风险
- 加强密钥派生参数 (Argon2id)
- 实现完整的审计日志系统

## [0.3.0] - 2024-04-01

### Added
- **Pro 版本功能**
  - 团队协作和成员管理
  - RBAC 权限控制系统
  - 审计日志功能
  - SSO 集成 (SAML/OIDC/LDAP)
  - 实时协作会话
  - 共享 Snippets 和配置

- **Standard 版本功能**
  - 嵌入式终端模拟器
  - 终端分屏布局
  - SFTP 文件传输
  - 服务器监控面板
  - Docker 容器管理
  - 配置导入/导出

- **Lite 版本功能**
  - 基础 SSH 连接管理
  - 服务器分组管理
  - 原生终端唤起
  - 主密码保护
  - Keychain 集成

- **核心功能**
  - SSH 连接池和会话管理
  - AES-256-GCM 加密存储
  - Argon2id 密钥派生
  - SQLite 持久化
  - 多平台支持 (Linux/macOS/Windows)
  - AI 自动编程接口 (Debug 模式)

### Changed
- 采用 Monorepo 结构管理多版本
- 重构 FFI 桥接层
- 统一错误处理系统

### Deprecated
- 旧版 Tauri-only 架构

### Removed
- 实验性功能：WebAssembly 支持

### Fixed
- 解决 GTK4 和 libadwaita 版本兼容问题
- 修复 Windows 平台依赖问题

### Security
- 实现 Argon2id + AES-256-GCM 加密方案
- 添加完整的输入验证
- 修复潜在的路径遍历漏洞

## [0.2.0] - 2024-03-15

### Added
- 新增 Windows 原生 UI (WinUI3) 支持
- 新增 GTK4 Linux 原生应用
- 新增 TUI (Terminal UI) 版本
- 实现基本 SSH 连接管理
- 添加服务器配置 CRUD
- 实现配置导入/导出

### Changed
- 将核心库独立为 `easyssh-core`
- 重构项目结构

### Fixed
- 修复 macOS Keychain 访问问题
- 修复 SSH agent 转发

## [0.1.0] - 2024-02-01

### Added
- 初始版本发布
- 基础 SSH 客户端功能
- 服务器列表管理
- 密码/密钥认证
- SQLite 数据存储

---

## 版本升级指南

### 从 0.2.x 升级到 0.3.x

#### 破坏性变更

1. **数据库架构变更**
   - 会话表现在包含 `created_at` 字段
   - 需要运行迁移脚本：
   ```bash
   cargo run --bin migrate -- up
   ```

2. **API 变更**
   - `SshManager::connect()` 参数顺序变更
   - `CryptoState::new()` 现在需要显式初始化

3. **配置文件格式**
   - `config.yml` 结构更新，需要更新配置文件

#### 迁移步骤

1. 备份现有数据库：
   ```bash
   cp ~/.easyssh/easyssh.db ~/.easyssh/easyssh.db.backup
   ```

2. 安装新版本：
   ```bash
   cargo install easyssh-core --version 0.3.0
   ```

3. 运行数据库迁移：
   ```bash
   easyssh migrate
   ```

4. 验证安装：
   ```bash
   easyssh --version
   ```

---

## 贡献者

感谢所有为本项目做出贡献的开发者！

### 0.3.0 版本贡献者
- @anixteam - 核心架构设计
- @rustdev - SSH 模块优化
- @security-expert - 安全审计
- @ui-designer - GTK4/WinUI 实现

### 0.2.0 版本贡献者
- @anixteam - 多平台架构
- @linux-guru - GTK4 实现
- @windows-dev - WinUI3 适配

### 0.1.0 版本贡献者
- @anixteam - 初始版本

---

## 参考

- [SemVer 规范](https://semver.org/lang/zh-CN/)
- [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
- [Conventional Commits](https://www.conventionalcommits.org/)
