# EasySSH Lite UI 截图指南

> 用于制作教程、文档和市场推广素材

---

## 截图环境要求

### 系统设置

| 项目 | Windows | macOS | Linux |
|------|---------|-------|-------|
| 分辨率 | 1920x1080 | 1920x1080 | 1920x1080 |
| 缩放 | 100% | 默认 | 100% |
| 主题 | 选择深色或浅色 | 与截图需求匹配 | 选择深色或浅色 |
| 壁纸 | 纯色/简洁 | 纯色/简洁 | 纯色/简洁 |
| Dock/任务栏 | 自动隐藏 | 自动隐藏 | 自动隐藏 |

### 应用准备

1. **使用发布版本**
   - 不要使用开发版本
   - 版本号应为正式版（非 beta/alpha）

2. **数据准备**
   - 使用示例数据（见下方）
   - 不要使用真实服务器信息
   - 敏感信息打码处理

3. **窗口大小**
   - 默认窗口尺寸：800x600
   - 最大化截图：1920x1080
   - 保持窗口边框完整

---

## 示例数据规范

### 服务器数据（用于截图）

```yaml
# 生产环境分组
production:
  - name: web-server-01
    host: 192.168.1.10
    user: admin
    port: 22
    status: online

  - name: web-server-02
    host: 192.168.1.11
    user: admin
    port: 22
    status: online

  - name: database-primary
    host: 192.168.1.20
    user: dba
    port: 22
    status: warning

# 测试环境分组
staging:
  - name: staging-web
    host: 10.0.2.10
    user: dev
    port: 22
    status: online

  - name: staging-db
    host: 10.0.2.20
    user: dev
    port: 22
    status: offline

# 未分组
ungrouped:
  - name: my-laptop
    host: 192.168.0.50
    user: z7299
    port: 22
    status: online
```

### 设置数据

```yaml
# 用于设置页面截图
settings:
  language: zh-CN
  theme: dark
  font_size: medium
  density: comfortable
  master_password: enabled
  auto_lock: 10min
```

---

## 截图清单

### 主界面截图

| 序号 | 描述 | 窗口尺寸 | 主题 | 文件名 |
|------|------|----------|------|--------|
| 1 | 空状态界面 | 800x600 | Dark | main-empty-dark.png |
| 2 | 空状态界面 | 800x600 | Light | main-empty-light.png |
| 3 | 带分组的服务器列表 | 800x600 | Dark | main-with-servers-dark.png |
| 4 | 带分组的服务器列表 | 800x600 | Light | main-with-servers-light.png |
| 5 | 搜索结果 | 800x600 | Dark | main-search-dark.png |
| 6 | 右键菜单 | 800x600 | Dark | main-context-menu.png |

### 添加服务器对话框

| 序号 | 描述 | 主题 | 文件名 |
|------|------|------|--------|
| 7 | 步骤一：基本信息 | Dark | add-step1-dark.png |
| 8 | 步骤二：认证方式-SSH Agent | Dark | add-step2-agent-dark.png |
| 9 | 步骤二：认证方式-密码 | Dark | add-step2-password-dark.png |
| 10 | 步骤二：认证方式-密钥 | Dark | add-step2-key-dark.png |
| 11 | 步骤三：选择分组 | Dark | add-step3-group-dark.png |
| 12 | 添加成功状态 | Dark | add-success-dark.png |

### 设置界面

| 序号 | 描述 | 主题 | 文件名 |
|------|------|------|--------|
| 13 | 通用设置 | Dark | settings-general-dark.png |
| 14 | 外观设置 | Dark | settings-appearance-dark.png |
| 15 | 外观设置-展开 | Dark | settings-appearance-expanded-dark.png |
| 16 | 安全设置 | Dark | settings-security-dark.png |
| 17 | 快捷键 | Dark | settings-shortcuts-dark.png |
| 18 | 分组管理 | Dark | settings-groups-dark.png |
| 19 | 导入导出 | Dark | settings-import-export-dark.png |

### 编辑服务器

| 序号 | 描述 | 主题 | 文件名 |
|------|------|------|--------|
| 20 | 编辑基本信息 | Dark | edit-basic-dark.png |
| 21 | 编辑认证方式 | Dark | edit-auth-dark.png |

### 其他功能

| 序号 | 描述 | 主题 | 文件名 |
|------|------|------|--------|
| 22 | 删除确认对话框 | Dark | dialog-delete-confirm.png |
| 23 | SSH Config 导入预览 | Dark | import-preview-dark.png |
| 24 | 导入结果统计 | Dark | import-result-dark.png |
| 25 | 连接状态提示 | Dark | connection-status-toast.png |

### 标注版截图

| 序号 | 描述 | 文件名 |
|------|------|--------|
| 26 | 主界面元素标注 | main-annotated.png |
| 27 | 添加流程标注 | add-flow-annotated.png |
| 28 | 设置界面标注 | settings-annotated.png |

---

## 截图规范

### 图片格式

| 用途 | 格式 | 尺寸 | 压缩 |
|------|------|------|------|
| 文档 | PNG | 原尺寸 | 无损 |
| 网页 | WebP | 原尺寸 | 质量90% |
| 缩略图 | WebP | 400x300 | 质量85% |
| 文档嵌入 | PNG | 800px宽 | 无损 |

### 文件命名

```
{功能}-{状态}-{主题}-{语言}.{格式}

示例：
main-empty-dark-zh.png
add-step1-light-en.png
settings-general-dark-zh.webp
```

### 目录结构

```
screenshots/
├── raw/                    # 原始截图
│   ├── windows/
│   ├── macos/
│   └── linux/
├── processed/              # 处理后截图
│   ├── dark/
│   ├── light/
│   └── annotated/
├── thumbnails/             # 缩略图
└── archive/                # 旧版本存档
```

---

## 后期处理

### 标注规范

使用统一标注样式：

```
元素标注：
┌────────────────────────────────┐
│ [1] 搜索框                     │
│     🔍 搜索服务器...           │
└────────────────────────────────┘

颜色：
- 重要元素：红色 #FF4444
- 次要元素：蓝色 #4488FF
- 提示文字：绿色 #44AA44

字体：
- 标注数字：24px bold
- 说明文字：16px regular
```

### 示例标注

```
┌─────────────────────────────────────────────┐
│  EasySSH Lite              [≡] [☀]         │
├─────────────────────────────────────────────┤
│  🔍 [1] 搜索服务器...                      │
├─────────────────────────────────────────────┤
│                                             │
│  ▼ [2] Production (3)                      │
│    ├─ 🟢 [3] web-server-01               │
│       admin@192.168.1.10           [4] ▶  │
│    └─ ...                                  │
│                                             │
├─────────────────────────────────────────────┤
│  [5] [+ 添加服务器]     [6] [⚙ 设置]      │
└─────────────────────────────────────────────┘

[1] 搜索框：快速过滤服务器
[2] 分组标题：点击展开/折叠
[3] 服务器名称：双击连接
[4] 连接按钮：一键连接
[5] 添加服务器：打开添加向导
[6] 设置：打开设置面板
```

---

## 截图检查清单

发布前逐项检查：

### 质量检查

- [ ] 图片清晰，无模糊
- [ ] 无窗口阴影/反光
- [ ] 敏感信息已打码
- [ ] 示例数据使用规范数据
- [ ] 分辨率正确

### 内容检查

- [ ] 界面元素完整
- [ ] 文字清晰可读
- [ ] 状态图标正确
- [ ] 示例数据合理

### 格式检查

- [ ] 文件名符合规范
- [ ] 格式正确（PNG/WebP）
- [ ] 已压缩优化
- [ ] 已备份原始文件

---

## 工具推荐

### 截图工具

| 平台 | 工具 | 特点 |
|------|------|------|
| Windows | ShareX | 功能强大，支持标注 |
| Windows | Snipaste | 简洁易用 |
| macOS | CleanShot X | 专业截图工具 |
| macOS | 系统自带 | 快捷键 Cmd+Shift+4 |
| Linux | Flameshot | 开源，支持标注 |
| Linux | Shutter | 功能丰富 |

### 标注工具

| 工具 | 平台 | 用途 |
|------|------|------|
| Figma | 全平台 | 专业设计标注 |
| Excalidraw | Web | 手绘风格标注 |
| Skitch | macOS | 快速标注 |
| Paint.NET | Windows | 简单编辑 |

---

## 更新流程

当应用 UI 更新时：

1. **创建新分支**
   ```bash
   git checkout -b update-screenshots-v1.1
   ```

2. **重新截图**
   - 按清单重新拍摄
   - 保持相同角度/尺寸

3. **对比检查**
   - 与旧版本对比
   - 确认变化点正确

4. **更新文档**
   - 替换文档中的图片
   - 更新图片引用路径

5. **提交审核**
   - 设计师审核标注
   - 产品经理确认内容

6. **合并发布**
   ```bash
   git merge update-screenshots-v1.1
   ```
