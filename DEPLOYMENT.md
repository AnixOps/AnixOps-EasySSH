# 部署文档

> EasySSH 部署指南 - 涵盖开发、测试、生产环境的完整部署流程

## 目录

1. [环境要求](#环境要求)
2. [开发环境部署](#开发环境部署)
3. [构建配置](#构建配置)
4. [生产环境部署](#生产环境部署)
5. [Docker 部署](#docker-部署)
6. [CI/CD 配置](#cicd-配置)
7. [更新与回滚](#更新与回滚)
8. [故障排除](#故障排除)

---

## 环境要求

### 系统要求

| 组件 | 最低版本 | 推荐版本 |
|------|----------|----------|
| Rust | 1.75.0 | 1.78.0+ |
| Node.js | 18.0.0 | 20.0.0+ |
| SQLite | 3.35.0 | 3.40.0+ |
| OpenSSL | 1.1.1 | 3.0.0+ |

### 平台特定要求

#### Linux (GTK4)

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    libgtk-4-dev \
    libadwaita-1-dev \
    libssl-dev \
    pkg-config \
    libssh2-1-dev \
    libsqlite3-dev

# Fedora/RHEL
sudo dnf install \
    gtk4-devel \
    libadwaita-devel \
    openssl-devel \
    pkg-config \
    libssh2-devel \
    sqlite-devel
```

#### macOS

```bash
# 安装 Xcode Command Line Tools
xcode-select --install

# 使用 Homebrew 安装依赖
brew install gtk4 libadwaita openssl sqlite pkg-config
```

#### Windows

```powershell
# 使用 vcpkg 安装依赖
vcpkg install openssl:x64-windows
vcpkg install sqlite3:x64-windows
vcpkg install libssh2:x64-windows

# 或使用 chocolatey
choco install openssl sqlite
```

---

## 开发环境部署

### 1. 克隆仓库

```bash
git clone https://github.com/anixops/easyssh.git
cd easyssh
```

### 2. 安装 Rust 工具链

```bash
# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 安装稳定版工具链
rustup install stable
rustup default stable

# 安装目标平台
rustup target add x86_64-pc-windows-msvc    # Windows
rustup target add x86_64-unknown-linux-gnu  # Linux
rustup target add x86_64-apple-darwin       # macOS Intel
rustup target add aarch64-apple-darwin      # macOS Apple Silicon
```

### 3. 安装 Node.js 依赖 (Tauri 前端)

```bash
cd tauri-ui
npm install
```

### 4. 构建核心库

```bash
# 构建 Lite 版本
cargo build --package easyssh-core --features lite

# 构建 Standard 版本
cargo build --package easyssh-core --features standard

# 构建完整 Pro 版本
cargo build --package easyssh-core --features pro
```

### 5. 运行测试

```bash
# 单元测试
cargo test --package easyssh-core

# 集成测试
cargo test --package easyssh-core --features standard -- --test-threads=1

# 端到端测试
cd tauri-ui
npm run test:e2e
```

---

## 构建配置

### Cargo.toml 配置示例

```toml
[package]
name = "easyssh"
version = "0.3.0"
edition = "2021"

[dependencies]
easyssh-core = { path = "../core", features = ["standard", "sftp"] }
tokio = { version = "1", features = ["full"] }

[features]
default = ["standard"]
lite = ["easyssh-core/lite"]
standard = ["easyssh-core/standard", "easyssh-core/sftp"]
pro = ["easyssh-core/pro"]

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.dev]
opt-level = 1
debug = true
```

### 特性组合表

| 目标版本 | 推荐特性组合 |
|----------|--------------|
| Lite | `lite` |
| Standard | `standard,sftp,split-screen,monitoring` |
| Pro | `pro,backup,sync,sso,audit` |
| DevOps | `standard,docker,kubernetes,monitoring` |
| Enterprise | `pro,backup,auto-update,workflow,telemetry` |

---

## 生产环境部署

### 二进制发布

#### Linux AppImage

```bash
# 构建 AppImage
cargo install cargo-appimage
cargo appimage --package easyssh-gtk4

# 输出: target/appimage/easyssh-x86_64.AppImage
```

#### macOS .app Bundle

```bash
# 构建 .app 包
cargo bundle --package easyssh-mac

# 签名（需要 Apple Developer ID）
codesign --deep --force --verify --verbose \
    --sign "Developer ID Application: Your Name" \
    target/release/bundle/osx/EasySSH.app

# 公证
xcrun altool --notarize-app \
    --primary-bundle-id "com.anixops.easyssh" \
    --username "your@email.com" \
    --password "@keychain:AC_PASSWORD" \
    --file target/release/bundle/osx/EasySSH.app
```

#### Windows MSI Installer

```bash
# 构建 MSI
cargo install cargo-wix
cargo wix --package easyssh-winui

# 签名（需要 Windows 证书）
signtool sign /f certificate.pfx /p password \
    /tr http://timestamp.digicert.com \
    /td sha256 \
    target/wix/easyssh-0.3.0-x86_64.msi
```

### 配置文件

#### 数据库配置

```yaml
# config/database.yml
development:
  path: ~/.easyssh/dev.db

production:
  path: ~/.easyssh/easyssh.db
  backup_enabled: true
  backup_interval: 86400  # 24 hours
```

#### SSH 配置

```yaml
# config/ssh.yml
connection_pool:
  max_connections: 50
  idle_timeout: 600
  max_age: 3600
  health_check_interval: 30

retry_policy:
  max_retries: 3
  base_delay_ms: 1000
  max_delay_ms: 10000
```

#### 日志配置

```yaml
# config/log.yml
level: info
format: json
output: file
path: ~/.easyssh/logs
rotation: daily
max_files: 30
```

---

## Docker 部署

### Dockerfile

```dockerfile
# Dockerfile
FROM rust:1.78-slim as builder

# 安装依赖
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    libsqlite3-dev \
    libssh2-1-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN cargo build --release --package easyssh-core --features standard

# 运行时镜像
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    libsqlite3-0 \
    libssh2-1 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/libeasyssh_core.so /usr/lib/
COPY --from=builder /build/target/release/examples/server /usr/local/bin/easyssh-server

EXPOSE 8080

CMD ["easyssh-server"]
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  easyssh:
    build: .
    container_name: easyssh
    ports:
      - "8080:8080"
    volumes:
      - easyssh_data:/data
      - ./config:/config:ro
    environment:
      - EASYSSH_CONFIG=/config/production.yml
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  backup:
    image: offen/docker-volume-backup:latest
    volumes:
      - easyssh_data:/backup/data:ro
      - /var/run/docker.sock:/var/run/docker.sock:ro
    environment:
      - BACKUP_CRON_EXPRESSION=0 2 * * *
      - BACKUP_RETENTION_DAYS=30
    restart: unless-stopped

volumes:
  easyssh_data:
```

### 构建和运行

```bash
# 构建镜像
docker-compose build

# 启动服务
docker-compose up -d

# 查看日志
docker-compose logs -f easyssh

# 停止服务
docker-compose down
```

---

## CI/CD 配置

### GitHub Actions

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }} --features standard

      - name: Package
        shell: bash
        run: |
          cd target/${{ matrix.target }}/release
          if [[ "${{ matrix.os }}" == "windows-latest" ]]; then
            7z a ../../../easyssh-${{ matrix.target }}.zip easyssh.exe
          else
            tar czvf ../../../easyssh-${{ matrix.target }}.tar.gz easyssh
          fi

      - name: Upload
        uses: actions/upload-artifact@v4
        with:
          name: easyssh-${{ matrix.target }}
          path: easyssh-${{ matrix.target }}.*

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          pattern: easyssh-*
          merge-multiple: true

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: easyssh-*
          draft: true
          prerelease: false
```

### 自动化测试

```yaml
# .github/workflows/test.yml
name: Test

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-4-dev libadwaita-1-dev

      - name: Run tests
        run: cargo test --all-features

      - name: Run clippy
        run: cargo clippy --all-features -- -D warnings

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --all-features --out xml

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          file: ./cobertura.xml
```

---

## 更新与回滚

### 自动更新 (Auto-update)

```rust
#[cfg(feature = "auto-update")]
use easyssh_core::auto_update::AutoUpdater;

// 配置更新器
let updater = AutoUpdater::new()
    .with_channel("stable")
    .with_check_interval(3600);  // 每小时检查

// 检查更新
match updater.check().await {
    Ok(Some(update)) => {
        println!("New version available: {}", update.version);

        // 下载并安装
        updater.download_and_install(update).await?;
    }
    Ok(None) => println!("No updates available"),
    Err(e) => eprintln!("Update check failed: {}", e),
}
```

### 数据库迁移

```bash
# 运行迁移
cargo run --bin migrate -- up

# 回滚迁移
cargo run --bin migrate -- down

# 查看状态
cargo run --bin migrate -- status
```

### 备份策略

```bash
# 手动备份
cargo run --bin easyssh -- backup create

# 自动备份配置
# ~/.easyssh/config.yml
backup:
  enabled: true
  interval_hours: 24
  retention_days: 30
  cloud_sync:
    provider: s3
    bucket: my-backup-bucket
```

---

## 故障排除

### 常见问题

#### 1. 构建失败 - 缺少依赖

```bash
# Linux: 安装开发包
sudo apt-get install libssl-dev pkg-config

# macOS: 使用 Homebrew
brew install openssl pkg-config
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"

# Windows: 使用 vcpkg
vcpkg install openssl:x64-windows
```

#### 2. 运行时崩溃 - OpenSSL 版本

```bash
# 检查 OpenSSL 版本
openssl version

# 更新 OpenSSL（如果需要）
# Linux
sudo apt-get install --only-upgrade libssl3

# macOS
brew upgrade openssl
```

#### 3. 数据库锁定

```bash
# 检查并修复 SQLite 数据库
sqlite3 ~/.easyssh/easyssh.db ".tables"
sqlite3 ~/.easyssh/easyssh.db "PRAGMA integrity_check;"

# 删除锁定文件
rm -f ~/.easyssh/easyssh.db-journal
rm -f ~/.easyssh/easyssh.db-wal
rm -f ~/.easyssh/easyssh.db-shm
```

#### 4. SSH 连接问题

```bash
# 启用详细日志
export RUST_LOG=easyssh_core=debug,ssh2=debug

# 检查 SSH agent
ssh-add -l

# 测试连接
ssh -v user@host
```

### 日志分析

```bash
# 查看实时日志
tail -f ~/.easyssh/logs/app.log

# 搜索错误
grep -i error ~/.easyssh/logs/app.log

# 按模块过滤
RUST_LOG=easyssh_core::ssh=debug cargo run
```

### 性能调优

```toml
# Cargo.toml - 优化配置
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"

[profile.release.build-override]
opt-level = 3
```

```yaml
# config/performance.yml
connection_pool:
  max_connections: 100
  idle_timeout: 300

ssh:
  keepalive_interval: 30
  compression: true

ui:
  render_fps: 60
  animation_duration: 150
```

---

## 安全部署检查清单

- [ ] 启用编译优化和 strip
- [ ] 配置防火墙规则
- [ ] 启用审计日志
- [ ] 设置定期备份
- [ ] 配置 SSL/TLS 证书
- [ ] 设置强密码策略
- [ ] 启用双因素认证（Pro）
- [ ] 配置会话超时
- [ ] 限制并发连接数
- [ ] 设置资源使用限制

---

## 支持

遇到问题？

- 查看 [故障排除](#故障排除) 章节
- 搜索 [Issues](https://github.com/anixops/easyssh/issues)
- 提交新 Issue
- 联系 support@easyssh.dev
