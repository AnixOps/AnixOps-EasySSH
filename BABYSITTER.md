# 🤖 EasySSH Babysitter 自动化系统

## 概述

本项目配置了完整的 AI 自动化开发流程，使用 [Babysitter SDK](https://github.com/a5c-ai/babysitter) 实现自测试、自修复、自迭代。

## 🎯 两种模式

### 1️⃣ 迭代重构模式 (Iterative Refactor)
**用途**: 大规模代码重构
**流程**: 设计系统 → 拆分领域存储 → 重定义产品模式 → 终端工作区

```bash
a5c run easyssh-iterative-refactor --input '{"maxIterations": 100}'
```

**适用场景**:
- 代码库结构重组
- 组件拆分
- 状态管理重构

### 2️⃣ 自主开发模式 (Autonomous Dev) ⭐ 推荐
**用途**: 自动化测试和修复
**流程**: 测试 → 分析 → 修复 → 验证 → 报告

```bash
# 快速测试
./scripts/autonomous-dev.sh tui 10 false

# 完整开发
./scripts/autonomous-dev.sh tui 50 true
```

**适用场景**:
- 日常开发验证
- Bug 自动修复
- CI/CD 集成

## 📂 文件结构

```
.a5c/
├── processes/
│   ├── easyssh-iterative-refactor.js   # 迭代重构流程 (100轮)
│   └── easyssh-autonomous-dev.js        # 自主开发流程 ⭐
├── runs/                                 # 运行记录
└── cache/                                # 缓存数据

scripts/
├── autonomous-dev.sh                     # Linux/Mac 运行脚本
├── autonomous-dev.bat                    # Windows 运行脚本
└── test-debug.sh                         # 调试脚本

docs/
├── AUTONOMOUS_DEV.md                     # 详细文档
└── standards/debug-interface.md          # AI 接口设计
```

## 🚀 快速开始

### 安装
```bash
npm install -g @a5c-ai/babysitter-cli
```

### 一键运行
```bash
# 运行 50 次迭代，自动修复问题
scripts/autonomous-dev.bat tui 50 true   # Windows
./scripts/autonomous-dev.sh tui 50 true  # Linux/Mac
```

### 查看结果
```bash
# 列出所有运行
a5c runs list

# 查看最新报告
a5c runs show <run-id>

# 实时日志
a5c runs logs <run-id> --follow
```

## 🧪 测试点矩阵

| 测试点 | 说明 | 触发条件 | 自动修复 | 阻断流程 |
|--------|------|---------|---------|---------|
| **BUILD** | `cargo build --release` | 每次迭代 | ✅ | ✅ |
| **CONNECTION** | SSH 命令构建 | BUILD 通过 | ✅ | ✅ |
| **CLI** | 命令行配置 | BUILD 通过 | ✅ | ✅ |
| **TUI** | 终端界面 | BUILD 通过 | ✅ | ❌ |
| **QUALITY** | Clippy + Fmt | BUILD 通过 | ✅ | ❌ |

## 🔧 自动修复能力

### 支持的修复类型

| 错误类型 | 示例 | 修复策略 |
|---------|------|---------|
| `build_error` | `error[E0308]: mismatched types` | 类型转换、添加 import |
| `connection_error` | `failed to launch wt` | 修复终端启动逻辑 |
| `cli_error` | `Unknown command` | 添加命令处理 |
| `tui_error` | `panic in raw mode` | 修复生命周期管理 |

### 修复流程
```
检测错误
    ↓
匹配模式 (正则)
    ↓
选择策略
    ↓
生成修复
    ↓
应用修改
    ↓
重新测试
    ↓
验证通过 ✅
```

## 📊 迭代统计

### 当前状态
- **总迭代数**: 50+ (上次运行)
- **成功率**: 100% (BUILD)
- **平均修复数**: 2-3 个/次完整运行
- **平均耗时**: 15-30 分钟/50次迭代

### 历史记录
查看 `.a5c/runs/` 目录获取详细运行历史。

## 🎮 使用场景

### 场景 1: 每日健康检查
```bash
# CI 定时运行 (凌晨 3 点)
a5c run easyssh-autonomous-dev \
  --input '{"maxIterations": 10, "autoFix": false}'
```

### 场景 2: 修复已知问题
```bash
# 针对性修复，最多 20 次尝试
a5c run easyssh-autonomous-dev \
  --input '{"maxIterations": 20, "autoFix": true}'
```

### 场景 3: 重构前验证
```bash
# 重构前确保所有测试通过
a5c run easyssh-iterative-refactor \
  --input '{"maxIterations": 5, "validateOnly": true}'
```

## ⚙️ 自定义配置

### 修改测试点
编辑 `.a5c/processes/easyssh-autonomous-dev.js`:

```javascript
const TEST_POINTS = {
  MY_TEST: {
    id: 'my_test',
    name: '我的测试',
    commands: [
      { cmd: 'cargo test custom', cwd: 'src-tauri' },
    ],
    mustPass: true,
  },
};
```

### 添加修复策略
```javascript
const FIX_STRATEGIES = {
  my_error: {
    patterns: [/my specific error/],
    actions: ['修复 A', '修复 B'],
  },
};
```

## 🔍 故障排除

### 问题: 迭代卡住
**解决**:
```bash
# 查看日志
a5c runs logs <run-id>

# 强制停止
a5c runs stop <run-id>
```

### 问题: 修复不生效
**检查**:
1. 修复策略的 `patterns` 是否匹配错误输出
2. 文件路径是否正确
3. 权限是否足够

### 问题: 构建超时
**解决**:
```javascript
// 在 TEST_POINTS 中增加 timeout
{ cmd: 'cargo build', timeout: 600000 }  // 10分钟
```

## 📈 CI/CD 集成

GitHub Actions 已配置:

```yaml
# .github/workflows/autonomous-dev.yml
name: Autonomous Dev
on:
  schedule:
    - cron: '0 3 * * *'  # 每天凌晨 3 点
  workflow_dispatch:      # 手动触发
```

## 🎯 设计理念

### AI Self-Improvement Loop
```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Observe │ ──▶ │ Analyze  │ ──▶ │   Plan   │ ──▶ │  Modify  │
│  代码状态 │     │  问题定位 │     │  修复方案 │     │  改代码  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
     ▲                                                       │
     │                                                       ▼
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Verify  │ ◀── │  Report  │ ◀── │  Execute │ ◀── │   Test   │
│  验证结果 │     │  生成报告 │     │  执行修复 │     │  跑测试  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
```

### 安全边界
- ✅ 读取源代码
- ✅ 修改代码文件
- ✅ 运行测试
- ❌ 自动提交 (需要人工审查)
- ❌ 推送到远程
- ❌ 破坏性操作

## 📚 相关文档

- [AUTONOMOUS_DEV.md](docs/AUTONOMOUS_DEV.md) - 详细使用指南
- [debug-interface.md](docs/standards/debug-interface.md) - AI 接口设计
- [CLAUDE.md](CLAUDE.md) - 项目架构文档

## 🏆 成功案例

### Case 1: SSH 连接修复
- **问题**: `wt new-tab` 参数错误导致双窗口
- **检测**: CONNECTION 测试失败
- **修复**: 自动修改 `terminal.rs`，简化启动逻辑
- **验证**: 重新测试通过
- **耗时**: 3 次迭代，约 5 分钟

### Case 2: CLI 命令解析
- **问题**: `help` 命令未处理
- **检测**: CLI 测试失败
- **修复**: 添加命令匹配分支
- **验证**: `easyssh.exe help` 输出正确
- **耗时**: 2 次迭代，约 3 分钟

---

**让 babysitter 替你打工！ 🤖**
