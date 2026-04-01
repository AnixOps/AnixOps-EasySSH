# 开发文档概览

欢迎来到 EasySSH 开发文档。本文档面向希望为 EasySSH 贡献代码、构建插件或进行二次开发的开发者。

## 开发环境

### 系统要求

| 组件 | 最低版本 | 说明 |
|------|----------|------|
| Rust | 1.75+ | 核心语言 |
| Node.js | 18+ | 前端构建 |
| pnpm | 8+ | 包管理 |
| SQLite | 3.39+ | 开发数据库 |
| libssh2 | 1.11+ | SSH 支持 |
| OpenSSL | 3.0+ | 加密支持 |

### 平台特定依赖

**macOS:**
```bash
brew install rust node pnpm sqlite3 libssh2 openssl pkg-config
```

**Windows:**
```powershell
# 使用 winget
winget install Rustlang.Rustup OpenJS.NodeJS SQLite

# 或使用 chocolatey
choco install rust nodejs sqlite
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install -y \
    rustc cargo \
    nodejs npm \
    libsqlite3-dev \
    libssh2-1-dev \
    libssl-dev \
    pkg-config \
    build-essential
```

### 验证环境

```bash
# 验证 Rust
rustc --version  # 应显示 1.75+
cargo --version

# 验证 Node
node --version   # 应显示 18+
npm --version

# 验证其他依赖
pkg-config --modversion libssh2  # 应显示 1.11+
pkg-config --modversion sqlite3  # 应显示 3.39+
```

## 项目结构

```
easyssh/
├── Cargo.toml              # 工作区配置
├── CLAUDE.md              # 项目总览
├── core/                  # 核心库 (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs         # 库入口
│       ├── ssh.rs         # SSH 模块
│       ├── db.rs          # 数据库模块
│       ├── crypto.rs      # 加密模块
│       ├── sftp.rs        # SFTP 模块
│       ├── terminal.rs    # 终端模块 (Standard+)
│       ├── layout.rs      # 布局模块 (Standard+)
│       ├── team.rs        # 团队模块 (Pro)
│       ├── rbac.rs        # 权限模块 (Pro)
│       ├── audit.rs       # 审计模块 (Pro)
│       └── ffi.rs         # FFI 接口
│
├── tui/                   # 命令行界面 (Rust)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
│
├── platforms/             # 原生平台实现
│   ├── linux/
│   │   └── easyssh-gtk4/  # GTK4 实现
│   ├── macos/
│   │   └── easyssh-swift/ # SwiftUI 实现
│   └── windows/
│       └── easyssh-winui/ # WinUI 3 实现
│
├── web/                   # Web 管理界面 (Pro)
│   ├── package.json
│   └── src/
│       └── ...
│
├── docs/                  # 设计文档
│   ├── architecture/
│   ├── standards/
│   └── *.md
│
└── docs-product/          # 产品文档 (本站点)
    └── ...
```

## 快速开始

### 1. 克隆仓库

```bash
git clone https://github.com/anixops/easyssh.git
cd easyssh
```

### 2. 安装依赖

```bash
# 安装前端依赖
cd web && pnpm install && cd ..

# 安装 Rust 依赖（自动）
cargo fetch
```

### 3. 构建项目

```bash
# 构建 Core 库
cargo build -p easyssh-core

# 构建 TUI
cargo build -p easyssh-tui

# 构建完整项目
cargo build --workspace
```

### 4. 运行测试

```bash
# 运行单元测试
cargo test --workspace

# 运行集成测试
cargo test --test integration

# 运行特定模块测试
cargo test -p easyssh-core ssh

# 生成覆盖率报告
cargo tarpaulin --out Html
```

### 5. 启动开发环境

```bash
# 启动 TUI（命令行界面）
cargo run -p easyssh-tui

# 构建并测试 Core
cargo test -p easyssh-core -- --nocapture
```

## 开发工作流

### 分支策略

```
main                    # 稳定分支，始终可发布
├── release/v1.x        # 发布分支
├── develop             # 开发分支
│   ├── feature/ssh-pool
│   ├── feature/team-module
│   └── bugfix/memory-leak
└── hotfix/             # 紧急修复
```

### 提交规范

使用 [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**类型：**
- `feat`: 新功能
- `fix`: 修复
- `docs`: 文档
- `style`: 格式（不影响代码）
- `refactor`: 重构
- `perf`: 性能优化
- `test`: 测试
- `chore`: 构建/工具

**示例：**
```
feat(ssh): add connection pooling

Implement connection pool for SSH sessions with configurable
max connections, idle timeout, and health checks.

Closes #123
```

### 代码审查

提交 PR 前请确保：

- [ ] 代码通过 `cargo check` 和 `cargo clippy`
- [ ] 所有测试通过 `cargo test`
- [ ] 格式化通过 `cargo fmt`
- [ ] 新增功能有测试覆盖
- [ ] 文档已更新
- [ ] CHANGELOG 已更新

## 调试技巧

### 日志输出

```bash
# 启用调试日志
RUST_LOG=debug cargo run -p easyssh-tui

# 仅 Core 模块调试
RUST_LOG=easyssh_core=debug cargo test

# 详细跟踪
RUST_LOG=trace cargo run
```

### 使用调试工具

```rust
// 在代码中添加断点
use std::io::{self, Write};

fn debug_point() {
    println!("Debug point hit. Press Enter to continue...");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}
```

### 内存分析

```bash
# 使用 valgrind (Linux)
valgrind --tool=memcheck --leak-check=full target/debug/easyssh-tui

# 使用 Instruments (macOS)
instruments -t "Leaks" target/debug/easyssh-tui
```

## 核心模块开发

### 添加新模块

1. **创建模块文件**
   ```bash
   touch core/src/my_module.rs
   ```

2. **在 lib.rs 中声明**
   ```rust
   pub mod my_module;
   ```

3. **添加条件编译（如需要）**
   ```rust
   #[cfg(feature = "my-feature")]
   pub mod my_module;
   ```

4. **更新 Cargo.toml**
   ```toml
   [features]
   my-feature = []
   ```

### 数据库迁移

```bash
# 创建迁移文件
cd core
cargo sqlx migrate add create_servers_table

# 运行迁移
cargo sqlx migrate run

# 回滚
cargo sqlx migrate revert
```

## FFI 开发

### 添加 FFI 函数

```rust
// core/src/ffi.rs
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

#[no_mangle]
pub extern "C" fn easyssh_my_function(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input).to_str().unwrap() };
    let output = format!("Processed: {}", input);
    CString::new(output).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn easyssh_free_string(s: *mut c_char) {
    if s.is_null() { return; }
    unsafe { CString::from_raw(s); };
}
```

### 生成头文件

```bash
# 使用 cbindgen
cbindgen --config cbindgen.toml --crate easyssh-core --output easyssh.h
```

## 性能优化

### 基准测试

```bash
# 运行基准测试
cargo bench

# 特定基准
cargo bench ssh_pool
```

### 性能分析

```bash
# CPU 分析
cargo flamegraph --bin easyssh-tui

# 生成火焰图
firefox flamegraph.svg
```

## 安全开发

### 安全检查

```bash
# 安全检查
cargo audit

# 检查不安全的代码
cargo geiger
```

### 模糊测试

```bash
# 安装 cargo-fuzz
cargo install cargo-fuzz

# 创建模糊测试
cargo fuzz init

# 运行模糊测试
cargo fuzz run my_target
```

## 文档生成

### Rust 文档

```bash
# 生成并打开文档
cargo doc --open

# 包含私有项
cargo doc --document-private-items
```

### API 文档

API 文档自动生成并部署到：
- https://api.easyssh.dev/rust/

## 发布流程

### 版本发布

```bash
# 1. 更新版本号
# 编辑 Cargo.toml 和 package.json

# 2. 更新 CHANGELOG
# 编辑 CHANGELOG.md

# 3. 提交版本更新
git add -A
git commit -m "chore(release): prepare v1.2.0"

# 4. 创建标签
git tag v1.2.0

# 5. 推送到远程
git push origin main --tags

# 6. CI 将自动构建和发布
```

## 贡献指南

### 如何贡献

1. Fork 仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 行为准则

- 尊重所有参与者
- 欢迎新手，耐心指导
- 建设性反馈
- 关注技术，不人身攻击

### 联系方式

- **Discussions**: [GitHub Discussions](https://github.com/anixops/easyssh/discussions)
- **Discord**: [邀请链接](https://discord.gg/easyssh)
- **邮件**: dev@easyssh.dev

## 下一步

- [架构说明](/zh/develop/architecture)
- [构建指南](/zh/develop/building)
- [测试指南](/zh/develop/testing)
- [代码规范](/zh/develop/coding-standards)
