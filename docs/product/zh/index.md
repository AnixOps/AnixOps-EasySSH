# EasySSH 文档

## 为开发者和团队提供安全、现代的 SSH 客户端

EasySSH 是一款面向开发者和团队的 SSH 客户端产品线，提供三个版本满足不同需求：

<br>

<div class="grid-container">
  <div class="grid-item">
    <h3>🔐 Lite</h3>
    <p>SSH 配置保险箱</p>
    <ul>
      <li>安全存储连接配置</li>
      <li>一键唤起原生终端</li>
      <li>本地加密，隐私优先</li>
    </ul>
    <a href="/zh/guide/features/lite">了解更多 →</a>
  </div>
  <div class="grid-item">
    <h3>⚡ Standard</h3>
    <p>全功能个人工作站</p>
    <ul>
      <li>嵌入式 WebGL 终端</li>
      <li>多标签和分屏功能</li>
      <li>SFTP 文件传输</li>
    </ul>
    <a href="/zh/guide/features/standard">了解更多 →</a>
  </div>
  <div class="grid-item">
    <h3>👥 Pro</h3>
    <p>团队协作平台</p>
    <ul>
      <li>团队管理和 RBAC</li>
      <li>审计日志和合规</li>
      <li>SSO 集成</li>
    </ul>
    <a href="/zh/guide/features/pro">了解更多 →</a>
  </div>
</div>

<br>

## 快速开始

::: tip 选择您的版本
根据使用场景选择：
- **个人用户**: Lite 或 Standard
- **团队用户**: Standard 或 Pro
- **企业用户**: Pro
:::

### 1. 下载和安装

```bash
# macOS
brew install easyssh

# Windows (Winget)
winget install EasySSH

# Linux
sudo apt install easyssh
```

### 2. 初始化

```bash
# 首次启动将自动初始化数据库
# 设置主密码 (Lite 版本)
easyssh --init
```

### 3. 添加第一台服务器

```bash
# 通过 CLI 添加
easyssh add-server --name "Production" --host "prod.example.com" --user "admin"
```

## 功能对比

| 功能 | Lite | Standard | Pro |
|------|:----:|:--------:|:---:|
| 密码/密钥认证 | ✅ | ✅ | ✅ |
| SSH Agent | ✅ | ✅ | ✅ |
| 原生终端 | ✅ | - | - |
| 嵌入式终端 | - | ✅ | ✅ |
| 多标签 | - | ✅ | ✅ |
| 分屏 | - | ✅ | ✅ |
| SFTP | - | ✅ | ✅ |
| 服务器分组 | ✅ | ✅ | ✅ |
| 团队协作 | - | - | ✅ |
| RBAC | - | - | ✅ |
| 审计日志 | - | - | ✅ |
| SSO | - | - | ✅ |

## 文档导航

<div class="doc-grid">
  <a href="/zh/guide/" class="doc-card">
    <h4>📖 用户指南</h4>
    <p>安装、快速开始、功能详情</p>
  </a>
  <a href="/zh/api/" class="doc-card">
    <h4>🔧 API 文档</h4>
    <p>核心库 API 参考、FFI 接口</p>
  </a>
  <a href="/zh/develop/" class="doc-card">
    <h4>💻 开发者文档</h4>
    <p>架构、贡献、构建指南</p>
  </a>
  <a href="/zh/deploy/" class="doc-card">
    <h4>🚀 部署</h4>
    <p>企业部署、安全、扩展</p>
  </a>
  <a href="/zh/faq/" class="doc-card">
    <h4>❓ FAQ</h4>
    <p>常见问题</p>
  </a>
  <a href="/zh/troubleshooting/" class="doc-card">
    <h4>🔍 故障排查</h4>
    <p>错误代码、解决方案</p>
  </a>
</div>

## 系统要求

### Lite
- **macOS**: 10.15+ (Intel/Apple Silicon)
- **Windows**: Windows 10 1809+
- **Linux**: Ubuntu 20.04+, Fedora 35+

### Standard
- **macOS**: 11.0+ (Intel/Apple Silicon)
- **Windows**: Windows 10 2004+
- **Linux**: Ubuntu 22.04+, Fedora 36+

### Pro
- **服务端**: Docker 20.10+ / Kubernetes 1.24+
- **客户端**: 与 Standard 相同

## 获取帮助

- [GitHub Issues](https://github.com/anixops/easyssh/issues)
- [Discord 社区](https://discord.gg/easyssh)
- [邮件支持](mailto:support@easyssh.dev)

<style>
.grid-container {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  margin: 2rem 0;
}
.grid-item {
  padding: 1.5rem;
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  background: var(--vp-c-bg-soft);
}
.grid-item h3 {
  margin-top: 0;
}
.grid-item ul {
  margin: 1rem 0;
  padding-left: 1.2rem;
}
.doc-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  margin: 2rem 0;
}
.doc-card {
  padding: 1.5rem;
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  text-decoration: none;
  color: inherit;
  background: var(--vp-c-bg-soft);
  transition: all 0.3s;
}
.doc-card:hover {
  border-color: var(--vp-c-brand);
  transform: translateY(-2px);
}
.doc-card h4 {
  margin: 0 0 0.5rem 0;
}
.doc-card p {
  margin: 0;
  font-size: 0.9rem;
  color: var(--vp-c-text-2);
}
@media (max-width: 768px) {
  .grid-container, .doc-grid {
    grid-template-columns: 1fr;
  }
}
</style>
