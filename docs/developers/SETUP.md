# EasySSH 开发环境设置指南

> 从零开始配置完整的 EasySSH 开发环境

---

## 目录

1. [系统要求](#1-系统要求)
2. [依赖安装](#2-依赖安装)
3. [项目初始化](#3-项目初始化)
4. [IDE配置](#4-ide配置)
5. [环境验证](#5-环境验证)
6. [常见问题](#6-常见问题)

---

## 1. 系统要求

### 1.1 最低配置

| 组件 | 要求 |
|------|------|
| **操作系统** | Windows 10/11, macOS 12+, Ubuntu 20.04+ |
| **内存** | 8 GB RAM (建议 16 GB) |
| **磁盘** | 10 GB 可用空间 |
| **网络** | 稳定的互联网连接 |

### 1.2 推荐配置

| 组件 | 推荐 |
|------|------|
| **CPU** | 8核心以上 |
| **内存** | 32 GB RAM |
| **磁盘** | SSD 50 GB 可用空间 |
| **显示器** | 1920x1080 或更高 |

---

## 2. 依赖安装

### 2.1 Rust 工具链

```bash
# 安装 Rust (使用 rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 或 Windows PowerShell
# winget install Rustlang.Rustup

# 配置环境变量
source $HOME/.cargo/env  # Linux/macOS
# Windows: 自动配置

# 安装必要组件
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown
```

### 2.2 Node.js 和 pnpm

```bash
# 安装 Node.js 20 LTS
# macOS/Linux
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# Windows
winget install OpenJS.NodeJS.LTS

# 安装 pnpm
npm install -g pnpm

# 配置 pnpm
pnpm config set store-dir ~/.pnpm-store
```

### 2.3 平台特定依赖

#### Windows

```powershell
# 安装 Visual Studio Build Tools
winget install Microsoft.VisualStudio.2022.BuildTools

# 安装 Windows SDK
winget install Microsoft.WindowsSDK

# 安装 LLVM (用于 bindings)
winget install LLVM.LLVM

# 安装 Git
winget install Git.Git
```

#### macOS

```bash
# 安装 Xcode Command Line Tools
xcode-select --install

# 安装 Homebrew
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# 安装依赖
brew install openssl sqlite3 pkg-config
```

#### Linux (Ubuntu/Debian)

```bash
# 安装基础依赖
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    libgtk-4-dev \
    libadwaita-1-dev \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf
```

### 2.4 GTK4 和 libadwaita (Linux)

```bash
# Ubuntu 22.04+
sudo apt-get install -y \
    libgtk-4-dev \
    libadwaita-1-dev \
    libgranite-7-dev

# Fedora
sudo dnf install gtk4-devel libadwaita-devel

# Arch
sudo pacman -S gtk4 libadwaita
```

---

## 3. 项目初始化

### 3.1 克隆仓库

```bash
# 克隆主仓库
git clone https://github.com/anixops/easyssh.git
cd easyssh

# 或克隆特定分支
git clone -b develop https://github.com/anixops/easyssh.git
```

### 3.2 安装 Rust 依赖

```bash
# 安装 workspace 依赖
cargo fetch

# 验证核心库编译
cargo build -p easyssh-core
```

### 3.3 构建前端 (Tauri 版本)

```bash
# 进入 Tauri 前端目录
cd platforms/desktop

# 安装依赖
pnpm install

# 生成类型定义
pnpm tauri dev
```

### 3.4 初始化数据库

```bash
# 创建开发数据库目录
mkdir -p ~/.local/share/easyssh
cd core

# 运行数据库迁移测试
cargo test db::tests::test_migrations -- --nocapture
```

---

## 4. IDE配置

### 4.1 VS Code 推荐配置

```json
// .vscode/settings.json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.procMacro.enable": true,
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "[typescript]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  },
  "typescript.tsdk": "node_modules/typescript/lib"
}
```

```json
// .vscode/extensions.json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "serayuzgur.crates",
    "vadimcn.vscode-lldb",
    "esbenp.prettier-vscode",
    "bradlc.vscode-tailwindcss",
    "tauri-apps.tauri-vscode"
  ]
}
```

### 4.2 Rust 专用配置

```toml
# rustfmt.toml
edition = "2021"
max_width = 100
tab_spaces = 4
```

```toml
# clippy.toml
cognitive-complexity-threshold = 30
too-many-arguments-threshold = 8
type-complexity-threshold = 500
```

### 4.3 调试配置

```json
// .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug EasySSH Core",
      "cargo": {
        "args": ["build", "-p", "easyssh-core"],
        "filter": {
          "name": "easyssh-core",
          "kind": "lib"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug GTK4 App",
      "cargo": {
        "args": ["build", "-p", "easyssh-gtk4"],
        "filter": {
          "name": "easyssh-gtk4",
          "kind": "bin"
        }
      },
      "args": [],
      "env": {
        "RUST_LOG": "easyssh_gtk4=debug"
      },
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

---

## 5. 环境验证

### 5.1 运行测试套件

```bash
# 运行核心库测试
cargo test -p easyssh-core --lib

# 运行集成测试
cargo test --test integration_tests

# 运行所有测试
cargo test --workspace
```

### 5.2 构建检查

```bash
# 检查代码格式
cargo fmt -- --check

# 运行 Clippy
cargo clippy --workspace -- -D warnings

# 构建所有目标
cargo build --workspace
```

### 5.3 运行应用

```bash
# GTK4 版本 (Linux)
cargo run -p easyssh-gtk4

# Tauri 版本 (所有平台)
cd platforms/desktop && pnpm tauri dev

# Windows 原生版本
cargo run -p easyssh-winui

# TUI 版本
cargo run -p easyssh-tui
```

### 5.4 验证脚本

```bash
# 创建验证脚本
#!/bin/bash
set -e

echo "=== EasySSH 环境验证 ==="

echo "1. 检查 Rust 版本..."
rustc --version
cargo --version

echo "2. 检查 Node.js..."
node --version
pnpm --version

echo "3. 检查依赖..."
cargo check --workspace

echo "4. 运行单元测试..."
cargo test -p easyssh-core --lib --quiet

echo "5. 构建核心库..."
cargo build -p easyssh-core --release

echo "✅ 环境验证通过！"
```

---

## 6. 常见问题

### 6.1 编译错误

#### OpenSSL 链接错误 (Linux)

```bash
# 错误: openssl-sys 找不到 openssl
sudo apt-get install libssl-dev pkg-config

# 或指定路径
export OPENSSL_DIR=/usr/lib/ssl
export OPENSSL_INCLUDE_DIR=/usr/include/openssl
```

#### SQLite 绑定错误

```bash
# 错误: libsqlite3-sys 编译失败
export SQLITE3_LIB_DIR=/usr/lib/x86_64-linux-gnu
export SQLITE3_INCLUDE_DIR=/usr/include

# 或使用 bundled 版本 (推荐)
# 在 Cargo.toml 中启用 bundled feature
```

#### Windows 链接错误

```powershell
# 错误: 找不到 link.exe
# 安装 Visual Studio Build Tools 并运行:
"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
```

### 6.2 运行时问题

#### GTK4 主题加载失败

```bash
# Linux 下 GTK4 主题问题
export GTK_THEME=Adwaita:dark

# 或安装主题
sudo apt-get install gnome-themes-extra
```

#### Keychain 访问失败

```bash
# Linux 下需要安装 keyring 后端
sudo apt-get install gnome-keyring libsecret-1-0

# 或安装 pass
sudo apt-get install pass
```

### 6.3 性能问题

#### 编译太慢

```bash
# 使用 sccache 加速
 cargo install sccache
 export RUSTC_WRAPPER=sccache

# 或启用增量编译
 export CARGO_INCREMENTAL=1
```

#### 内存不足

```bash
# 限制并行编译作业数
cargo build -j 2

# 或使用 release 配置优化内存
# 在 Cargo.toml 中设置 codegen-units = 1
```

---

## 7. 开发工作流

### 7.1 日常开发命令

```bash
# 快速检查
cargo check -p easyssh-core

# 格式化代码
cargo fmt

# 修复 Clippy 警告
cargo clippy --fix --allow-dirty

# 运行特定测试
cargo test test_name -- --nocapture
```

### 7.2 提交前检查清单

- [ ] `cargo fmt` 通过
- [ ] `cargo clippy` 无警告
- [ ] `cargo test` 全部通过
- [ ] 文档已更新
- [ ] CHANGELOG 已更新

---

## 8. 相关文档

- [调试指南](./DEBUGGING.md) - 故障排查和调试技巧
- [测试指南](./TESTING.md) - 测试策略和最佳实践
- [性能分析指南](./PROFILING.md) - 性能优化工具
- [故障排除指南](./TROUBLESHOOTING.md) - 常见问题解决方案

---

*最后更新: 2026-04-01*
