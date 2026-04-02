# EasySSH 依赖清理与优化报告

**生成日期**: 2026-04-01
**分析范围**: 全工作区 (1352+ crate 依赖)

---

## 1. 安全漏洞分析

### 严重漏洞 (需立即修复)

| 漏洞ID | 组件 | 当前版本 | 风险等级 | 修复方案 |
|--------|------|----------|----------|----------|
| RUSTSEC-2024-0421 | idna | 0.4.0 | 中 | 升级到 >=1.0.0 |
| RUSTSEC-2023-0071 | rsa | 0.9.10 | 中 | **无可用修复** - Marvin Attack |
| RUSTSEC-2024-0363 | sqlx | 0.8.0 | 高 | 升级到 >=0.8.1 |
| RUSTSEC-2026-0008 | git2 | 0.18.3 | 中 | 升级到 >=0.20.0 |
| RUSTSEC-2026-0002 | lru | 0.12.5 | 中 | 升级到 >=0.13.0 |
| RUSTSEC-2022-0008 | windows | 0.7.0 | 中 | 升级到 >=0.48.0 |
| RUSTSEC-2024-0429 | glib | 0.18.5/0.19.9 | 中 | 监控上游修复 |

### 漏洞详细分析

#### 1. rsa (RUSTSEC-2023-0071) - 关键风险
- **影响**: 潜在的时序攻击密钥恢复 (Marvin Attack)
- **依赖路径**:
  - sqlx-mysql 0.8.0 -> rsa 0.9.10
  - openidconnect 3.5.0 -> rsa 0.9.10
- **状态**: 上游无修复版本
- **建议**:
  - 考虑禁用 MySQL 支持或等待上游修复
  - 评估切换到 `rustls` + `ring` 替代方案

#### 2. sqlx (RUSTSEC-2024-0363) - 高优先级
- **影响**: 二进制协议误解导致内存安全问题
- **修复**: 升级 sqlx 到 0.8.1+ (注意: 需协调 rusqlite 版本)

#### 3. idna (RUSTSEC-2024-0421)
- **影响**: Punycode 标签解码问题
- **路径**: validator 0.16.1 -> idna 0.4.0
- **修复**: 升级 validator 到 0.18.0+

---

## 2. 依赖版本冲突

### 关键冲突

```
libsqlite3-sys 版本冲突:
├── rusqlite v0.31.0 -> libsqlite3-sys v0.28.0
└── sqlx-sqlite v0.8.x -> libsqlite3-sys v0.30.1
```

**影响**: 构建失败 - 无法同时链接两个版本的 sqlite3

**解决方案**:
1. 升级 rusqlite 到 0.34.0+ (使用 libsqlite3-sys 0.30+)
2. 或在 Cargo.toml 中统一 sqlx 和 rusqlite 版本

### 重复依赖分析

| 组件 | 版本 | 重复版本 | 优化建议 |
|------|------|----------|----------|
| ahash | 0.8.12 | 0.7.8 | 移除旧版本 |
| base64 | 0.22.1 | 0.13.1, 0.21.7 | 统一使用 0.22.1 |
| bindgen | 0.72.1 | 0.69.5 | 统一版本 |
| bitflags | 2.11.0 | 1.3.2 | 移除旧版本 |
| parking_lot | 0.12 | 0.11 | 统一版本 |
| reqwest | 0.12 | 0.11 | 统一使用 0.12 |
| redis | 0.25 | 0.24 | 升级 pro-server |
| tokio-tungstenite | 0.24 | 0.21 | 统一版本 |
| futures-lite | 2.x | 1.x | 移除旧版本 |

---

## 3. 已弃用/无人维护的依赖

### 29个已弃用 crate 警告

| 组件 | 版本 | 弃用日期 | 替代方案 |
|------|------|----------|----------|
| atk | 0.18.2 | 2024-03 | GTK4 (gtk4-rs) |
| atk-sys | 0.18.2 | 2024-03 | GTK4 |
| gdk | 0.18.2 | 2024-03 | gdk4 |
| gdk-sys | 0.18.2 | 2024-03 | gdk4-sys |
| gdkx11 | 0.18.2 | 2024-03 | gdk4-x11 |
| gdkx11-sys | 0.18.2 | 2024-03 | gdk4-x11-sys |
| gtk | 0.18.2 | 2024-03 | gtk4 |
| gtk-sys | 0.18.2 | 2024-03 | gtk4-sys |
| gtk3-macros | 0.18.2 | 2024-03 | gtk4-macros |
| gdkwayland-sys | 0.18.2 | 2024-03 | gdk4-wayland-sys |
| derivative | 2.2.0 | 2024-06 | educe 或手动实现 |
| fxhash | 0.2.1 | 2025-09 | rustc-hash 或 ahash |
| instant | 0.1.13 | 2024-09 | web-time (WASM) 或 std::time |
| paste | 1.0.15 | 2024-10 | 内联或 const_format |
| proc-macro-error | 1.0.4 | 2024-09 | syn 2.0 错误处理 |
| rusoto_core | 0.48.0 | 2022-04 | aws-sdk-* (官方) |
| rusoto_s3 | 0.48.0 | 2022-04 | aws-sdk-s3 |
| rustls-pemfile | 1.0.4 | 2025-11 | rustls-pemfile 2.x |
| serial | 0.4.0 | 2017-07 | tokio-serial |
| yaml-rust | 0.4.5 | 2024-03 | yaml-rust2 或 serde_yaml |
| unic-char-* | 0.9.0 | 2025-10 | unicode-ident 替代 |

---

## 4. 待清理依赖分析

### 疑似未使用的依赖

#### easyssh-core
| 依赖 | 用途分析 | 建议 |
|------|----------|------|
| rusoto_core | 被 aws-sdk-* 替代 | 移除 |
| rusoto_s3 | 被 aws-sdk-s3 替代 | 移除 |
| windows-sys 0.59 | 与 windows 0.59 重复 | 合并 |

#### easyssh-winui
| 依赖 | 用途分析 | 建议 |
|------|----------|------|
| windows-webview2 | 被 webview2-com 替代 | 移除 |
| windows 0.56 | 升级到 0.59 与 core 一致 | 升级 |
| wry 0.46 | 存在 0.51 重复 | 统一版本 |
| parking_lot 0.12 | 已包含在标准库中 | 评估移除 |

#### pro-server
| 依赖 | 用途分析 | 建议 |
|------|----------|------|
| actix-web | 与 axum 重复 | 统一使用 axum |
| reqwest 0.11 | 升级到 0.12 | 升级 |
| redis 0.24 | 升级到 0.25 | 升级 |
| validator 0.16 | 升级到 0.18 | 升级 (修复 idna) |

---

## 5. 许可证合规性分析

### 依赖许可证分布 (估算)

| 许可证 | 数量 | 状态 |
|--------|------|------|
| MIT | ~850 | 兼容 |
| Apache-2.0 | ~280 | 兼容 |
| BSD-3-Clause | ~45 | 兼容 |
| ISC | ~15 | 兼容 |
| MPL-2.0 | ~12 | 兼容 |
| OpenSSL | ~8 | 兼容 |
| Unicode-3.0 | ~5 | 兼容 |

### 许可证冲突检查
- **无 GPL/AGPL 依赖** - 与 MIT 项目兼容
- **无 LGPL 依赖** - 无需动态链接考虑

### 需注意的依赖
| 依赖 | 许可证 | 备注 |
|------|--------|------|
| ring | ISC + OpenSSL | 多许可 |
| encoding_rs | MIT | 包含 Unicode 数据 |
| webpki-roots | MPL-2.0 | 证书存储 |

---

## 6. 优化建议与行动计划

### 阶段 1: 安全修复 (立即)

1. **修复 sqlx 漏洞**
   ```toml
   [dependencies]
   sqlx = { version = "0.8.3", ... }  # 升级到 0.8.1+
   rusqlite = { version = "0.34.0", ... }  # 升级到 0.34.0+
   ```

2. **修复 idna 漏洞**
   ```toml
   validator = "0.18"  # 升级到 0.18+
   ```

3. **修复 lru 漏洞**
   ```toml
   lru = "0.13"  # 升级到 0.13+
   ```

4. **移除 windows 0.7.0**
   - 移除 windows-webview2 或升级

### 阶段 2: 版本统一 (本周)

1. **统一 base64**: 全部使用 0.22.1
2. **统一 reqwest**: 全部使用 0.12
3. **统一 redis**: 升级到 0.25
4. **统一 bitflags**: 使用 2.x
5. **移除 ahash 0.7.x**: 迁移到 0.8.x

### 阶段 3: 移除弃用依赖 (本月)

1. **移除 Rusoto**: 完成迁移到 aws-sdk-*
2. **移除 yaml-rust**: 使用 yaml-rust2
3. **升级 GTK3 依赖**: 迁移到 GTK4 (已在 easyssh-gtk4 中使用)
4. **移除 instant**: 使用标准库或 web-time

### 阶段 4: 清理未使用依赖 (本月)

1. 运行 `cargo machete` 识别未使用依赖
2. 审核可选特性的依赖树
3. 移除 fake-winui-app-sdk (如不再需要)

---

## 7. 依赖优化脚本

### 推荐的 Cargo.toml 修改

```toml
# workspace/Cargo.toml 添加统一版本
[workspace.dependencies]
# 统一版本
base64 = "0.22"
reqwest = { version = "0.12", features = ["json"] }
lru = "0.13"
validator = "0.18"
redis = "0.25"
bindgen = "0.72"
bitflags = "2.8"
```

### 清理检查命令

```bash
# 1. 检查未使用依赖
cargo install cargo-machete
cargo machete

# 2. 检查重复依赖
cargo tree --duplicates

# 3. 检查更新
cargo install cargo-outdated
cargo outdated -w

# 4. 安全审计
cargo audit
```

---

## 8. 优化收益预估

| 指标 | 当前 | 优化后预估 | 收益 |
|------|------|------------|------|
| 总依赖数 | 1352 | ~1100 | -18% |
| 重复依赖 | ~45 | <10 | -78% |
| 安全漏洞 | 7+29 | 0+5 | 严重降低 |
| 编译时间 | ~8min | ~6min | -25% |
| 二进制大小 | ~45MB | ~38MB | -15% |

---

## 附录: 详细依赖清单

见 `Cargo.lock` 完整分析 (1352 crates)

---

**报告生成**: Claude Code - Dependency Optimization
**下次审查**: 建议每月运行 `cargo audit` 和 `cargo outdated`
