# EasySSH Documentation

## Secure, Modern SSH Client for Developers and Teams

EasySSH is a product line of SSH clients for developers and teams, offering three editions to meet different needs:

<br>

<div class="grid-container">
  <div class="grid-item">
    <h3>🔐 Lite</h3>
    <p>SSH Configuration Vault</p>
    <ul>
      <li>Securely store connection configs</li>
      <li>One-click native terminal launch</li>
      <li>Local encryption, privacy-first</li>
    </ul>
    <a href="/en/guide/features/lite">Learn more →</a>
  </div>
  <div class="grid-item">
    <h3>⚡ Standard</h3>
    <p>Full-Featured Personal Workstation</p>
    <ul>
      <li>Embedded WebGL terminal</li>
      <li>Multi-tabs and split panes</li>
      <li>SFTP file transfer</li>
    </ul>
    <a href="/en/guide/features/standard">Learn more →</a>
  </div>
  <div class="grid-item">
    <h3>👥 Pro</h3>
    <p>Team Collaboration Platform</p>
    <ul>
      <li>Team management and RBAC</li>
      <li>Audit logs and compliance</li>
      <li>SSO integration</li>
    </ul>
    <a href="/en/guide/features/pro">Learn more →</a>
  </div>
</div>

<br>

## Quick Start

::: tip Choose Your Edition
Select based on your use case:
- **Individual users**: Lite or Standard
- **Team users**: Standard or Pro
- **Enterprise users**: Pro
:::

### 1. Download and Install

```bash
# macOS
brew install easyssh

# Windows (Winget)
winget install EasySSH

# Linux
sudo apt install easyssh
```

### 2. Initialize

```bash
# First launch will auto-initialize database
# Set master password (Lite edition)
easyssh --init
```

### 3. Add Your First Server

```bash
# Add via CLI
easyssh add-server --name "Production" --host "prod.example.com" --user "admin"
```

## Feature Comparison

| Feature | Lite | Standard | Pro |
|---------|:----:|:--------:|:---:|
| Password/Key Auth | ✅ | ✅ | ✅ |
| SSH Agent | ✅ | ✅ | ✅ |
| Native Terminal | ✅ | - | - |
| Embedded Terminal | - | ✅ | ✅ |
| Multi-Tabs | - | ✅ | ✅ |
| Split Panes | - | ✅ | ✅ |
| SFTP | - | ✅ | ✅ |
| Server Groups | ✅ | ✅ | ✅ |
| Team Collaboration | - | - | ✅ |
| RBAC | - | - | ✅ |
| Audit Logs | - | - | ✅ |
| SSO | - | - | ✅ |

## Documentation Navigation

<div class="doc-grid">
  <a href="/en/guide/" class="doc-card">
    <h4>📖 User Guide</h4>
    <p>Installation, quick start, feature details</p>
  </a>
  <a href="/en/api/" class="doc-card">
    <h4>🔧 API Docs</h4>
    <p>Core library API reference, FFI interface</p>
  </a>
  <a href="/en/develop/" class="doc-card">
    <h4>💻 Developer Docs</h4>
    <p>Architecture, contributing, building guide</p>
  </a>
  <a href="/en/deploy/" class="doc-card">
    <h4>🚀 Deployment</h4>
    <p>Enterprise deployment, security, scaling</p>
  </a>
  <a href="/en/faq/" class="doc-card">
    <h4>❓ FAQ</h4>
    <p>Frequently asked questions</p>
  </a>
  <a href="/en/troubleshooting/" class="doc-card">
    <h4>🔍 Troubleshooting</h4>
    <p>Error codes, solutions</p>
  </a>
</div>

## System Requirements

### Lite
- **macOS**: 10.15+ (Intel/Apple Silicon)
- **Windows**: Windows 10 1809+
- **Linux**: Ubuntu 20.04+, Fedora 35+

### Standard
- **macOS**: 11.0+ (Intel/Apple Silicon)
- **Windows**: Windows 10 2004+
- **Linux**: Ubuntu 22.04+, Fedora 36+

### Pro
- **Server**: Docker 20.10+ / Kubernetes 1.24+
- **Client**: Same as Standard

## Get Help

- [GitHub Issues](https://github.com/anixops/easyssh/issues)
- [Discord Community](https://discord.gg/easyssh)
- [Email Support](mailto:support@easyssh.dev)

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
