# 常见问题解答 (FAQ)

## 一般问题

### EasySSH 是什么？

EasySSH 是一个面向开发者和团队的 SSH 客户端产品线，提供三个版本：
- **Lite**: SSH 配置保险箱 - 安全存储 + 一键连接
- **Standard**: 全功能个人工作台 - 嵌入式终端 + 分屏 + SFTP
- **Pro**: 团队协作平台 - 团队管理 + 审计 + SSO

### EasySSH 与其他 SSH 客户端有什么不同？

| 特性 | EasySSH | Termius | PuTTY | iTerm2 |
|------|---------|---------|-------|--------|
| 本地加密 | ✅ | ⚠️ (可选) | ❌ | ❌ |
| 原生终端唤起 | ✅ (Lite) | ❌ | ❌ | N/A |
| 嵌入式终端 | ✅ (Standard+) | ✅ | ❌ | ✅ |
| 团队协作 | ✅ (Pro) | ✅ | ❌ | ❌ |
| 审计日志 | ✅ (Pro) | ✅ | ❌ | ❌ |
| 开源 | ✅ | ❌ | ✅ | ❌ |

### EasySSH 是免费的吗？

- **Lite**: 完全免费，开源
- **Standard**: 付费订阅（$9.9/月）或一次性购买
- **Pro**: 按用户数付费（$19.9/人/月）

教育用户和开源项目维护者可申请免费使用 Standard/Pro。

### 数据存储在哪里？

| 版本 | 存储位置 | 加密 |
|------|----------|------|
| Lite | 本地 SQLite | AES-256-GCM |
| Standard | 本地 SQLite | AES-256-GCM |
| Pro | 本地 + 服务端 | AES-256-GCM + TLS |

Lite 和 Standard 的数据完全本地存储，不上传云端。

### 支持哪些操作系统？

| 平台 | Lite | Standard | Pro 客户端 |
|------|:----:|:--------:|:----------:|
| macOS 10.15+ | ✅ | ✅ | ✅ |
| Windows 10+ | ✅ | ✅ | ✅ |
| Ubuntu 20.04+ | ✅ | ✅ | ✅ |
| Fedora 35+ | ✅ | ✅ | ✅ |
| iOS/iPadOS | 🔄 | 🔄 | 🔄 |
| Android | 🔄 | 🔄 | 🔄 |

🔄 = 开发中

### 如何获取支持？

- **文档**: https://docs.easyssh.dev
- **GitHub Issues**: https://github.com/anixops/easyssh/issues
- **Discord**: https://discord.gg/easyssh
- **邮件**: support@easyssh.dev
- **Pro 专属**: support@easyssh.dev (4 小时响应)

## 安装问题

### 安装后无法打开应用？

**macOS "无法验证开发者":**
```bash
# 方法 1: 右键打开
右键点击应用 → 打开

# 方法 2: 命令行
xattr -dr com.apple.quarantine /Applications/EasySSH.app

# 方法 3: 系统设置
系统偏好设置 → 安全性与隐私 → 通用 → 仍要打开
```

**Windows "Windows 已保护你的电脑":**
1. 点击「更多信息」
2. 点击「仍要运行」
3. 我们正在申请代码签名证书，这将很快解决

**Linux 缺少依赖:**
```bash
# Ubuntu/Debian
sudo apt install libgtk-3-0 libwebkit2gtk-4.0-37

# Fedora
sudo dnf install gtk3 webkit2gtk3

# Arch
sudo pacman -S gtk3 webkit2gtk
```

### 如何卸载？

**macOS:**
```bash
# Homebrew
brew uninstall easyssh

# 手动
rm -rf /Applications/EasySSH.app
rm -rf ~/Library/Application\ Support/EasySSH
```

**Windows:**
```powershell
# Winget
winget uninstall EasySSH

# 设置 → 应用 → 应用和功能 → EasySSH → 卸载
```

**Linux:**
```bash
# Ubuntu/Debian
sudo apt remove easyssh

# Fedora
sudo dnf remove easyssh

# Arch
sudo pacman -R easyssh

# 清理数据
rm -rf ~/.config/easyssh
```

### 如何同时安装多个版本？

不建议在同一设备安装 Lite 和 Standard，它们使用相同的数据目录。

如需测试不同版本：
1. 使用虚拟机
2. 使用不同用户账户
3. 使用便携版（设置不同数据目录）

## 连接问题

### 连接服务器失败？

**检查清单：**

1. **网络连通性**
   ```bash
   ping <host>
   telnet <host> <port>
   ```

2. **SSH 服务状态**
   ```bash
   # 在目标服务器上
   sudo systemctl status sshd
   ```

3. **防火墙设置**
   ```bash
   # 检查端口是否开放
   nc -zv <host> 22
   ```

4. **认证方式**
   - 密码是否正确？
   - 密钥权限是否正确？（应为 600）
   - 是否支持该密钥算法？

5. **详细日志**
   ```bash
   easyssh connect <server> --verbose
   ```

### 密钥认证失败？

**权限问题：**
```bash
# 修复密钥权限
chmod 600 ~/.ssh/id_rsa
chmod 700 ~/.ssh
```

**密钥格式问题：**
```bash
# 转换密钥格式
ssh-keygen -p -m PEM -f ~/.ssh/id_rsa

# 或生成新密钥
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519
```

**Agent 问题：**
```bash
# 检查 Agent 是否运行
echo $SSH_AGENT_SOCK

# 添加密钥到 Agent
ssh-add ~/.ssh/id_rsa

# 列出已添加的密钥
ssh-add -l
```

### 连接超时？

**增加超时时间：**
```bash
# 全局配置
easyssh config set ssh.timeout 60

# 单个服务器
easyssh edit-server <id> --timeout 60
```

**检查网络：**
```bash
# 使用 mtr 诊断
mtr <host>

# 检查路由
traceroute <host>
```

## 功能问题

### Lite 版如何配置默认终端？

```bash
# 查看检测到的终端
easyssh config get terminal.detected

# 设置首选终端
easyssh config set terminal "iTerm"

# 或使用完整路径
easyssh config set terminal-path "/Applications/iTerm.app"
```

### Standard 版终端显示乱码？

**字体问题：**
```bash
# 安装 Powerline 字体
easyssh config set terminal.font-family "Meslo LG M for Powerline"

# 或使用 Nerd Fonts
easyssh config set terminal.font-family "JetBrainsMono Nerd Font"
```

**编码问题：**
```bash
# 设置终端编码
easyssh config set terminal.encoding "utf-8"

# 在远程服务器上
export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8
```

### Standard 版分屏后无法调整大小？

**重置布局：**
```
1. 右键分屏边框
2. 选择「重置布局」
3. 或按 Cmd/Ctrl + Shift + M
```

**手动调整：**
```
Cmd/Ctrl + Shift + 方向键：调整面板大小
```

### SFTP 传输速度慢？

**优化设置：**
```bash
# 增加并发连接
easyssh config set sftp.max-concurrent 10

# 启用压缩
easyssh config set sftp.compression true

# 调整缓冲区大小
easyssh config set sftp.buffer-size 65536
```

### 如何同步服务器配置？

**Lite/Standard:**
```bash
# 导出配置
easyssh export --format json --output config.json

# 在另一设备导入
easyssh import --source json --file config.json
```

**Pro:**
配置自动同步，无需手动操作。

## 安全问题

### 忘记主密码怎么办？

**Lite/Standard:**
::: danger 警告
忘记主密码意味着数据永久丢失。我们无法恢复，因为：
1. 数据使用 AES-256-GCM 加密
2. 密钥由主密码派生（Argon2id）
3. 没有后门或恢复机制
:::

**预防措施：**
- 使用密码管理器保存主密码
- 定期导出备份
- 考虑升级到 Pro 版，支持管理员重置

### 如何确保数据安全？

**Lite/Standard:**
1. 设置强主密码（12+ 字符，混合类型）
2. 启用自动备份
3. 定期更新软件
4. 使用 SSH 密钥而非密码
5. 限制物理设备访问

**Pro:**
1. 强制启用 2FA
2. 配置会话录制（敏感环境）
3. 定期审查访问日志
4. 使用 SSO 集成
5. 配置审批工作流

### 密钥存储安全吗？

**Lite/Standard:**
- 私钥存储在系统 Keychain
- 主密码派生密钥加密
- 内存中短暂存在，使用后清除

**Pro:**
- 可选 HSM 集成
- 密钥访问审计
- 定期密钥轮换

### 会记录我的密码吗？

**不会。**
- 密码仅用于连接认证
- 不会保存在日志中
- 不会发送到我们的服务器（Lite/Standard）

## 升级与迁移

### 从 Lite 升级到 Standard？

**自动迁移：**
1. 安装 Standard 版
2. 自动识别并读取 Lite 数据
3. 获得嵌入式终端等新功能

**注意事项：**
- 配置完整保留
- 无需重新设置主密码
- 终端会话历史将从头开始

### 从 Termius 迁移？

```bash
# 从 Termius 导出 CSV
# 设置 → 导出 → CSV

# 导入到 EasySSH
easyssh import --source termius --file termius-export.csv
```

### 从其他工具迁移？

支持的导入格式：
- SSH config (`~/.ssh/config`)
- Termius (CSV)
- MobaXterm
- SecureCRT
- PuTTY (Windows Registry)

```bash
# 列出支持的导入源
easyssh import --list-sources

# 导入
easyssh import --source <source> --file <path>
```

## Pro 版问题

### 如何部署 Pro 服务端？

详见 [企业部署指南](/zh/deploy/enterprise)。

快速开始：
```bash
# Docker Compose
curl -fsSL https://easyssh.dev/install-pro.sh | sh
```

### Pro 版数据同步失败？

**检查清单：**
1. 客户端能否访问服务端？
   ```bash
   curl https://easyssh.company.com/health
   ```

2. 检查网络防火墙
3. 验证 SSL 证书
4. 查看客户端日志
   ```bash
   easyssh --verbose --sync
   ```

### 如何配置 SSO？

支持 SAML 和 OIDC：

**SAML (Okta):**
```yaml
sso:
  provider: saml
  config:
    entrypoint: "https://company.okta.com/app/easyssh/sso/saml"
    cert: "/path/to/cert.pem"
```

**OIDC (Google):**
```yaml
sso:
  provider: oidc
  config:
    issuer: "https://accounts.google.com"
    client_id: "YOUR_CLIENT_ID"
    client_secret: "YOUR_CLIENT_SECRET"
```

详见 [SSO 配置指南](/zh/deploy/sso)。

### 如何查看审计日志？

```bash
# Web 管理界面
登录 → 审计 → 筛选 → 导出

# CLI
easyssh admin audit query --from "2026-01-01" --to "2026-01-31"

# API
curl https://easyssh.company.com/api/v1/audit \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"start_date": "2026-01-01", "event_type": "session.connect"}'
```

## 性能问题

### 内存占用过高？

**Lite:**
- 正常：50-100MB
- 如果过高，检查日志级别设置

**Standard:**
- 正常：150-300MB
- 减少终端回滚缓冲区大小
  ```bash
  easyssh config set terminal.scrollback 10000
  ```

**优化建议：**
```bash
# 限制同时连接数
easyssh config set ssh.max-concurrent 10

# 清理旧会话
easyssh config set session.auto-cleanup 24h
```

### 终端响应慢？

**检查：**
1. 网络延迟：`ping <host>`
2. 服务器负载：`uptime`
3. 启用压缩：`easyssh config set ssh.compression true`

**禁用 WebGL（如 GPU 问题）：**
```bash
easyssh config set terminal.webgl false
```

### 启动速度慢？

**常见原因：**
1. 大量服务器配置 → 使用分组和搜索
2. 慢速网络 → 禁用自动检查更新
   ```bash
   easyssh config set updates.check-on-startup false
   ```
3. 旧硬件 → 考虑使用 Lite 版

## 贡献与开发

### 如何贡献代码？

1. Fork 仓库
2. 创建功能分支
3. 提交 PR

详见 [贡献指南](/zh/develop/contributing)。

### 如何报告 Bug？

```
1. 检查是否已存在相关 Issue
2. 提供详细信息：
   - 版本号
   - 操作系统
   - 复现步骤
   - 错误日志
   - 截图（如适用）
```

### 如何请求新功能？

在 [GitHub Discussions](https://github.com/anixops/easyssh/discussions) 发起讨论。

## 其他

### 快捷键冲突？

**查看并修改快捷键：**
```bash
# 列出当前快捷键
easyssh shortcuts list

# 修改冲突的快捷键
easyssh shortcuts set terminal.new-tab "Cmd+Shift+T"
```

### 如何禁用更新检查？

```bash
easyssh config set updates.enabled false
```

### 日志文件在哪里？

```
macOS: ~/Library/Application Support/EasySSH/logs/
Windows: %APPDATA%\EasySSH\logs\
Linux: ~/.config/easyssh/logs/
```

### 如何完全重置应用？

::: danger 警告
这将删除所有数据！
:::

```bash
# 停止应用

# 删除数据目录
rm -rf ~/Library/Application\ Support/EasySSH  # macOS
rm -rf %APPDATA%\EasySSH                         # Windows
rm -rf ~/.config/easyssh                       # Linux

# 重新启动应用
```

## 没有找到答案？

- 搜索文档：https://docs.easyssh.dev
- 查看 GitHub Issues：https://github.com/anixops/easyssh/issues
- 加入 Discord 社区：https://discord.gg/easyssh
- 发送邮件：support@easyssh.dev
