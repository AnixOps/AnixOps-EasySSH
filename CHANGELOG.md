# Changelog / 变更日志

> All notable changes to this project will be documented in this file.
> This project adheres to [Semantic Versioning](https://semver.org/).

**[English](#english-changelog) | [中文](#中文变更日志)**

---

# English Changelog

## [Unreleased]

### Added
- Working Add Server dialog for Windows native UI
- Working Connect dialog with SSH connection support
- Clean Windows dependencies and build configuration
- Comprehensive bilingual documentation (EN/CN)

### Changed
- Improved CI/CD workflow for cross-platform builds
- Enhanced Windows native UI with egui

### Fixed
- Windows platform dependency issues
- Build configuration for release profile

---

## [0.3.0] - 2026-04-01

### Highlights / 亮点

This is a major release introducing three product tiers and a complete native desktop architecture.

### Added

#### Core Features
- **Three-Tier Product Architecture**: Lite, Standard, and Pro editions
- **Native Desktop Implementations**: Pure native UIs for all platforms
  - Windows: egui (Rust native)
  - Linux: GTK4 (pure native)
  - macOS: SwiftUI (pure native)
- **AI Auto-Programming Interface**: Debug mode only AI development capabilities
- **Enterprise Vault**: Master password + E2EE encryption for Lite
- **Connection Pooling**: Efficient SSH session management

#### Lite Edition
- Basic SSH connection management
- Server grouping (single-level)
- Native terminal launch integration
- Master password protection
- Keychain integration

#### Standard Edition
- Embedded WebGL terminal (xterm.js)
- Multi-tab and split-pane support (golden-layout)
- SFTP file transfer
- Server monitoring dashboard
- Docker container management
- Configuration import/export

#### Pro Edition
- Team management and member invitations
- RBAC permission control system
- Audit logging for compliance
- SSO integration (SAML/OIDC/LDAP)
- Real-time collaboration sessions
- Shared snippets and configurations

#### Additional Features
- Multi-language i18n support (EN/CN/JP)
- Telemetry analytics system
- Auto-update functionality
- Configuration backup system
- Log monitoring center
- Remote desktop (RDP/VNC) management
- Workflow automation system

### Security
- Argon2id + AES-256-GCM encryption scheme
- Complete audit logging system
- Enhanced key derivation parameters
- Fixed timing attack vulnerabilities in crypto implementation
- Complete input validation
- Fixed potential path traversal vulnerabilities

### Changed
- Reorganized to monorepo structure with workspace crates
- Migrated from Tauri-only to multi-platform native architecture
- Optimized SSH connection pool performance
- Improved error handling with internationalization support
- Updated all dependencies to latest versions
- Refactored FFI bridge layer

### Deprecated
- Legacy Tauri-only architecture

### Removed
- Experimental WebAssembly support

### Fixed
- Windows platform compilation issues
- GTK4 and libadwaita version compatibility
- Database concurrent access conflicts
- SFTP large file transfer issues
- macOS Keychain access issues

---

# 中文变更日志

## [未发布]

### 新增
- Windows原生UI的添加服务器对话框
- SSH连接支持的连接对话框
- 清理Windows依赖和构建配置
- 完整的中英文双语文档

### 变更
- 改进跨平台构建的CI/CD工作流
- 使用egui增强Windows原生UI

### 修复
- Windows平台依赖问题
- Release配置的构建设置

---

## [0.3.0] - 2026-04-01

### 亮点

这是一个主要版本，引入了三层产品架构和完整的原生桌面架构。

### 新增

#### 核心功能
- **三层产品架构**: Lite、Standard 和 Pro 版本
- **原生桌面实现**: 所有平台的纯原生UI
  - Windows: egui (Rust原生)
  - Linux: GTK4 (纯原生)
  - macOS: SwiftUI (纯原生)
- **AI自动编程接口**: 仅Debug模式的AI开发能力
- **企业级保险箱**: Lite版本的主密码 + E2EE加密
- **连接池**: 高效的SSH会话管理

#### Lite版本
- 基础SSH连接管理
- 服务器分组（单层）
- 原生终端唤起集成
- 主密码保护
- Keychain集成

#### Standard版本
- 嵌入式WebGL终端 (xterm.js)
- 多标签和分屏支持 (golden-layout)
- SFTP文件传输
- 服务器监控面板
- Docker容器管理
- 配置导入/导出

#### Pro版本
- 团队管理和成员邀请
- RBAC权限控制系统
- 合规审计日志
- SSO集成 (SAML/OIDC/LDAP)
- 实时协作会话
- 共享代码片段和配置

#### 附加功能
- 多语言i18n支持 (EN/CN/JP)
- 遥测分析系统
- 自动更新功能
- 配置备份系统
- 日志监控中心
- 远程桌面 (RDP/VNC) 管理
- 工作流自动化系统

### 安全
- Argon2id + AES-256-GCM加密方案
- 完整的审计日志系统
- 增强的密钥派生参数
- 修复加密实现中的时序攻击风险
- 完整的输入验证
- 修复潜在的路径遍历漏洞

### 变更
- 重组为monorepo结构和工作区crates
- 从仅Tauri迁移到多平台原生架构
- 优化SSH连接池性能
- 改进带国际化支持的错误处理
- 更新所有依赖到最新版本
- 重构FFI桥接层

### 废弃
- 旧的仅Tauri架构

### 移除
- 实验性WebAssembly支持

### 修复
- Windows平台编译问题
- GTK4和libadwaita版本兼容性
- 数据库并发访问冲突
- SFTP大文件传输问题
- macOS Keychain访问问题

---

## [0.2.0] - 2024-03-15

### Added / 新增
- Windows native UI (WinUI3) support / Windows原生UI (WinUI3) 支持
- GTK4 Linux native application / GTK4 Linux原生应用
- TUI (Terminal UI) version / TUI (终端UI) 版本
- Basic SSH connection management / 实现基础SSH连接管理
- Server configuration CRUD / 添加服务器配置CRUD
- Configuration import/export / 实现配置导入/导出

### Changed / 变更
- Core library split to `easyssh-core` / 将核心库独立为 `easyssh-core`
- Project structure refactoring / 重构项目结构

### Fixed / 修复
- macOS Keychain access issues / 修复macOS Keychain访问问题
- SSH agent forwarding / 修复SSH agent转发

---

## [0.1.0] - 2024-02-01

### Added / 新增
- Initial release / 初始版本发布
- Basic SSH client functionality / 基础SSH客户端功能
- Server list management / 服务器列表管理
- Password/key authentication / 密码/密钥认证
- SQLite data storage / SQLite数据存储

---

## Migration Guide / 升级指南

### Upgrading from 0.2.x to 0.3.x / 从 0.2.x 升级到 0.3.x

#### Breaking Changes / 破坏性变更

1. **Database Schema Changes / 数据库架构变更**
   - Sessions table now includes `created_at` field / 会话表现在包含 `created_at` 字段
   - Run migration script: / 需要运行迁移脚本:
   ```bash
   cargo run --bin migrate -- up
   ```

2. **API Changes / API 变更**
   - `SshManager::connect()` parameter order changed / 参数顺序变更
   - `CryptoState::new()` now requires explicit initialization / 现在需要显式初始化

3. **Configuration Format / 配置文件格式**
   - `config.yml` structure updated / 结构更新

#### Migration Steps / 迁移步骤

1. Backup existing database: / 备份现有数据库:
   ```bash
   cp ~/.easyssh/easyssh.db ~/.easyssh/easyssh.db.backup
   ```

2. Install new version: / 安装新版本:
   ```bash
   cargo install easyssh-core --version 0.3.0
   ```

3. Run database migration: / 运行数据库迁移:
   ```bash
   easyssh migrate
   ```

4. Verify installation: / 验证安装:
   ```bash
   easyssh --version
   ```

---

## Contributors / 贡献者

### 0.3.0 / 0.3.0版本
- @anixteam - Core architecture design / 核心架构设计
- @rustdev - SSH module optimization / SSH模块优化
- @security-expert - Security audit / 安全审计
- @ui-designer - GTK4/WinUI implementation / GTK4/WinUI实现

### 0.2.0 / 0.2.0版本
- @anixteam - Multi-platform architecture / 多平台架构
- @linux-guru - GTK4 implementation / GTK4实现
- @windows-dev - WinUI3 adaptation / WinUI3适配

### 0.1.0 / 0.1.0版本
- @anixteam - Initial version / 初始版本

---

## References / 参考

- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
- [Conventional Commits](https://www.conventionalcommits.org/)
