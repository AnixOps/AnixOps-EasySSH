# EasySSH - SSH客户端产品线

## 产品定位

| 版本 | 定位 | 核心价值 | 目标用户 |
|------|------|----------|----------|
| **Lite** | SSH配置保险箱 | 原生终端 + 安全存储 | 注重隐私的开发者 |
| **Standard** | 全功能客户端 | 嵌入式终端 + 分屏 + 监控 | 多服务器管理者 |
| **Pro** | 团队协作平台 | 团队管理 + 审计 + SSO | IT团队/企业 |

---

## 文档结构

```
docs/
├── INDEX.md                       # 文档总索引 (本文档)
├── competitor-analysis.md         # 竞品分析(痛点/优点/警示)
├── easyssh-lite-planning.md       # Lite版本详细规划
├── easyssh-standard-planning.md   # Standard版本详细规划
├── easyssh-pro-planning.md        # Pro版本详细规划
│
├── architecture/                  # 架构设计文档
│   ├── overall-architecture.md    # 整体架构设计
│   ├── system-architecture.md     # 系统架构
│   ├── api-design.md              # API设计
│   ├── data-flow.md               # 数据流设计
│   ├── deployment.md              # 部署架构
│   └── termius-inspired-redesign.md # Termius风格重构方案
│
├── developers/                    # 开发者指南
│   ├── SETUP.md                   # 开发环境配置
│   ├── DEBUGGING.md               # 调试指南
│   ├── TESTING.md                 # 测试指南
│   ├── PROFILING.md               # 性能分析
│   └── TROUBLESHOOTING.md         # 故障排查
│
├── standards/                     # 开发标准
│   ├── code-quality.md            # 代码质量标准
│   ├── ui-ux-automation.md        # UI/UX自动化
│   └── debug-interface.md         # Debug接口
│
├── security/                      # 安全文档
│   ├── audit-report.md            # 安全审计报告
│   ├── audit-fix-report.md        # 安全修复报告
│   ├── audit-complete-2026-04-01.md # 审计完成报告
│   └── patch-guide.md             # 安全补丁指南
│
├── deployment/                    # 部署运维
├── features/                      # 功能实现文档
├── analysis/                      # 分析报告
└── dependency-analysis/           # 依赖分析

CLAUDE.md                          # 本文件 - 总览
```

---

## 快速导航

### 开发指南
- [文档索引](docs/INDEX.md) - 完整文档导航
- [竞品分析](docs/competitor-analysis.md) - SSH客户端痛点与警示
- [Lite版本规划](docs/easyssh-lite-planning.md) - Lite版完整功能规格
- [Standard版本规划](docs/easyssh-standard-planning.md) - Standard版完整功能规格
- [Pro版本规划](docs/easyssh-pro-planning.md) - Pro版完整功能规格
- [架构设计](docs/architecture/overall-architecture.md) - Monorepo结构、依赖关系
- [Termius风格重构](docs/architecture/termius-inspired-redesign.md) - 全平台工作区与版本重构方案

### 开发者文档
- [开发环境设置](docs/developers/SETUP.md) - 环境配置指南
- [调试指南](docs/developers/DEBUGGING.md) - 调试技巧与工具
- [测试指南](docs/developers/TESTING.md) - 测试策略与用例编写
- [性能分析](docs/developers/PROFILING.md) - 性能分析工具
- [故障排查](docs/developers/TROUBLESHOOTING.md) - 常见问题解决

### 开发标准
- [代码质量标准](docs/standards/code-quality.md) - Rust/TypeScript编码规范
- [UI/UX自动化](docs/standards/ui-ux-automation.md) - AI辅助设计、视觉回归测试
- [Debug接口](docs/standards/debug-interface.md) - AI Agent集成、CLI工具

### 安全文档
- [安全审计报告](docs/security/audit-report.md) - 安全审计详细报告
- [安全修复报告](docs/security/audit-fix-report.md) - 安全漏洞修复记录

---

## 核心技术栈

| 组件 | 技术选型 |
|------|----------|
| Windows UI | egui (纯Rust原生) |
| Linux UI | GTK4 (纯原生) |
| macOS UI | SwiftUI (纯原生) |
| 前端 (API Tester) | React 18 + TypeScript |
| 状态管理 | Zustand |
| 终端 (Standard) | xterm.js + xterm-addon-webgl |
| SSH | ssh2 crate / russh |
| 数据库 | SQLite |
| 加密 | Argon2id + AES-256-GCM |
| Keychain | keyring crate |
| 分屏 | golden-layout |
| CI/CD | GitHub Actions |

---

## 功能矩阵

### SSH连接

| 功能 | Lite | Standard | Pro |
|------|------|----------|-----|
| 密码/密钥认证 | ✓ | ✓ | ✓ |
| SSH Agent | ✓ | ✓ | ✓ |
| Agent转发 | - | ✓ | ✓ |
| ProxyJump | - | ✓ | ✓ |
| 自动重连 | - | ✓ | ✓ |

### 终端

| 功能 | Lite | Standard | Pro |
|------|------|----------|-----|
| 原生终端唤起 | ✓ | - | - |
| 嵌入式Web终端 | - | ✓ | ✓ |
| 多标签页 | - | ✓ | ✓ |
| 分屏 | - | ✓ | ✓ |
| WebGL加速 | - | ✓ | ✓ |

### 管理

| 功能 | Lite | Standard | Pro |
|------|------|----------|-----|
| 服务器分组 | ✓ (单层) | ✓ (嵌套) | ✓ (团队) |
| 批量操作 | - | ✓ | ✓ |
| 导入~/.ssh/config | - | ✓ | ✓ |

### 安全

| 功能 | Lite | Standard | Pro |
|------|------|----------|-----|
| Keychain集成 | ✓ | ✓ | ✓ |
| 主密码保护 | ✓ | - | - |
| 配置加密(E2EE) | - | ✓ | ✓ |
| 审计日志 | - | - | ✓ |

### 团队协作

| 功能 | Lite | Standard | Pro |
|------|------|----------|-----|
| 团队管理 | - | - | ✓ |
| RBAC权限 | - | - | ✓ |
| SSO (SAML/OIDC) | - | - | ✓ |
| 共享Snippets | - | - | ✓ |

---

## 开发优先级

```
Phase 1: Lite版本 ✅ 完成
├── ✅ 项目脚手架 + Monorepo
├── ✅ Windows egui原生UI
├── ✅ Linux GTK4原生UI
├── ✅ 加密存储 (Argon2id + AES-256-GCM)
├── ✅ Keychain集成
├── ✅ 服务器CRUD + 分组
├── ✅ 原生终端唤起
└── ✅ 搜索过滤

Phase 2: Standard版本
├── 基于Lite代码添加嵌入式终端
├── 分屏布局
├── SQLite数据库
├── 监控小组件
└── SFTP文件管理

Phase 3: Pro版本
├── Pro Backend服务
├── 团队管理 + 成员邀请
├── 审计日志
├── SSO集成
└── 协作功能
```

---

## 技术决策记录

| 日期 | 决策 | 理由 |
|------|------|------|
| 2026-03-28 | 三版本战略 | 满足不同用户群体 |
| 2026-03-28 | Lite用原生终端 | 极简+安全，专注配置管理 |
| 2026-03-28 | Monorepo结构 | 代码复用，版本协同 |
| 2026-03-28 | Argon2id + AES-256-GCM | 业界标准安全加密 |
| 2026-04-02 | 纯原生UI (egui/GTK4/SwiftUI) | 替代Tauri，追求原生性能和体验 |
| 2026-04-03 | v0.3.0-beta.1 发布 | Lite版本核心功能完成，962测试通过 |
| 2026-03-28 | xterm-addon-webgl | 多会话高频输出时GPU加速 |
| 2026-03-28 | golden-layout | 成熟的面板管理库 |
| 2026-03-28 | /proc/监控数据 | 避免命令解析脆弱性 |

---

## UI/UX自动化

- Figma → Design Tokens → Tailwind自动同步
- AI辅助组件生成 (Claude Code/GPT-4)
- Playwright视觉回归测试 (Chromatic)
- axe-core accessibility自动检测
- Lighthouse CI性能监控

---

## AI全自动编程接口 (开发专用)

> ⚠️ **仅debug模式编译，release版本完全不包含此功能**

### 编译隔离

```rust
#[cfg(debug_assertions)]
pub mod ai_programming;  // 完整AI编程能力

#[cfg(not(debug_assertions))]
pub mod ai_programming {
    pub fn enabled() -> bool { false }
}
```

### AI自我改进闭环

```
Observe → Analyze → Plan → Modify → Test → Verify → Report
   ↑                                              │
   └────────────────── loop ←────────────────────┘
```

### 核心AI能力

| 能力 | 工具 | 说明 |
|------|------|------|
| **自我诊断** | `ai_self_fix` | 分析错误 → 生成修复 → 验证 → 迭代 |
| **自我测试** | `ai_self_test` | 理解功能 → 生成测试 → 运行 → 验证覆盖率 |
| **自我重构** | `ai_self_refactor` | 分析代码 → 制定方案 → 执行 → 验证 |
| **代码理解** | `ai_explain_error` | 解释错误 → 定位原因 → 建议修复 |

### 代码操作工具 (40+)

| 类别 | 工具 |
|------|------|
| **读取** | `read_source_code`, `list_directory`, `search_in_codebase` |
| **修改** | `edit_file`, `write_file`, `create_file`, `delete_file` |
| **测试** | `run_tests`, `run_type_check`, `run_linter`, `run_build` |
| **Git** | `git_status`, `git_diff`, `git_commit` |
| **分析** | `find_related_files`, `get_code_metrics` |

### 自我修复示例

```
ai_self_fix("TypeScript error in auth.ts:42")

  Iteration 1:
    Fix: 添加 null check
    Test: FAIL (2 tests failed)
    │
  Iteration 2:
    Fix: 更新返回类型注解
    Test: FAIL (1 test failed)
    │
  Iteration 3:
    Fix: 更新测试用例
    Test: PASS ✓
    │
  Result: { success: true, iterations: 3 }
```

### 安全与审计

- **权限控制**: 写文件/删除/提交需人工批准
- **迭代限制**: 默认最多5次迭代
- **完整审计**: 所有AI操作记录在案
- **禁止自动提交**: 保护代码库安全

---

## 当前状态

**v0.3.0-beta.1 已发布** (2026-04-03)

- [x] 三版本战略定位
- [x] 功能矩阵
- [x] 技术架构
- [x] Lite版本详细设计
- [x] Standard版本详细设计
- [x] Pro版本详细设计
- [x] Monorepo结构规划
- [x] UI/UX自动化方案
- [x] 代码质量标准
- [x] **AI全自动化运维接口** (新增)
- [x] **文档归档与索引** (2026-04-01)
- [x] **Phase 1 Lite版本开发完成**

**测试状态**: 962 tests passing

**下一步**:
- [ ] 用户反馈收集
- [ ] Phase 2 Standard版本开发
- [ ] 嵌入式终端集成
- [ ] 分屏布局实现
