# EasySSH v0.3.0 Release Checklist

## 版本信息
- **版本号**: 0.3.0
- **发布日期**: 2026-04-01
- **Git Commit**: 59b5783
- **平台**: Windows x86-64

## 编译验证

| 组件 | 调试构建 | 发布构建 | 状态 |
|------|---------|---------|------|
| easyssh-core | 通过 | 通过 | 已修复变量名错误 |
| easyssh-winui | 通过 | 通过 | 编译警告已接受 |
| easyssh-gtk4 | 阻塞 | 阻塞 | 需要Linux环境 |

## 测试验证

| 测试类型 | 通过 | 失败 | 忽略 |
|---------|------|------|------|
| 单元测试 | 186 | 0 | 7 |
| 文档测试 | 28 | 0 | 0 |
| **总计** | **214** | **0** | **7** |

## 二进制文件

| 文件 | 大小 | 状态 |
|------|------|------|
| EasySSH.exe | 10.1 MB | 已生成 |
| EasySSH-Debug.exe | 9.1 MB | 已生成 |
| easyssh_core.dll | 1.8 MB | 已生成 |
| easyssh_core.lib | 60.6 MB | 已生成 |

## 修复的问题

1. log_monitor.rs:819 - 修复 `_source_id` -> `source_id`
2. design.rs:853 - 修复 `_theme` -> `theme`
3. apple_design.rs:703 - 修复 `_theme` -> `theme`

## 发布状态

- [x] Windows版本: **READY**
- [ ] Linux版本: **BLOCKED** (需要GTK4系统库)
- [ ] macOS版本: **NOT IMPLEMENTED**
- [ ] TUI版本: **NOT IMPLEMENTED** (目录缺失)
- [ ] Pro Server: **NOT IMPLEMENTED** (目录缺失)

## 发布建议

### 立即执行
1. 打包 Windows 版本发布包
2. 创建 GitHub Release
3. 编写发布说明

### 后续工作
1. 在 Linux CI 环境中编译 GTK4 版本
2. 实现缺失的 workspace 成员
3. 修复 clippy 警告 (可选)

## 发布命令

```bash
# 生成发布构建
cargo build --release -p easyssh-winui

# 复制二进制文件到发布目录
cp target/release/EasySSH.exe releases/v0.3.0/
cp target/release/easyssh_core.dll releases/v0.3.0/

# 运行测试
cargo test -p easyssh-core --release
```

---
**验证完成时间**: 2026-04-01 18:45
**验证工具**: Claude Code
