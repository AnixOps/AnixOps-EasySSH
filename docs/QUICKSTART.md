# EasySSH Quick Start Guide | EasySSH 快速入门指南

---

## Table of Contents | 目录

1. [Introduction | 简介](#introduction--简介)
2. [Edition Selection Guide | 版本选择指南](#edition-selection-guide--版本选择指南)
3. [Installation | 安装](#installation--安装)
4. [First Steps | 第一步](#first-steps--第一步)
5. [Basic Operations | 基本操作](#basic-operations--基本操作)
6. [Next Steps | 下一步](#next-steps--下一步)

---

## Introduction | 简介

### What is EasySSH? | 什么是 EasySSH？

**English:**

EasySSH is a modern, secure, and cross-platform SSH client designed to simplify server management. Whether you're a solo developer managing a few servers or an IT team handling hundreds of connections, EasySSH has an edition tailored for your needs.

Key features include:
- **Secure Storage**: Your credentials are encrypted with industry-standard AES-256-GCM encryption
- **Native Performance**: Platform-specific UIs (egui on Windows, GTK4 on Linux, SwiftUI on macOS)
- **Keychain Integration**: Seamless integration with your system's secure credential storage
- **Server Organization**: Group and categorize your connections for easy management

**中文：**

EasySSH 是一款现代化、安全且跨平台的 SSH 客户端，旨在简化服务器管理。无论您是管理少量服务器的独立开发者，还是处理数百个连接的 IT 团队，EasySSH 都有适合您需求的版本。

主要特性包括：
- **安全存储**：使用业界标准的 AES-256-GCM 加密保护您的凭据
- **原生性能**：平台特定的 UI（Windows 上使用 egui，Linux 上使用 GTK4，macOS 上使用 SwiftUI）
- **Keychain 集成**：与系统安全凭据存储无缝集成
- **服务器组织**：分组和分类您的连接，便于管理

---

## Edition Selection Guide | 版本选择指南

### Lite Edition | Lite 版本

**English:**

Best for: Developers who prefer their native terminal and want a secure configuration vault.

| Feature | Availability |
|---------|--------------|
| Password/Key Authentication | Yes |
| Keychain Integration | Yes |
| Master Password Protection | Yes |
| Server Grouping (Single Level) | Yes |
| Search & Filter | Yes |
| Native Terminal Launch | Yes |

Choose Lite if you:
- Prefer using your existing terminal (Terminal.app, iTerm2, Windows Terminal, etc.)
- Want a secure place to store SSH configurations
- Don't need embedded terminal features
- Value simplicity and minimal resource usage

**中文：**

适合人群：喜欢使用原生终端并需要一个安全配置保险箱的开发者。

| 功能 | 可用性 |
|------|--------|
| 密码/密钥认证 | 是 |
| Keychain 集成 | 是 |
| 主密码保护 | 是 |
| 服务器分组（单层） | 是 |
| 搜索与过滤 | 是 |
| 原生终端启动 | 是 |

如果您符合以下情况，请选择 Lite：
- 喜欢使用现有终端（Terminal.app、iTerm2、Windows Terminal 等）
- 想要一个安全的地方存储 SSH 配置
- 不需要嵌入式终端功能
- 重视简洁和最小资源占用

---

### Standard Edition | Standard 版本

**English:**

Best for: Power users who need an integrated SSH experience with advanced features.

| Feature | Availability |
|---------|--------------|
| All Lite Features | Yes |
| Embedded Web Terminal | Yes |
| Multi-tab Support | Yes |
| Split Screen Layout | Yes |
| WebGL Acceleration | Yes |
| Server Grouping (Nested) | Yes |
| Batch Operations | Yes |
| SFTP File Management | Yes |
| Auto Reconnect | Yes |
| System Monitoring Widgets | Yes |
| Import ~/.ssh/config | Yes |

Choose Standard if you:
- Want an all-in-one SSH client
- Need to manage multiple servers simultaneously
- Use split-screen layouts for monitoring
- Require SFTP file transfer capabilities

**中文：**

适合人群：需要集成 SSH 体验和高级功能的高级用户。

| 功能 | 可用性 |
|------|--------|
| 所有 Lite 功能 | 是 |
| 嵌入式 Web 终端 | 是 |
| 多标签页支持 | 是 |
| 分屏布局 | 是 |
| WebGL 加速 | 是 |
| 服务器分组（嵌套） | 是 |
| 批量操作 | 是 |
| SFTP 文件管理 | 是 |
| 自动重连 | 是 |
| 系统监控小组件 | 是 |
| 导入 ~/.ssh/config | 是 |

如果您符合以下情况，请选择 Standard：
- 想要一体化 SSH 客户端
- 需要同时管理多台服务器
- 使用分屏布局进行监控
- 需要 SFTP 文件传输功能

---

### Pro Edition | Pro 版本

**English:**

Best for: Teams and organizations requiring collaboration and enterprise features.

| Feature | Availability |
|---------|--------------|
| All Standard Features | Yes |
| Team Management | Yes |
| RBAC Permissions | Yes |
| SSO (SAML/OIDC) | Yes |
| Shared Snippets | Yes |
| Audit Logs | Yes |
| End-to-End Encrypted Sync | Yes |
| Centralized Configuration | Yes |

Choose Pro if you:
- Manage an IT team
- Need compliance and audit capabilities
- Require SSO integration
- Share SSH configurations across team members

**中文：**

适合人群：需要协作和企业功能的团队和组织。

| 功能 | 可用性 |
|------|--------|
| 所有 Standard 功能 | 是 |
| 团队管理 | 是 |
| RBAC 权限控制 | 是 |
| SSO (SAML/OIDC) | 是 |
| 共享代码片段 | 是 |
| 审计日志 | 是 |
| 端到端加密同步 | 是 |
| 集中配置管理 | 是 |

如果您符合以下情况，请选择 Pro：
- 管理 IT 团队
- 需要合规和审计功能
- 需要 SSO 集成
- 在团队成员之间共享 SSH 配置

---

## Installation | 安装

### Windows | Windows 系统

**English:**

1. **Download**: Visit the [Releases page](https://github.com/AnixOps/AnixOps-EasySSH/releases) and download the latest `.msi` or `.exe` installer.

2. **Install**: Double-click the downloaded file and follow the installation wizard.

3. **Launch**: Find "EasySSH" in your Start Menu and click to launch.

**System Requirements:**
- Windows 10/11 (64-bit)
- No additional runtime required (egui provides native performance)

**中文：**

1. **下载**：访问 [发布页面](https://github.com/AnixOps/AnixOps-EasySSH/releases) 下载最新的 `.msi` 或 `.exe` 安装程序。

2. **安装**：双击下载的文件并按照安装向导操作。

3. **启动**：在开始菜单中找到 "EasySSH" 并点击启动。

**系统要求：**
- Windows 10/11（64 位）
- 无需额外运行时（egui 提供原生性能）

---

### Linux | Linux 系统

**English:**

#### Option 1: Package Manager | 选项 1：包管理器

```bash
# Debian/Ubuntu
sudo apt install easyssh

# Fedora
sudo dnf install easyssh

# Arch Linux (AUR)
yay -S easyssh
```

#### Option 2: Flatpak | 选项 2：Flatpak

```bash
flatpak install flathub com.easyssh.EasySSH
flatpak run com.easyssh.EasySSH
```

#### Option 3: AppImage | 选项 3：AppImage

```bash
# Download the AppImage
wget https://github.com/AnixOps/AnixOps-EasySSH/releases/latest/download/EasySSH-x86_64.AppImage

# Make it executable
chmod +x EasySSH-x86_64.AppImage

# Run
./EasySSH-x86_64.AppImage
```

**System Requirements:**
- GTK4 runtime
- Modern Linux distribution (Ubuntu 22.04+, Fedora 36+, etc.)

**中文：**

#### 选项 1：包管理器

```bash
# Debian/Ubuntu
sudo apt install easyssh

# Fedora
sudo dnf install easyssh

# Arch Linux (AUR)
yay -S easyssh
```

#### 选项 2：Flatpak

```bash
flatpak install flathub com.easyssh.EasySSH
flatpak run com.easyssh.EasySSH
```

#### 选项 3：AppImage

```bash
# 下载 AppImage
wget https://github.com/AnixOps/AnixOps-EasySSH/releases/latest/download/EasySSH-x86_64.AppImage

# 添加执行权限
chmod +x EasySSH-x86_64.AppImage

# 运行
./EasySSH-x86_64.AppImage
```

**系统要求：**
- GTK4 运行时
- 现代 Linux 发行版（Ubuntu 22.04+、Fedora 36+ 等）

---

### macOS | macOS 系统

**English:**

#### Option 1: Homebrew | 选项 1：Homebrew

```bash
brew install --cask easyssh
```

#### Option 2: DMG Installer | 选项 2：DMG 安装程序

1. Download the `.dmg` file from the [Releases page](https://github.com/AnixOps/AnixOps-EasySSH/releases)
2. Open the DMG file
3. Drag EasySSH to your Applications folder
4. Launch from Applications or Spotlight

**System Requirements:**
- macOS 12.0 (Monterey) or later
- Apple Silicon (M1/M2/M3) and Intel Macs are both supported

**中文：**

#### 选项 1：Homebrew

```bash
brew install --cask easyssh
```

#### 选项 2：DMG 安装程序

1. 从 [发布页面](https://github.com/AnixOps/AnixOps-EasySSH/releases) 下载 `.dmg` 文件
2. 打开 DMG 文件
3. 将 EasySSH 拖入应用程序文件夹
4. 从应用程序或 Spotlight 启动

**系统要求：**
- macOS 12.0 (Monterey) 或更高版本
- 支持 Apple Silicon (M1/M2/M3) 和 Intel Mac

---

## First Steps | 第一步

### Creating Your First Server Connection | 创建您的第一个服务器连接

**English:**

1. **Open EasySSH**: Launch the application on your system.

2. **Add New Server**: Click the "+" button or use the keyboard shortcut `Ctrl+N` (Windows/Linux) or `Cmd+N` (macOS).

3. **Fill in Connection Details**:
   - **Name**: A friendly name for your server (e.g., "Production Web Server")
   - **Host**: The server's IP address or domain name
   - **Port**: SSH port (default: 22)
   - **Username**: Your SSH username
   - **Authentication Method**: Choose from:
     - Password
     - SSH Key
     - SSH Agent

4. **Save**: Click "Save" to store the connection.

**中文：**

1. **打开 EasySSH**：在您的系统上启动应用程序。

2. **添加新服务器**：点击 "+" 按钮或使用快捷键 `Ctrl+N`（Windows/Linux）或 `Cmd+N`（macOS）。

3. **填写连接信息**：
   - **名称**：服务器的友好名称（例如："生产 Web 服务器"）
   - **主机**：服务器的 IP 地址或域名
   - **端口**：SSH 端口（默认：22）
   - **用户名**：您的 SSH 用户名
   - **认证方式**：选择：
     - 密码
     - SSH 密钥
     - SSH Agent

4. **保存**：点击"保存"以存储连接。

---

### Using Keychain for Credentials | 使用 Keychain 管理凭据

**English:**

EasySSH integrates seamlessly with your system's secure credential storage:

| Platform | Keychain Service |
|----------|------------------|
| Windows | Windows Credential Manager |
| macOS | Keychain Access |
| Linux | Secret Service (GNOME Keyring, KWallet) |

**To store a password in Keychain:**

1. When adding or editing a server, select "Password" authentication
2. Enter your password
3. Check "Store in Keychain" option
4. Save the connection

**Benefits:**
- Your passwords never leave the secure storage
- Auto-fill on future connections
- No master password required (Lite edition)

**中文：**

EasySSH 与您系统的安全凭据存储无缝集成：

| 平台 | Keychain 服务 |
|------|---------------|
| Windows | Windows 凭据管理器 |
| macOS | 钥匙串访问 |
| Linux | Secret Service（GNOME Keyring、KWallet）|

**将密码存储到 Keychain：**

1. 添加或编辑服务器时，选择"密码"认证
2. 输入您的密码
3. 勾选"存储到 Keychain"选项
4. 保存连接

**优势：**
- 您的密码永远不会离开安全存储
- 未来连接时自动填充
- 无需主密码（Lite 版本）

---

### Organizing Servers with Groups | 使用分组组织服务器

**English:**

**Lite Edition (Single Level Groups):**

1. Click the folder icon or `Ctrl+G` / `Cmd+G`
2. Enter a group name (e.g., "Production", "Development", "Clients")
3. Drag servers into the group
4. Collapse/expand groups for organization

**Standard/Pro Edition (Nested Groups):**

1. Create groups as above
2. Right-click a group to create sub-groups
3. Build hierarchical structures like:
   ```
   Production
   ├── Web Servers
   │   ├── US East
   │   └── US West
   └── Database Servers
       ├── Primary
       └── Replicas
   ```

**中文：**

**Lite 版本（单层分组）：**

1. 点击文件夹图标或按 `Ctrl+G` / `Cmd+G`
2. 输入分组名称（例如："生产环境"、"开发环境"、"客户"）
3. 将服务器拖入分组
4. 折叠/展开分组进行组织

**Standard/Pro 版本（嵌套分组）：**

1. 按上述方式创建分组
2. 右键点击分组创建子分组
3. 构建层次结构，如：
   ```
   生产环境
   ├── Web 服务器
   │   ├── 美国东部
   │   └── 美国西部
   └── 数据库服务器
       ├── 主库
       └── 从库
   ```

---

## Basic Operations | 基本操作

### Connecting to a Server | 连接到服务器

**English:**

**Lite Edition:**
1. Find your server in the list (or use search: `Ctrl+F` / `Cmd+F`)
2. Double-click the server entry
3. EasySSH will launch your system's default terminal with the SSH connection

**Standard/Pro Edition:**
1. Find your server in the list
2. Double-click or press Enter
3. The embedded terminal opens in a new tab
4. For split view: Right-click → "Open in Split Pane"

**中文：**

**Lite 版本：**
1. 在列表中找到您的服务器（或使用搜索：`Ctrl+F` / `Cmd+F`）
2. 双击服务器条目
3. EasySSH 将启动系统默认终端并建立 SSH 连接

**Standard/Pro 版本：**
1. 在列表中找到您的服务器
2. 双击或按 Enter 键
3. 嵌入式终端在新标签页中打开
4. 分屏查看：右键点击 → "在分屏窗格中打开"

---

### Managing SSH Keys | 管理 SSH 密钥

**English:**

**Adding a New SSH Key:**

1. Go to **Settings** → **SSH Keys**
2. Click **Import Key**
3. Choose:
   - **Existing Key**: Select your private key file (usually in `~/.ssh/`)
   - **Generate New**: Create a new key pair
4. Optionally add a passphrase for extra security
5. Save the key

**Generating a New Key:**

```
Key Type: ED25519 (recommended) or RSA (2048/4096 bits)
Key Name: A descriptive name
Passphrase: Optional but recommended
```

**Using Keys with Servers:**

1. Edit a server connection
2. Select "SSH Key" as authentication method
3. Choose the key from your saved keys
4. Save the connection

**中文：**

**添加新的 SSH 密钥：**

1. 进入 **设置** → **SSH 密钥**
2. 点击 **导入密钥**
3. 选择：
   - **现有密钥**：选择您的私钥文件（通常在 `~/.ssh/`）
   - **生成新密钥**：创建新的密钥对
4. 可选：添加密码短语以增强安全性
5. 保存密钥

**生成新密钥：**

```
密钥类型：ED25519（推荐）或 RSA（2048/4096 位）
密钥名称：描述性名称
密码短语：可选但推荐
```

**在服务器上使用密钥：**

1. 编辑服务器连接
2. 选择"SSH 密钥"作为认证方式
3. 从已保存的密钥中选择
4. 保存连接

---

### Using the Search Feature | 使用搜索功能

**English:**

Quickly find servers using the powerful search feature:

**Keyboard Shortcut:** `Ctrl+F` (Windows/Linux) or `Cmd+F` (macOS)

**Search by:**
- Server name
- Host/IP address
- Username
- Group name
- Custom tags (Pro edition)

**Search Tips:**
- Use partial matches: "prod" finds "Production Server"
- Search is case-insensitive
- Use filters to narrow by group or authentication type

**中文：**

使用强大的搜索功能快速找到服务器：

**快捷键：** `Ctrl+F`（Windows/Linux）或 `Cmd+F`（macOS）

**搜索范围：**
- 服务器名称
- 主机/IP 地址
- 用户名
- 分组名称
- 自定义标签（Pro 版本）

**搜索技巧：**
- 支持部分匹配："prod" 可以找到 "Production Server"
- 搜索不区分大小写
- 使用过滤器按分组或认证类型缩小范围

---

## Next Steps | 下一步

### Continue Learning | 继续学习

**English:**

Ready to explore more? Check out these resources:

| Resource | Description |
|----------|-------------|
| [Full User Guide](product/en/index.md) | Comprehensive documentation for all features |
| [Advanced Features](features/lite-features.md) | ProxyJump, SSH Agent forwarding, and more |
| [Security Best Practices](security/audit-report.md) | Keep your connections secure |
| [Troubleshooting](developers/TROUBLESHOOTING.md) | Common issues and solutions |

**中文：**

准备好探索更多了吗？查看这些资源：

| 资源 | 描述 |
|------|------|
| [完整用户指南](product/zh/guide/index.md) | 所有功能的综合文档 |
| [高级功能](features/lite-features.md) | ProxyJump、SSH Agent 转发等 |
| [安全最佳实践](security/audit-report.md) | 保持您的连接安全 |
| [故障排查](developers/TROUBLESHOOTING.md) | 常见问题和解决方案 |

---

### Community & Support | 社区与支持

**English:**

- **GitHub Issues**: [Report bugs or request features](https://github.com/AnixOps/AnixOps-EasySSH/issues)
- **Discussions**: [Join the community](https://github.com/AnixOps/AnixOps-EasySSH/discussions)
- **Documentation**: [Full docs](INDEX.md)

**中文：**

- **GitHub Issues**：[报告问题或请求功能](https://github.com/AnixOps/AnixOps-EasySSH/issues)
- **讨论区**：[加入社区](https://github.com/AnixOps/AnixOps-EasySSH/discussions)
- **文档**：[完整文档](INDEX.md)

---

## Quick Reference Card | 快速参考卡

### Keyboard Shortcuts | 键盘快捷键

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| New Connection | `Ctrl+N` | `Cmd+N` |
| Search | `Ctrl+F` | `Cmd+F` |
| New Group | `Ctrl+G` | `Cmd+G` |
| Connect | `Enter` | `Enter` |
| Edit | `F2` or `Ctrl+E` | `F2` or `Cmd+E` |
| Delete | `Delete` | `Delete` |
| Settings | `Ctrl+,` | `Cmd+,` |
| Quit | `Ctrl+Q` | `Cmd+Q` |

---

**Happy SSH-ing! | 祝您使用愉快！**

*EasySSH Team*