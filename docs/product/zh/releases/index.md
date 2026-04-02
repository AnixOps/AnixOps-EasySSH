# 版本历史与更新日志

## 版本号说明

EasySSH 使用 [语义化版本](https://semver.org/lang/zh-CN/)：

```
版本格式: 主版本号.次版本号.修订号
示例: 1.2.3

- 主版本号: 不兼容的 API 修改
- 次版本号: 向下兼容的功能新增
- 修订号: 向下兼容的问题修正
```

## 当前版本

### v1.3.0 (2026-03-31)

**Pro 版正式发布！**

#### 新增功能

- **团队管理**: 完整的团队、部门、成员管理功能
- **RBAC 权限**: 基于角色的细粒度访问控制
- **审计日志**: 全量操作审计，支持导出和分析
- **SSO 集成**: 支持 SAML 和 OIDC 单点登录
- **Pro 服务端**: Docker 和 Kubernetes 部署支持

#### 改进

- 优化了 Standard 版的终端性能
- 改进了 SFTP 大文件传输稳定性
- 新增 5 种终端主题
- 改进搜索算法，速度提升 40%

#### 修复

- 修复了 Windows 下路径包含空格的问题
- 修复了 macOS 下终端唤起的偶发失败
- 修复了分屏布局保存后的恢复问题

#### 安全

- 更新了 OpenSSL 到 3.2.1
- 修复了潜在的路径遍历漏洞
- 增强了密码策略检查

**下载:**
- [Lite (免费)](https://easyssh.dev/download/lite/1.3.0)
- [Standard](https://easyssh.dev/download/standard/1.3.0)
- [Pro 服务端](https://easyssh.dev/download/pro/1.3.0)

---

## 历史版本

### v1.2.0 (2026-02-15)

**Standard 版重大更新**

#### 新增功能

- **WebGL 终端**: GPU 加速的终端渲染
- **分屏系统**: 支持 2x2、三栏等布局
- **SFTP 客户端**: 内置文件传输，支持拖拽
- **布局保存**: 保存和恢复常用分屏布局
- **命令历史**: 跨会话的命令记录和搜索

#### 改进

- SSH 连接池优化，连接速度提升 60%
- 终端滚动性能提升 3 倍
- 改进密钥管理界面
- 新增批量编辑功能

#### 修复

- 修复了长输出时的内存泄漏
- 修复了密钥认证在特定场景下的失败
- 修复了主题切换时的闪烁问题

### v1.1.0 (2026-01-10)

**Lite 版增强**

#### 新增功能

- **自动备份**: 每日自动备份配置
- **高级搜索**: 支持标签、分组、状态筛选
- **导入/导出**: 支持更多格式（Termius、MobaXterm）
- **快捷键**: 完整的快捷键支持

#### 改进

- 启动速度提升 50%
- 搜索响应时间优化
- 改进错误提示信息

### v1.0.0 (2025-12-01)

**首个正式版本**

- Lite 版完整功能
- 基础 SSH 连接管理
- Keychain 集成
- 分组管理
- 搜索功能

---

## 路线图

### 2026 Q2 (v1.4.0)

- [ ] 移动端 App (iOS/Android)
- [ ] Pro 版高级分析仪表板
- [ ] 插件系统 Beta
- [ ] 改进的同步算法

### 2026 Q3 (v1.5.0)

- [ ] 终端多路复用 (tmux/screen 集成)
- [ ] AI 辅助命令建议
- [ ] 改进的 SFTP 同步浏览
- [ ] 更多 SSO 提供商支持

### 2026 Q4 (v1.6.0)

- [ ] 脚本录制和回放
- [ ] 高级监控面板
- [ ] 多因素认证改进
- [ ] 企业级 SLA 支持

### 2027 Q1 (v2.0.0)

- [ ] 全新的 UI/UX 设计
- [ ] WebAssembly 终端核心
- [ ] 云端配置同步 (Standard)
- [ ] API 市场

---

## 迁移指南

### 从 v1.0 升级到 v1.1+

**自动迁移：**
1. 安装新版本
2. 数据自动保留
3. 无需额外操作

**手动备份（可选）：**
```bash
easyssh export --format json --output pre-upgrade-backup.json
```

### 从 v1.1 升级到 v1.2+ (Standard)

**注意：**
- Lite 版数据可直接迁移到 Standard
- 升级前建议导出备份

**步骤：**
1. 导出 Lite 数据
2. 安装 Standard
3. Standard 自动识别并导入 Lite 数据
4. 开始使用新功能

### 从其他工具迁移

详见 [导入配置指南](/zh/guide/import-config)。

---

## 兼容性说明

### 向后兼容

- v1.x 数据文件完全兼容
- API 保持向后兼容
- 配置自动迁移

### 破坏性变更

| 版本 | 变更 | 影响 | 迁移方案 |
|------|------|------|----------|
| v2.0 | 配置文件格式变更 | 配置需手动转换 | 提供转换工具 |

---

## 安全公告

### 安全更新

| 日期 | 版本 | 漏洞 | 严重程度 | CVE |
|------|------|------|----------|-----|
| 2026-03-15 | 1.2.2 | 路径遍历 | 中等 | CVE-2026-XXXX |
| 2026-02-01 | 1.1.1 | 密钥泄露 | 高 | CVE-2026-YYYY |

**建议：** 始终保持更新到最新版本。

---

## 变更日志详情

### v1.3.0 (2026-03-31)

```
[Added]
- Pro 版团队管理功能
- RBAC 权限控制系统
- 全量审计日志
- SAML/OIDC SSO 集成
- Pro 服务端 Docker/K8s 部署
- 会话录制功能
- 审批工作流
- 共享 Snippets
- SCIM 用户同步

[Changed]
- 改进终端渲染性能
- 优化 SFTP 传输算法
- 改进搜索索引
- 更新依赖库版本

[Fixed]
- Windows 路径空格问题 (#456)
- macOS 终端唤起失败 (#432)
- 分屏布局恢复 (#398)
- 内存泄漏修复 (#412)
- SSH 密钥权限检测 (#445)

[Security]
- 更新 OpenSSL 到 3.2.1
- 修复路径遍历漏洞
- 增强密码策略
- 改进密钥存储安全
```

### v1.2.0 (2026-02-15)

```
[Added]
- WebGL 终端渲染
- 分屏系统 (2x2, 三栏, 自定义)
- SFTP 客户端
- 布局保存和恢复
- 命令历史记录
- 监控小组件

[Changed]
- SSH 连接池优化
- 终端性能提升
- UI 响应优化

[Fixed]
- 长输出内存泄漏
- 密钥认证失败
- 主题闪烁
```

---

## 获取更新通知

- GitHub Watch: [anixops/easyssh](https://github.com/anixops/easyssh)
- Twitter: [@easyssh](https://twitter.com/easyssh)
- 邮件订阅: [newsletter](https://easyssh.dev/newsletter)
- RSS: [releases.xml](https://easyssh.dev/releases.xml)

## 反馈与建议

- GitHub Discussions: [github.com/anixops/easyssh/discussions](https://github.com/anixops/easyssh/discussions)
- Discord: [discord.gg/easyssh](https://discord.gg/easyssh)
- 邮件: feedback@easyssh.dev
