# EasySSH 自动化开发指南

## 🎯 快速开始

### 1. 安装 babysitter
```bash
npm install -g @a5c-ai/babysitter-cli
```

### 2. 运行自动化开发
```bash
# Linux/Mac
./scripts/autonomous-dev.sh tui 50 true

# Windows
scripts\autonomous-dev.bat tui 50 true
```

## 🔧 测试点定义

自动化流程会自动运行以下测试：

| 测试点 | 说明 | 必须通过 | 自动修复 |
|--------|------|---------|---------|
| **BUILD** | `cargo build --release` | ✅ | ✅ |
| **CONNECTION** | SSH 命令构建测试 | ✅ | ✅ |
| **CLI** | 命令行配置测试 | ✅ | ✅ |
| **TUI** | 终端界面测试 | ❌ | ✅ |
| **QUALITY** | Clippy + Fmt | ❌ | ✅ |

## 🔄 迭代流程

```
迭代 1/N
├── 🧪 运行测试
│   ├── BUILD → ❌ 失败 (类型错误)
│   └── 停止 (关键测试失败)
│
├── 🔧 分析错误
│   └── 识别: build_error
│       └── 策略: 修复类型不匹配
│
├── 📝 应用修复
│   └── 修改 src/terminal.rs:42
│       修复: String → &str
│
迭代 2/N
├── 🧪 运行测试
│   ├── BUILD → ✅ 通过
│   ├── CONNECTION → ✅ 通过
│   └── CLI → ❌ 失败 (命令解析)
│
└── 🔧 自动修复 CLI 解析...
```

## 🛠️ 修复策略

系统自动识别并修复以下问题：

### 构建错误 (`build_error`)
- 类型不匹配 → 添加类型转换或修改签名
- 未找到模块 → 添加 `use` 语句
- 缺少依赖 → 更新 `Cargo.toml`

### 连接错误 (`connection_error`)
- 终端启动失败 → 修复 `terminal.rs`
- 命令构建错误 → 修复 SSH 参数拼接
- 进程 spawn 失败 → 添加错误处理

### CLI 错误 (`cli_error`)
- 未知命令 → 添加命令处理分支
- 参数解析错误 → 修复参数检查
- 帮助信息错误 → 更新帮助文本

### TUI 错误 (`tui_error`)
- Panic → 添加错误边界
- 终端模式错误 → 修复 raw mode 管理
- 渲染错误 → 检查 ratatui 用法

## 📊 报告输出

每次运行生成 JSON 报告：

```json
{
  "success": true,
  "iterations": 12,
  "tests": {
    "build": true,
    "connection": true,
    "cli": true,
    "tui": false,
    "quality": true
  },
  "fixes": [
    {
      "iteration": 2,
      "type": "build_error",
      "files": ["src/terminal.rs"],
      "description": "修复 SSH 命令参数类型"
    },
    {
      "iteration": 5,
      "type": "cli_error",
      "files": ["src/bin/tui.rs"],
      "description": "添加缺失的 help 命令处理"
    }
  ],
  "summary": "✅ 全自动开发完成！共迭代 12 次，应用 2 个修复"
}
```

## 🎮 手动触发 babysitter

### 基础用法
```bash
# 运行 50 次迭代，自动修复
a5c run easyssh-autonomous-dev \
  --input '{"maxIterations": 50, "testMode": "tui", "autoFix": true}'

# 只测试不修复
a5c run easyssh-autonomous-dev \
  --input '{"maxIterations": 10, "autoFix": false}'

# GUI 模式测试
a5c run easyssh-autonomous-dev \
  --input '{"testMode": "gui", "maxIterations": 30}'
```

### 查看运行日志
```bash
# 列出所有运行
a5c runs list

# 查看特定运行详情
a5c runs show <run-id>

# 查看实时日志
a5c runs logs <run-id> --follow
```

## 🔧 自定义测试点

编辑 `.a5c/processes/easyssh-autonomous-dev.js`：

```javascript
const TEST_POINTS = {
  MY_CUSTOM_TEST: {
    id: 'custom',
    name: '自定义测试',
    commands: [
      { cmd: 'cargo test my_test', cwd: 'src-tauri' },
    ],
    mustPass: true,
  },
};
```

## 🚨 故障排除

### 迭代卡住
- 检查是否达到 `maxIterations`
- 查看日志: `a5c runs logs <run-id>`
- 可能是修复策略无法匹配错误

### 修复不生效
- 检查修复策略的 `patterns` 是否匹配错误输出
- 手动运行测试确认错误复现
- 更新修复策略正则表达式

### 构建太慢
- 使用 `cargo check` 代替 `cargo build` 快速测试
- 在 `TEST_POINTS.BUILD` 中移除 `--release` 标志
- 增加迭代 `timeout` 值

## 📝 最佳实践

1. **从小迭代开始**: 先用 `maxIterations: 10` 测试流程
2. **分阶段启用**: 先 `autoFix: false` 观察测试点，再启用修复
3. **保留日志**: 所有运行记录保存在 `.a5c/runs/` 目录
4. **定期审查**: 检查自动修复的代码质量
5. **组合使用**: 自动化 + 人工审查 = 最佳效果

## 🎯 使用场景

| 场景 | 命令 |
|------|------|
| 快速验证 | `autonomous-dev.sh tui 10 false` |
| 完整开发 | `autonomous-dev.sh tui 50 true` |
| 仅构建测试 | `autonomous-dev.sh tui 5 false` |
| 质量检查 | `autonomous-dev.sh all 20 true` |

## 🔗 相关文件

- 流程定义: `.a5c/processes/easyssh-autonomous-dev.js`
- 运行脚本: `scripts/autonomous-dev.sh` (Linux/Mac)
- 运行脚本: `scripts/autonomous-dev.bat` (Windows)
- 原始重构流程: `.a5c/processes/easyssh-iterative-refactor.js`
