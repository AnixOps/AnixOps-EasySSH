# AI终端快速启动指南

## 安装依赖

确保Cargo.toml中已添加以下依赖：

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
async-trait = "0.1"
regex = "1.10"
```

## 配置AI提供商

### 选项1: Claude (推荐)

1. 访问 https://console.anthropic.com/
2. 创建API密钥
3. 在EasySSH中点击 `AI Assistant` 按钮
4. 点击设置 `⚙️`
5. 选择 "Claude"
6. 粘贴API密钥
7. 点击 "Save"

### 选项2: OpenAI

1. 访问 https://platform.openai.com/
2. 创建API密钥
3. 配置方式同Claude

### 选项3: 本地模型 (隐私优先)

**使用Ollama:**
```bash
# 安装Ollama
# Windows: https://ollama.com/download

# 运行本地模型
ollama run llama3

# 在EasySSH中选择 "Local" 提供商
```

**使用llama.cpp:**
```bash
# 下载llama.cpp
# 加载模型并启动HTTP服务
./server -m model.gguf --port 8080
```

## 快速使用

### 1. 打开AI助手
点击工具栏 `🧐` 按钮

### 2. 自然语言转命令
```
输入: "查找大于100MB的文件"
输出: find . -type f -size +100M
```

### 3. 命令解释
选择命令，点击 "Explain" 标签

### 4. 安全审计
在 "Audit" 标签中输入命令，查看风险等级

### 5. 错误诊断
当命令出错时，切换到 "Fix" 标签获取解决方案

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Shift+A` | 切换AI面板 |
| `Tab` | 命令补全 |

## 故障排查

### 连接失败
- 检查网络连接
- 验证API密钥
- 检查AI提供商服务状态

### 响应慢
- 切换到本地模型
- 启用缓存（默认已启用）
- 降低max_tokens设置

## 隐私说明

- 使用本地模型时，数据不会离开您的机器
- 使用云端API时，命令数据会被发送到AI提供商
- 敏感命令（含密码等）会自动过滤

## 安全警告

- AI建议仅供参考，执行前请确认
- 高危命令（rm -rf等）会要求额外确认
- 建议开启自动安全审计

## 支持

遇到问题？
- 提交Issue: https://github.com/anix/easyssh/issues
- 查看文档: AI_TERMINAL.md

## 功能截图

```
┌─────────────────────────────────────────────────────────┐
│ EasySSH                               [🧐] [🎨] [⚙️]   │
├─────────────────────────────────────────────────────────┤
│ Server List    │ Terminal            │ AI Assistant     │
│ ─────────────  │ ─────────────────── │ ──────────────── │
│                │ $                   │ Ask  Explain Fix │
│ Production     │                     │ ──────────────── │
│   web-server-1 │                     │ Input:           │
│   db-master    │                     │ [show memory   ] │
│                │                     │                  │
│ Development    │                     │ Suggestions:     │
│   dev-box      │                     │ • free -h        │
│                │                     │ • htop           │
│                │                     │                  │
│                │                     │ Output:          │
│                │                     │ $ free -h        │
│                │                     │ [Execute] [Copy]   │
└─────────────────────────────────────────────────────────┘
```
