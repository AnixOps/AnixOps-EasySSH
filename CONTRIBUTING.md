# Contributing to EasySSH / 贡献指南

> Thank you for your interest in contributing to EasySSH! This guide will help you get started.
> 感谢您对 EasySSH 项目的关注！本指南将帮助您参与到项目开发中。

**[English](#english) | [中文](#中文)**

---

# English

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [How to Contribute](#how-to-contribute)
3. [Development Environment](#development-environment)
4. [Coding Standards](#coding-standards)
5. [Commit Conventions](#commit-conventions)
6. [Pull Request Process](#pull-request-process)
7. [Testing Requirements](#testing-requirements)
8. [Documentation](#documentation)
9. [Release Process](#release-process)
10. [Getting Help](#getting-help)

---

## Code of Conduct

### Our Standards

We are committed to providing a friendly, safe, and welcoming environment for all:

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

### Unacceptable Behavior

- Trolling, insulting/derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other conduct which could reasonably be considered inappropriate

---

## How to Contribute

### Reporting Bugs

Before submitting a bug report, please:

1. Search existing issues to ensure the bug hasn't been reported
2. Try the latest version to confirm the issue still exists
3. Gather relevant information: error logs, environment, reproduction steps

**Bug Report Template:**

```markdown
**Description**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. Scroll down to '....'
4. See error

**Expected behavior**
A clear description of what you expected to happen.

**Environment:**
- OS: [e.g., Windows 11, macOS 14, Ubuntu 22.04]
- Rust Version: [e.g., 1.78.0]
- EasySSH Version: [e.g., 0.3.0]
- Features: [e.g., standard, sftp]

**Screenshots**
If applicable, add screenshots to help explain your problem.

**Additional context**
Add any other context about the problem here.
```

### Suggesting Features

Feature requests should include:

1. Clear problem description
2. Proposed solution
3. Alternatives considered
4. Use cases and benefits

### Submitting Code

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Create a Pull Request

---

## Development Environment

### Quick Setup

```bash
# 1. Fork and clone the repository
git clone https://github.com/YOUR_USERNAME/easyssh.git
cd easyssh

# 2. Add upstream remote
git remote add upstream https://github.com/anixops/easyssh.git

# 3. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 4. Install dependencies (platform-specific)
# Linux:
sudo apt-get install libgtk-4-dev libadwaita-1-dev libssl-dev pkg-config

# macOS:
brew install gtk4 libadwaita openssl pkg-config

# Windows:
vcpkg install openssl:x64-windows sqlite3:x64-windows

# 5. Build the project
cargo build --features standard

# 6. Run tests
cargo test --features standard
```

### Project Structure

```
easyssh/
├── crates/
│   ├── easyssh-core/           # Core library
│   ├── easyssh-platforms/      # Platform-specific implementations
│   │   ├── windows/easyssh-egui/   # Windows egui version
│   │   ├── linux/easyssh-gtk4/     # Linux GTK4 version
│   │   └── macos/easyssh-swiftui/  # macOS SwiftUI version
│   ├── easyssh-api-tester/     # API tester (React)
│   └── easyssh-pro-server/     # Pro backend service
├── docs/                       # Documentation
├── tests/                      # Test suites
└── examples/                   # Example code
```

---

## Coding Standards

### Rust Naming Conventions

```rust
// Structs: PascalCase
pub struct ServerConfig { }

// Enums: PascalCase
pub enum ConnectionState { }

// Traits: PascalCase
pub trait SessionManager { }

// Functions: snake_case
fn connect_to_server() { }

// Variables: snake_case
let server_address = "192.168.1.1";

// Constants: SCREAMING_SNAKE_CASE
const MAX_CONNECTIONS: usize = 100;

// Static variables: SCREAMING_SNAKE_CASE
static DATABASE_URL: &str = "sqlite://...";
```

### Documentation Comments

```rust
/// Establish SSH connection to remote server.
///
/// # Arguments
///
/// * `host` - Remote host address
/// * `port` - SSH port, typically 22
/// * `username` - Login username
/// * `auth` - Authentication method
///
/// # Examples
///
/// ```rust,no_run
/// use easyssh_core::ssh::SshManager;
///
/// let manager = SshManager::new();
/// let session = manager.connect("192.168.1.1", 22, "root", &auth).await?;
/// ```
///
/// # Errors
///
/// Returns `LiteError::ConnectionFailed` when connection fails
pub async fn connect(
    &self,
    host: &str,
    port: u16,
    username: &str,
    auth: &AuthMethod,
) -> Result<Session, LiteError> {
    // ...
}
```

### Error Handling

```rust
// Use Result and thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LiteError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed")]
    AuthFailed,
}

// Use ? operator
pub async fn connect() -> Result<Session, LiteError> {
    let stream = TcpStream::connect(addr).await
        .map_err(|e| LiteError::ConnectionFailed(e.to_string()))?;
    // ...
}
```

### Async Code

```rust
// Use tokio
use tokio::time::{sleep, Duration};

pub async fn retry_with_backoff<F, Fut, T>(
    f: F,
    retries: u32,
) -> Result<T, LiteError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, LiteError>>,
{
    for i in 0..retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if i < retries - 1 => {
                sleep(Duration::from_millis(100 * 2_u64.pow(i))).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

### Security Coding

```rust
// 1. Don't log sensitive information
// Wrong
log::info!("Password: {}", password);

// Correct
log::info!("Authenticating user: {}", username);

// 2. Zeroize sensitive data
use zeroize::Zeroize;

let mut password = String::from("secret");
// After use
password.zeroize();

// 3. Validate all inputs
pub fn set_port(&mut self, port: u16) -> Result<(), LiteError> {
    if port == 0 || port > 65535 {
        return Err(LiteError::InvalidPort(port));
    }
    self.port = port;
    Ok(())
}
```

---

## Commit Conventions

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only changes |
| `style` | Code style changes (formatting, no functional changes) |
| `refactor` | Code refactoring |
| `perf` | Performance improvements |
| `test` | Adding or correcting tests |
| `chore` | Build process or auxiliary tool changes |
| `ci` | CI/CD changes |
| `security` | Security fixes |

### Scopes

- `core` - Core library
- `ssh` - SSH module
- `crypto` - Crypto module
- `db` - Database module
- `ui` - User interface
- `api` - API interface
- `docs` - Documentation
- `ci` - CI/CD

### Subject Rules

- Use imperative mood ("change" not "changed" or "changes")
- Don't capitalize first letter
- No period at the end

### Examples

```
feat(ssh): add connection pooling support

Implement connection pooling for SSH sessions to improve
performance when connecting to the same server multiple times.

- Add ConnectionPool struct
- Implement pooling in SshManager
- Add configuration options for pool size and timeout

Refs #123
```

```
fix(crypto): resolve memory leak in encryption

The encryption key was not being properly zeroized after use,
leading to potential memory exposure.

Fixes #456
```

---

## Pull Request Process

### Pre-PR Checklist

- [ ] Code compiles and passes all tests
- [ ] Added/updated relevant tests
- [ ] Updated documentation (if needed)
- [ ] Follows coding standards
- [ ] Commit messages follow conventions
- [ ] No unrelated changes included
- [ ] Updated CHANGELOG.md (if needed)

### PR Template

```markdown
## Description
Brief description of this PR's purpose.

## Type
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring

## Testing
- [ ] Local tests pass
- [ ] Added unit tests
- [ ] Added integration tests

## Checklist
- [ ] Code follows project standards
- [ ] Documentation updated
- [ ] All tests pass
- [ ] CHANGELOG.md updated

## Related Issues
Fixes #(issue number)
Refs #(issue number)

## Screenshots (if applicable)
```

### Review Process

1. **Automated Checks**
   - CI build must pass
   - Test coverage must not decrease
   - Clippy warnings must be fixed
   - Code format checks pass

2. **Manual Review**
   - At least 1 maintainer approval required
   - All review comments must be resolved
   - New features require documentation review

3. **Merge**
   - Use "Squash and Merge"
   - Ensure commit message follows convention
   - Delete feature branch

---

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_pool_creation() {
        let pool = ConnectionPool::new(10);
        assert_eq!(pool.capacity(), 10);
        assert_eq!(pool.size(), 0);
    }

    #[tokio::test]
    async fn test_async_connection() {
        let manager = SshManager::new();
        let result = manager.connect("localhost", 22, "test", &auth).await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

```rust
// tests/ssh_integration.rs
use easyssh_core::{AppState, init_database, ssh_connect};

#[tokio::test]
async fn test_ssh_connection_lifecycle() {
    let state = AppState::new();
    init_database(&state).unwrap();

    // Add test server
    // ...

    // Connect
    let session = ssh_connect(&state, "test-server", None).await;
    assert!(session.is_ok());

    // Disconnect
    let result = ssh_disconnect(&state, &session.unwrap().id).await;
    assert!(result.is_ok());
}
```

### Coverage Requirements

- Core business logic: > 90%
- Error handling paths: > 80%
- UI code: > 70%

```bash
# Generate coverage report
cargo install cargo-tarpaulin
cargo tarpaulin --all-features --out html

# View report
open tarpaulin-report.html
```

---

## Documentation

### Code Documentation

- All public APIs must have doc comments
- Complex logic needs inline comments
- Example code should be compilable

### User Documentation

New features require updates to:

- `API_GUIDE.md` - API usage guide
- `DEPLOYMENT.md` - If deployment-affecting
- `CHANGELOG.md` - Changelog

### Architecture Documentation

Major architecture changes require updates to:

- Relevant docs in `docs/architecture/`
- Add Architecture Decision Record (ADR)

---

## Release Process

### Versioning

Following [SemVer](https://semver.org/):

- `MAJOR` - Breaking changes
- `MINOR` - New features (backward compatible)
- `PATCH` - Bug fixes

### Release Steps

1. **Preparation**
   ```bash
   # Update version
   cargo set-version 0.4.0

   # Update CHANGELOG
   # Add all changes
   ```

2. **Testing**
   ```bash
   # Full test suite
   cargo test --all-features

   # Build check
   cargo build --release --all-features
   ```

3. **Release**
   ```bash
   # Create tag
   git tag -a v0.4.0 -m "Release version 0.4.0"
   git push origin v0.4.0

   # Publish to crates.io
   cargo publish --package easyssh-core
   ```

4. **Post-Release**
   - Create GitHub Release
   - Publish changelog
   - Update documentation site

---

## Getting Help

### Resources

- [API Docs](https://docs.rs/easyssh-core)
- [Architecture Docs](docs/architecture/)
- [FAQ](docs/faq.md)

### Contact

- **General Questions**: GitHub Discussions
- **Bug Reports**: GitHub Issues
- **Security Issues**: security@easyssh.dev
- **Mailing List**: dev@easyssh.dev

### Community

- [Discord](https://discord.gg/easyssh)
- [Twitter](https://twitter.com/easyssh)

---

**Thank you for contributing!**

---

# 中文

## 目录

1. [行为准则](#行为准则-1)
2. [如何贡献](#如何贡献-1)
3. [开发环境](#开发环境-1)
4. [代码规范](#代码规范)
5. [提交规范](#提交规范)
6. [Pull Request 流程](#pull-request-流程)
7. [测试要求](#测试要求)
8. [文档要求](#文档要求)
9. [发布流程](#发布流程-1)
10. [获取帮助](#获取帮助)

---

## 行为准则

### 我们的承诺

为了营造一个开放和友好的环境，我们作为贡献者和维护者承诺：

- 尊重所有参与者，无论经验水平、性别、性别认同和表达、性取向、残疾、个人外貌、体型、种族、民族、年龄、宗教或国籍
- 接受建设性的批评，以优雅的态度接受
- 关注对社区最有利的事情
- 对其他社区成员表示同理心

### 不可接受的行为

- 使用性别歧视、种族歧视或排他性语言
- 骚扰、侮辱/贬损性评论、个人或政治攻击
- 公开或私下骚扰
- 未经明确许可发布他人的私人信息
- 其他可以被合理认为不适当或违反职业操守的行为

---

## 如何贡献

### 报告 Bug

在报告 Bug 之前，请先：

1. 搜索现有 Issues，确认问题未被报告
2. 尝试最新版本，确认问题仍然存在
3. 收集相关信息：错误日志、系统环境、复现步骤

**Bug 报告模板:**

```markdown
**描述 Bug**
清晰简洁地描述 Bug 是什么。

**复现步骤**
1. 进入 '...'
2. 点击 '...'
3. 滚动到 '...'
4. 看到错误

**预期行为**
描述您期望发生的事情。

**环境信息:**
- OS: [例如: Windows 11, macOS 14, Ubuntu 22.04]
- Rust 版本: [例如: 1.78.0]
- EasySSH 版本: [例如: 0.3.0]
- 功能特性: [例如: standard, sftp]

**截图**
如果适用，添加截图帮助说明问题。

**附加信息**
添加关于问题的任何其他上下文。
```

### 建议新功能

功能请求应包含：

1. 清晰的问题描述
2. 提议的解决方案
3. 替代方案（如果有）
4. 使用场景和好处

### 提交代码

1. Fork 仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

---

## 开发环境

### 快速设置

```bash
# 1. Fork 并克隆仓库
git clone https://github.com/YOUR_USERNAME/easyssh.git
cd easyssh

# 2. 添加上游仓库
git remote add upstream https://github.com/anixops/easyssh.git

# 3. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 4. 安装依赖（根据平台选择）
# Linux:
sudo apt-get install libgtk-4-dev libadwaita-1-dev libssl-dev pkg-config

# macOS:
brew install gtk4 libadwaita openssl pkg-config

# Windows:
vcpkg install openssl:x64-windows sqlite3:x64-windows

# 5. 构建项目
cargo build --features standard

# 6. 运行测试
cargo test --features standard
```

### 项目结构

```
easyssh/
├── crates/
│   ├── easyssh-core/           # 核心库
│   ├── easyssh-platforms/      # 平台特定实现
│   │   ├── windows/easyssh-egui/   # Windows egui版本
│   │   ├── linux/easyssh-gtk4/     # Linux GTK4版本
│   │   └── macos/easyssh-swiftui/  # macOS SwiftUI版本
│   ├── easyssh-api-tester/     # API测试器 (React)
│   └── easyssh-pro-server/     # Pro后端服务
├── docs/                       # 文档
├── tests/                      # 测试套件
└── examples/                   # 示例代码
```

---

## 代码规范

### Rust 命名规范

```rust
// 结构体：PascalCase
pub struct ServerConfig { }

// 枚举：PascalCase
pub enum ConnectionState { }

// 特征：PascalCase
pub trait SessionManager { }

// 函数：snake_case
fn connect_to_server() { }

// 变量：snake_case
let server_address = "192.168.1.1";

// 常量：SCREAMING_SNAKE_CASE
const MAX_CONNECTIONS: usize = 100;

// 静态变量：SCREAMING_SNAKE_CASE
static DATABASE_URL: &str = "sqlite://...";
```

### 文档注释

```rust
/// 建立 SSH 连接到远程服务器。
///
/// # 参数
///
/// * `host` - 远程主机地址
/// * `port` - SSH 端口，通常为 22
/// * `username` - 登录用户名
/// * `auth` - 认证方式
///
/// # 示例
///
/// ```rust,no_run
/// use easyssh_core::ssh::SshManager;
///
/// let manager = SshManager::new();
/// let session = manager.connect("192.168.1.1", 22, "root", &auth).await?;
/// ```
///
/// # 错误
///
/// 当连接失败时返回 `LiteError::ConnectionFailed`
pub async fn connect(
    &self,
    host: &str,
    port: u16,
    username: &str,
    auth: &AuthMethod,
) -> Result<Session, LiteError> {
    // ...
}
```

### 错误处理

```rust
// 使用 Result 和 thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LiteError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),
    #[error("认证失败")]
    AuthFailed,
}

// 使用 ? 操作符
pub async fn connect() -> Result<Session, LiteError> {
    let stream = TcpStream::connect(addr).await
        .map_err(|e| LiteError::ConnectionFailed(e.to_string()))?;
    // ...
}
```

### 异步代码

```rust
// 使用 tokio
use tokio::time::{sleep, Duration};

pub async fn retry_with_backoff<F, Fut, T>(
    f: F,
    retries: u32,
) -> Result<T, LiteError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, LiteError>>,
{
    for i in 0..retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if i < retries - 1 => {
                sleep(Duration::from_millis(100 * 2_u64.pow(i))).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

### 安全编码

```rust
// 1. 不要记录敏感信息
// 错误
log::info!("Password: {}", password);

// 正确
log::info!("Authenticating user: {}", username);

// 2. 使用零化清除敏感数据
use zeroize::Zeroize;

let mut password = String::from("secret");
// 使用完成后
password.zeroize();

// 3. 验证所有输入
pub fn set_port(&mut self, port: u16) -> Result<(), LiteError> {
    if port == 0 || port > 65535 {
        return Err(LiteError::InvalidPort(port));
    }
    self.port = port;
    Ok(())
}
```

---

## 提交规范

### 格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type

| 类型 | 描述 |
|------|------|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `docs` | 文档更新 |
| `style` | 代码格式（不影响功能的变动） |
| `refactor` | 重构 |
| `perf` | 性能优化 |
| `test` | 添加测试 |
| `chore` | 构建过程或辅助工具的变动 |
| `ci` | CI/CD 配置 |
| `security` | 安全修复 |

### Scope

可选，用于说明提交影响的范围：

- `core` - 核心库
- `ssh` - SSH 模块
- `crypto` - 加密模块
- `db` - 数据库模块
- `ui` - 用户界面
- `api` - API 接口
- `docs` - 文档
- `ci` - CI/CD

### Subject

- 使用祈使语气，现在时
- 首字母不要大写
- 末尾不加句号

### Body

- 使用祈使语气
- 说明变动的动机和与之前行为的对比

### Footer

- `BREAKING CHANGE:` - 破坏性变更说明
- `Closes #123` - 关闭 Issue
- `Refs #456` - 引用 Issue

### 提交示例

```
feat(ssh): add connection pooling support

Implement connection pooling for SSH sessions to improve
performance when connecting to the same server multiple times.

- Add ConnectionPool struct
- Implement pooling in SshManager
- Add configuration options for pool size and timeout

Refs #123
```

```
fix(crypto): resolve memory leak in encryption

The encryption key was not being properly zeroized after use,
leading to potential memory exposure.

Fixes #456
```

---

## Pull Request 流程

### 创建 PR 前检查清单

- [ ] 代码可以编译并通过所有测试
- [ ] 添加/更新了相关测试
- [ ] 更新了文档（如果需要）
- [ ] 遵循了代码规范
- [ ] 提交了信息遵循规范
- [ ] 没有包含无关的更改
- [ ] 更新了 CHANGELOG.md（如果需要）

### PR 模板

```markdown
## 描述
简要描述这个 PR 的目的。

## 类型
- [ ] Bug 修复
- [ ] 新功能
- [ ] 破坏性变更
- [ ] 文档更新
- [ ] 性能优化
- [ ] 代码重构

## 测试
- [ ] 本地测试通过
- [ ] 添加了单元测试
- [ ] 添加了集成测试

## 检查清单
- [ ] 代码遵循项目规范
- [ ] 文档已更新
- [ ] 所有测试通过
- [ ] CHANGELOG.md 已更新

## 相关 Issues
Fixes #(issue number)
Refs #(issue number)

## 截图（如果适用）
```

### 审查流程

1. **自动检查**
   - CI 构建必须通过
   - 测试覆盖率不能下降
   - Clippy 警告必须修复
   - 代码格式检查通过

2. **人工审查**
   - 至少需要 1 个维护者批准
   - 所有审查意见必须解决
   - 新功能需要文档审查

3. **合并**
   - 使用 "Squash and Merge"
   - 确保提交信息符合规范
   - 删除特性分支

---

## 测试要求

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_pool_creation() {
        let pool = ConnectionPool::new(10);
        assert_eq!(pool.capacity(), 10);
        assert_eq!(pool.size(), 0);
    }

    #[tokio::test]
    async fn test_async_connection() {
        let manager = SshManager::new();
        let result = manager.connect("localhost", 22, "test", &auth).await;
        assert!(result.is_ok());
    }
}
```

### 集成测试

```rust
// tests/ssh_integration.rs
use easyssh_core::{AppState, init_database, ssh_connect};

#[tokio::test]
async fn test_ssh_connection_lifecycle() {
    let state = AppState::new();
    init_database(&state).unwrap();

    // 添加测试服务器
    // ...

    // 连接
    let session = ssh_connect(&state, "test-server", None).await;
    assert!(session.is_ok());

    // 断开
    let result = ssh_disconnect(&state, &session.unwrap().id).await;
    assert!(result.is_ok());
}
```

### 测试覆盖率

- 核心业务逻辑：> 90%
- 错误处理路径：> 80%
- UI 代码：> 70%

```bash
# 生成覆盖率报告
cargo install cargo-tarpaulin
cargo tarpaulin --all-features --out html

# 查看报告
open tarpaulin-report.html
```

---

## 文档要求

### 代码文档

- 所有公共 API 必须有文档注释
- 复杂逻辑需要行内注释
- 示例代码应该可以编译运行

### 用户文档

新功能需要更新：

- `API_GUIDE.md` - API 使用指南
- `DEPLOYMENT.md` - 如果影响部署
- `CHANGELOG.md` - 变更日志

### 架构文档

重大架构变更需要更新：

- `docs/architecture/` 下的相关文档
- 添加架构决策记录 (ADR)

---

## 发布流程

### 版本号规则

遵循 [SemVer](https://semver.org/lang/zh-CN/)：

- `MAJOR` - 破坏性变更
- `MINOR` - 新功能（向后兼容）
- `PATCH` - Bug 修复

### 发布步骤

1. **准备阶段**
   ```bash
   # 更新版本号
   cargo set-version 0.4.0

   # 更新 CHANGELOG
   # 添加所有变更
   ```

2. **测试阶段**
   ```bash
   # 完整测试
   cargo test --all-features

   # 构建检查
   cargo build --release --all-features
   ```

3. **发布阶段**
   ```bash
   # 创建标签
   git tag -a v0.4.0 -m "Release version 0.4.0"
   git push origin v0.4.0

   # 发布到 crates.io
   cargo publish --package easyssh-core
   ```

4. **发布后**
   - 创建 GitHub Release
   - 发布更新日志
   - 更新文档网站

---

## 获取帮助

### 资源

- [API 文档](https://docs.rs/easyssh-core)
- [架构文档](docs/architecture/)
- [FAQ](docs/faq.md)

### 联系方式

- **一般问题**: GitHub Discussions
- **Bug 报告**: GitHub Issues
- **安全问题**: security@easyssh.dev
- **邮件列表**: dev@easyssh.dev

### 社区

- [Discord](https://discord.gg/easyssh)
- [Twitter](https://twitter.com/easyssh)

---

**再次感谢您的贡献！**
