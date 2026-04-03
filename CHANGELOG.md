# Changelog / 变更日志

> All notable changes to this project will be documented in this file.
> This project adheres to [Semantic Versioning](https://semver.org/).

**[English](#english-changelog) | [中文](#中文变更日志)**

---

# English Changelog

## [Unreleased]

## [0.3.0-beta.2] - Unreleased

### Added

- TerminalCoordinator for unified session management
- Tab bar UI component for multi-terminal support
- SSH config import dialog with preview
- GitHub Issue templates (bug report, feature request, feedback)
- Azure deployment documentation
- Startup performance optimizations (parallel init, cold start cache)
- Startup benchmarks

### Fixed

- workflow_demo.rs compilation error
- Platform example files moved to correct crates
- wry 0.46+ API compatibility
- telemetry_demo unused imports

### Changed

- WebView integration updated for latest wry API
- Database fast path with deferred indexing

## [0.3.0-beta.1] - 2026-04-03

### Highlights

This is the initial beta release of EasySSH, introducing a secure and native SSH client with three product editions.

- Initial beta release of EasySSH
- Three editions: Lite, Standard, and Pro
- 962 tests passing with 100% test pass rate
- Native UI on all platforms (egui on Windows, GTK4 on Linux, SwiftUI on macOS)
- Secure credential storage with Argon2id + AES-256-GCM encryption
- Keychain integration for secure password management
- Full SSH functionality with password and key-based authentication

### Added

- Complete test suite with 962 passing tests
- Native desktop UI implementations for all platforms
- Secure credential encryption using Argon2id for key derivation and AES-256-GCM for data encryption
- Keychain integration on macOS, Windows Credential Manager, and Linux secret service
- SSH connection management with support for password and SSH key authentication
- Server grouping and organization capabilities
- Configuration import from ~/.ssh/config
- Bilingual documentation (English/Chinese)

### Changed

- Improved build system for cross-platform compilation
- Enhanced error handling with user-friendly messages

### Fixed

- Test failures in validation, edition, and models modules
- Clippy warnings including unused variables and redundant closures
- Backup test issues with schema migrations and timestamp handling
- Compilation errors in easyssh-tui module

### Security

- Argon2id key derivation function for secure password hashing
- AES-256-GCM authenticated encryption for credential storage
- Secure keychain integration across all platforms
- Input validation and sanitization

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

## [0.3.0-beta.2] - 未发布

### 新增

- TerminalCoordinator 统一会话管理
- 多终端标签栏UI组件
- SSH配置导入对话框（带预览）
- GitHub Issue模板（错误报告、功能请求、反馈）
- Azure部署文档
- 启动性能优化（并行初始化、冷启动缓存）
- 启动性能基准测试

### 修复

- workflow_demo.rs 编译错误
- 平台示例文件移动到正确的crate
- wry 0.46+ API兼容性
- telemetry_demo 未使用导入

### 变更

- WebView集成更新到最新wry API
- 数据库快速路径与延迟索引

## [0.3.0-beta.1] - 2026-04-03

### 亮点

这是EasySSH的首个测试版本，推出了安全、原生的SSH客户端，包含三个产品版本。

- EasySSH首个测试版本发布
- 三个版本：Lite、Standard 和 Pro
- 962个测试通过，测试通过率100%
- 所有平台原生UI（Windows使用egui，Linux使用GTK4，macOS使用SwiftUI）
- 使用Argon2id + AES-256-GCM安全存储凭据
- Keychain集成用于安全密码管理
- 完整SSH功能，支持密码和密钥认证

### 新增

- 完整的测试套件，962个测试通过
- 所有平台的原生桌面UI实现
- 使用Argon2id密钥派生和AES-256-GCM数据加密的安全凭据加密
- macOS、Windows凭据管理器和Linux密钥服务的Keychain集成
- SSH连接管理，支持密码和SSH密钥认证
- 服务器分组和组织功能
- 从~/.ssh/config导入配置
- 中英双语文档

### 变更

- 改进跨平台编译的构建系统
- 增强错误处理，提供用户友好的错误信息

### 修复

- 验证、版本和模型模块中的测试失败
- Clippy警告，包括未使用的变量和冗余闭包
- 备份测试中的架构迁移和时间戳处理问题
- easyssh-tui模块中的编译错误

### 安全

- Argon2id密钥派生函数用于安全密码哈希
- AES-256-GCM认证加密用于凭据存储
- 所有平台的安全Keychain集成
- 输入验证和清理

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
