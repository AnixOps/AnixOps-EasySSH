# docs.rs 配置

> EasySSH 核心库的 docs.rs 文档生成配置

## Cargo.toml 配置

在 `core/Cargo.toml` 中添加以下配置以优化 docs.rs 构建：

```toml
[package.metadata.docs.rs]
# 构建所有特性以获得完整文档
all-features = true

# 需要的外部依赖
[package.metadata.docs.rs.dependencies]
# 文档生成时需要的系统库
```

## 当前配置

```toml
# core/Cargo.toml 中已包含:
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

## 文档特性标注

为了正确显示特性 gated 的功能，在代码中使用 `#[cfg(docsrs)]`：

```rust
// 示例：在 lib.rs 中
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "pro")]
#[cfg_attr(docsrs, doc(cfg(feature = "pro")))]
pub mod pro;
```

## 文档结构

在 docs.rs 上，文档将按以下结构组织：

```
easyssh_core
├── Re-exports
├── Modules
│   ├── ssh (SSH 连接管理)
│   ├── crypto (加密系统)
│   ├── db (数据库)
│   ├── sftp (SFTP 文件传输)
│   ├── terminal (终端模拟器)
│   ├── team (团队管理)
│   ├── audit (审计日志)
│   ├── vault (密码保险箱)
│   ├── kubernetes (K8s 管理)
│   ├── docker (Docker 管理)
│   └── workflow (工作流自动化)
├── Structs
│   ├── AppState
│   ├── SshSessionManager
│   ├── CryptoState
│   ├── LiteError
│   └── ...
├── Enums
│   ├── ConnectionHealth
│   ├── Edition
│   └── ...
├── Traits
│   └── ...
└── Functions
    ├── init_database
    ├── ssh_connect
    ├── ssh_execute
    └── ...
```

## 本地文档生成

```bash
# 生成文档
cd core
cargo doc --no-deps --all-features

# 打开文档
open target/doc/easyssh_core/index.html

# 生成并检查链接
cargo doc --no-deps --all-features -- --check
```

## 文档覆盖率

使用 `cargo doc-coverage` 检查文档覆盖率：

```bash
cargo install cargo-doc-coverage
cargo doc-coverage --all-features
```

## 文档发布检查清单

- [ ] 所有公共 API 都有文档注释
- [ ] 所有示例代码都可以编译
- [ ] 文档链接有效
- [ ] 特性标注正确
- [ ] 文档测试通过 (`cargo test --doc`)
- [ ] 没有警告 (`#![deny(rustdoc::warnings)]`)

## 链接

- [docs.rs 首页](https://docs.rs/easyssh-core)
- [docs.rs 构建日志](https://docs.rs/crate/easyssh-core/builds)
- [docs.rs 文档](https://docs.rs/about)

## 徽章

```markdown
[![Docs.rs](https://docs.rs/easyssh-core/badge.svg)](https://docs.rs/easyssh-core)
```
