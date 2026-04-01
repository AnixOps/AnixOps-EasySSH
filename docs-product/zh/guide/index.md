# 快速开始

欢迎开始使用 EasySSH！本指南将帮助你在 5 分钟内完成安装并开始使用。

## 选择版本

EasySSH 提供三个版本，请根据需求选择：

| 版本 | 适合用户 | 核心功能 |
|------|----------|----------|
| **Lite** | 注重隐私的开发者 | 安全存储、一键连接、原生终端 |
| **Standard** | 多服务器管理者 | 嵌入式终端、分屏、SFTP |
| **Pro** | IT 团队/企业 | 团队协作、审计、SSO |

## 安装

### macOS

::: tabs
@tab Lite
```bash
brew install easyssh-lite
```
@tab Standard
```bash
brew install easyssh
# 或下载 DMG
# https://easyssh.dev/download/macos-standard
```
@tab Pro
```bash
brew install easyssh
# Pro 需要服务端部署，详见部署文档
```
:::

### Windows

::: tabs
@tab Lite
```powershell
winget install EasySSH.Lite
# 或从 Microsoft Store 安装
```
@tab Standard
```powershell
winget install EasySSH
# 或下载安装包
# https://easyssh.dev/download/windows
```
@tab Pro
```powershell
winget install EasySSH
# Pro 需要服务端部署
```
:::

### Linux

::: tabs
@tab Ubuntu/Debian
```bash
# Lite
curl -fsSL https://easyssh.dev/install.sh | sh -s -- lite

# Standard
curl -fsSL https://easyssh.dev/install.sh | sh
```
@tab Fedora
```bash
sudo dnf copr enable easyssh/easyssh
sudo dnf install easyssh
```
@tab Arch
```bash
yay -S easyssh
# 或
paru -S easyssh
```
:::

## 首次启动

### Lite 版

```bash
# 启动应用
easyssh

# 设置主密码（首次）
# 这将用于加密你的配置数据

# 添加第一个服务器
easyssh add-server \
  --name "My Server" \
  --host "192.168.1.100" \
  --port 22 \
  --user "admin" \
  --auth-type "key" \
  --key-path "~/.ssh/id_rsa"
```

### Standard 版

```bash
# 启动应用
easyssh

# 或使用图形界面
# 在应用菜单中找到 EasySSH

# 首次启动向导：
# 1. 选择数据存储位置
# 2. 设置主密码（可选）
# 3. 导入现有配置（可选）
```

### Pro 版

```bash
# Pro 版需要先部署服务端
# 详见 [企业部署指南](/zh/deploy/enterprise)

# 然后配置客户端连接服务端
easyssh --server https://easyssh.company.com
```

## 基本操作

### 添加服务器

**图形界面方式：**
1. 点击左下角「+」按钮
2. 填写服务器信息：
   - 名称：用于识别的别名
   - 主机：IP 地址或域名
   - 端口：默认为 22
   - 用户名：登录账户
   - 认证方式：密码或密钥
3. 点击「测试连接」验证
4. 保存

**命令行方式：**

```bash
# 密码认证
easyssh add-server \
  --name "Web Server" \
  --host "web.example.com" \
  --user "ubuntu" \
  --auth-type "password"

# 密钥认证
easyssh add-server \
  --name "Database" \
  --host "db.internal" \
  --user "postgres" \
  --auth-type "key" \
  --key-path "~/.ssh/production"

# Agent 认证
easyssh add-server \
  --name "Staging" \
  --host "staging.internal" \
  --user "deploy" \
  --auth-type "agent"
```

### 连接服务器

**Lite 版：**
1. 在服务器列表中点击目标服务器
2. 选择「连接」
3. 系统自动唤起原生终端并建立连接

**Standard/Pro 版：**
1. 双击服务器卡片
2. 或在服务器上右键选择「在新标签页打开」
3. 嵌入式终端将显示在中央区域

### 导入现有配置

```bash
# 从 ~/.ssh/config 导入
easyssh import --source ssh-config

# 从 Termius 导出文件导入
easyssh import --source termius --file ~/Downloads/termius-export.csv

# 从其他工具导入
easyssh import --source mobaxterm --file ~/moba-export.txt
```

## 下一步

- [Lite 版功能详解](/zh/guide/features/lite)
- [Standard 版功能详解](/zh/guide/features/standard)
- [Pro 版功能详解](/zh/guide/features/pro)
- [快捷键参考](/zh/guide/shortcuts)

## 常见问题

**Q: Lite 和 Standard 的数据可以迁移吗？**
A: 可以。Standard 完全兼容 Lite 的数据格式，安装 Standard 后会自动读取 Lite 的数据。

**Q: 可以同时在多台设备使用吗？**
A: Lite/Standard 支持本地导出/导入。Pro 版支持云端同步。

**Q: 忘记主密码怎么办？**
A: Lite/Standard 使用本地加密，忘记主密码无法恢复数据。Pro 版管理员可重置密码。

**Q: 支持哪些认证方式？**
A: 所有版本都支持密码认证、SSH 密钥认证、SSH Agent 认证。Pro 版还支持 SSO (SAML/OIDC)。
