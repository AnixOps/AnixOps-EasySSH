# Frequently Asked Questions / 常见问题解答

> Quick answers to common questions about EasySSH
> EasySSH 常见问题快速解答

**[English](#english) | [中文](#中文)**

---

# English

## General Questions

### What is EasySSH?

EasySSH is a modern SSH client product line with three editions:
- **Lite**: SSH configuration vault with secure storage
- **Standard**: Full-featured client with embedded terminal and SFTP
- **Pro**: Team collaboration platform with RBAC and audit logs

### Is EasySSH free?

- **Lite**: Free and open source
- **Standard**: Free for personal use, commercial license available
- **Pro**: Enterprise license required

### Which edition should I choose?

| Use Case | Recommended Edition |
|----------|-------------------|
| Personal, privacy-focused | Lite |
| Managing multiple servers | Standard |
| Team/Enterprise use | Pro |

---

## Installation

### How do I install EasySSH?

**macOS:**
```bash
brew install easyssh  # Standard edition
brew install easyssh-lite  # Lite edition
```

**Windows:**
```powershell
winget install EasySSH
```

**Linux:**
```bash
curl -fsSL https://easyssh.dev/install.sh | sh
```

### What are the system requirements?

**Lite:**
- macOS 10.15+, Windows 10 1809+, Ubuntu 20.04+
- 4 GB RAM
- 100 MB disk space

**Standard:**
- macOS 11.0+, Windows 10 2004+, Ubuntu 22.04+
- 8 GB RAM
- 500 MB disk space

**Pro:**
- Same as Standard for client
- Server requires Docker 20.10+ or Kubernetes 1.24+

---

## Features

### Does Lite support embedded terminal?

No. Lite intentionally uses native terminal integration for maximum compatibility and minimal resource usage. Standard and Pro include embedded terminals.

### Can I transfer files with EasySSH?

- **Lite**: No, use external SFTP clients
- **Standard/Pro**: Built-in SFTP support

### Does EasySSH support SSH keys?

Yes, all editions support:
- Password authentication
- SSH key authentication (RSA, Ed25519, ECDSA)
- SSH Agent forwarding

### Can I import my existing SSH config?

Yes, all editions support importing from:
- `~/.ssh/config`
- Termius export
- MobaXterm export
- Custom CSV/JSON formats

---

## Security

### Is my data encrypted?

**Lite/Standard:**
- Local database encrypted with AES-256-GCM
- Master password using Argon2id
- Keychain integration for credentials

**Pro:**
- End-to-end encryption for sync
- Enterprise key management
- Audit logging

### What encryption does EasySSH use?

- **Algorithm**: AES-256-GCM
- **Key Derivation**: Argon2id (memory-hard)
- **Key Storage**: OS-native keychain

### Can I recover my data if I forget the master password?

**Lite/Standard:** No. The encryption is designed to be unrecoverable without the password. Please store your password securely.

**Pro:** Administrators can reset passwords through the admin console.

### Does EasySSH collect telemetry?

Optional, anonymous telemetry can be enabled to help improve the product. No sensitive data is collected. See [Telemetry Analytics](../telemetry-analytics.md) for details.

---

## Troubleshooting

### Connection fails with "Authentication failed"

1. Verify username and password
2. Check SSH key permissions (should be 600)
3. Ensure key format is correct (PEM or OpenSSH)
4. Test with command-line SSH first

### Cannot connect to server

1. Check network connectivity: `ping <host>`
2. Verify SSH port is open: `nc -zv <host> 22`
3. Check firewall settings
4. Verify server's SSH service is running

### Database is locked

```bash
# Reset database lock
rm ~/.local/share/easyssh/*.db-journal
```

### App crashes on startup

1. Check logs: `RUST_LOG=debug easyssh 2>&1 | tee log.txt`
2. Reset configuration: `mv ~/.config/easyssh ~/.config/easyssh.bak`
3. Reinstall the application

---

## Development

### How can I contribute?

See [Contributing Guide](../../CONTRIBUTING.md) for:
- Code contributions
- Bug reports
- Feature requests
- Documentation improvements

### Where is the source code?

https://github.com/anixops/easyssh

### What license is EasySSH under?

MIT License - See [LICENSE](../../LICENSE) file

---

# 中文

## 一般问题

### EasySSH 是什么？

EasySSH 是一个现代 SSH 客户端产品线，包含三个版本：
- **Lite**: SSH 配置保险箱，安全存储配置
- **Standard**: 全功能客户端，内置终端和 SFTP
- **Pro**: 团队协作平台，包含 RBAC 和审计日志

### EasySSH 免费吗？

- **Lite**: 免费开源
- **Standard**: 个人使用免费，商业用途需购买授权
- **Pro**: 需要企业授权

### 我应该选择哪个版本？

| 使用场景 | 推荐版本 |
|----------|----------|
| 个人使用，注重隐私 | Lite |
| 管理多台服务器 | Standard |
| 团队/企业使用 | Pro |

---

## 安装

### 如何安装 EasySSH？

**macOS:**
```bash
brew install easyssh  # Standard 版本
brew install easyssh-lite  # Lite 版本
```

**Windows:**
```powershell
winget install EasySSH
```

**Linux:**
```bash
curl -fsSL https://easyssh.dev/install.sh | sh
```

### 系统要求是什么？

**Lite:**
- macOS 10.15+, Windows 10 1809+, Ubuntu 20.04+
- 4 GB 内存
- 100 MB 磁盘空间

**Standard:**
- macOS 11.0+, Windows 10 2004+, Ubuntu 22.04+
- 8 GB 内存
- 500 MB 磁盘空间

**Pro:**
- 客户端与 Standard 相同
- 服务端需要 Docker 20.10+ 或 Kubernetes 1.24+

---

## 功能

### Lite 支持内置终端吗？

不支持。Lite 故意使用原生终端集成，以获得最大的兼容性和最小的资源占用。Standard 和 Pro 包含内置终端。

### EasySSH 支持文件传输吗？

- **Lite**: 不支持，请使用外部 SFTP 客户端
- **Standard/Pro**: 内置 SFTP 支持

### EasySSH 支持 SSH 密钥吗？

是的，所有版本都支持：
- 密码认证
- SSH 密钥认证（RSA、Ed25519、ECDSA）
- SSH Agent 转发

### 我可以导入现有的 SSH 配置吗？

是的，所有版本都支持从以下来源导入：
- `~/.ssh/config`
- Termius 导出
- MobaXterm 导出
- 自定义 CSV/JSON 格式

---

## 安全

### 我的数据是加密的吗？

**Lite/Standard:**
- 本地数据库使用 AES-256-GCM 加密
- 主密码使用 Argon2id
- 凭据集成系统钥匙串

**Pro:**
- 同步使用端到端加密
- 企业密钥管理
- 审计日志

### EasySSH 使用什么加密？

- **算法**: AES-256-GCM
- **密钥派生**: Argon2id（内存困难）
- **密钥存储**: 操作系统原生钥匙串

### 忘记主密码后能恢复数据吗？

**Lite/Standard:** 不能。加密设计为没有密码就无法恢复。请安全存储您的密码。

**Pro:** 管理员可以通过管理控制台重置密码。

### EasySSH 收集遥测数据吗？

可选的匿名遥测可以帮助改进产品。不收集敏感数据。详见 [遥测分析](../telemetry-analytics.md)。

---

## 故障排查

### 连接失败显示"认证失败"

1. 验证用户名和密码
2. 检查 SSH 密钥权限（应为 600）
3. 确保密钥格式正确（PEM 或 OpenSSH）
4. 先用命令行 SSH 测试

### 无法连接到服务器

1. 检查网络连通性: `ping <host>`
2. 验证 SSH 端口是否开放: `nc -zv <host> 22`
3. 检查防火墙设置
4. 验证服务器的 SSH 服务是否运行

### 数据库被锁定

```bash
# 重置数据库锁
rm ~/.local/share/easyssh/*.db-journal
```

### 应用启动时崩溃

1. 检查日志: `RUST_LOG=debug easyssh 2>&1 | tee log.txt`
2. 重置配置: `mv ~/.config/easyssh ~/.config/easyssh.bak`
3. 重新安装应用

---

## 开发

### 如何贡献代码？

详见 [贡献指南](../../CONTRIBUTING.md)：
- 代码贡献
- 错误报告
- 功能请求
- 文档改进

### 源代码在哪里？

https://github.com/anixops/easyssh

### EasySSH 使用什么许可？

MIT 许可证 - 详见 [LICENSE](../../LICENSE) 文件

---

*Last Updated: 2026-04-02 / 最后更新: 2026-04-02*
