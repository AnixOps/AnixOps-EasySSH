# EasySSH Termius 风格重构方案

> 目标：做出 Termius 那种“全平台 + 工作区”的产品感，但保持 EasySSH 的本地优先、安全和克制。

---

## 1. 重构目标

### 要解决的核心问题
- 当前前端显示像 demo，不像成熟产品。
- Lite / Standard / Pro 只是 viewMode，不是产品分层。
- 终端、服务器、团队、同步混在一起，缺少清晰边界。
- UI 没有统一设计系统，难以形成品牌感。

### 重构原则
- **先统一体验，再分版本**
- **先共享核心，再做多端壳**
- **产品分层必须体现在工作区和能力边界上**
- **版本差异不是功能堆叠，而是使用场景不同**

---

## 2. 全平台架构图

```text
┌──────────────────────────────────────────────────────────────┐
│                        EasySSH 产品层                       │
├──────────────────────────────────────────────────────────────┤
│ Lite      → SSH 配置保险箱 + 快速连接 + 原生终端唤起        │
│ Standard  → 嵌入式终端工作台 + 分屏 + 多会话 + SFTP        │
│ Pro       → 团队控制台 + RBAC + 审计 + SSO + 共享资源       │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌──────────────────────────────────────────────────────────────┐
│                      统一设计系统层                          │
├──────────────────────────────────────────────────────────────┤
│ tokens / typography / spacing / color / motion / icons      │
│ sidebar / command palette / cards / tabs / panes / modals   │
│ terminal chrome / empty states / onboarding / dialogs       │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌──────────────────────────────────────────────────────────────┐
│                      统一业务核心层                          │
├──────────────────────────────────────────────────────────────┤
│ SSH Profile / Group / Session / Secret / Sync / Policy      │
│ Encryption / Keychain / Connection Manager / History        │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌──────────────────────────────────────────────────────────────┐
│                     平台适配与展示层                         │
├──────────────────────────────────────────────────────────────┤
│ Desktop Shell (Tauri 或 Electron)                            │
│ Mobile Shell (React Native)                                  │
│ Optional Web Admin / Pro Console                             │
└──────────────────────────────────────────────────────────────┘
```

---

## 3. 三个版本的真正区别

### Lite
**定位**：配置保险箱

**体验关键词**
- 轻
- 快
- 安全
- 原生

**界面形态**
- 服务器列表 / 分组树
- 快速搜索
- 一键连接
- 设置面板

**不做**
- 嵌入式终端
- 分屏
- 云同步
- 团队协作
- 监控小组件

---

### Standard
**定位**：个人终端工作台

**体验关键词**
- 工作区
- 多会话
- 分屏
- 高效操作

**界面形态**
- 左侧服务器空间
- 中央终端工作区
- 右侧详情/工具面板
- 标签页 + 分屏布局

**核心能力**
- xterm.js + WebGL
- 多标签页
- 分屏保存
- 命令历史
- SFTP
- 会话恢复

---

### Pro
**定位**：团队协作控制台

**体验关键词**
- 协作
- 审计
- 权限
- 合规

**界面形态**
- 团队空间
- 审计视图
- 权限管理
- 共享资源库

**核心能力**
- RBAC
- SSO
- 审计日志
- 共享 snippets
- 共享服务器空间
- 管理控制台

---

## 4. 前端结构建议

```text
src/
├── app/
│   ├── App.tsx
│   ├── providers/
│   └── routes/
├── features/
│   ├── servers/
│   ├── terminals/
│   ├── sessions/
│   ├── sync/
│   ├── auth/
│   └── team/
├── components/
│   ├── layout/
│   ├── navigation/
│   ├── forms/
│   ├── terminal/
│   └── feedback/
├── stores/
│   ├── uiStore.ts
│   ├── serverStore.ts
│   ├── sessionStore.ts
│   └── teamStore.ts
├── lib/
│   ├── api/
│   ├── crypto/
│   ├── sync/
│   └── terminal/
└── styles/
    ├── tokens.css
    └── globals.css
```

### 结构规则
- `app/` 只负责装配。
- `features/` 按业务场景拆。
- `components/` 只放可复用 UI。
- `stores/` 按领域拆，不混 UI 与业务。
- `lib/` 放平台无关核心能力。

---

## 5. UI 重构重点

### 现在的问题
- 太依赖 inline style。
- 层级不清晰。
- 侧栏、列表、终端、设置都挤在一个页面里。
- 没有统一的 spacing / typography / color 规范。

### 需要建立的基础组件
- AppShell
- Sidebar
- TopBar
- Workspace
- SessionTabs
- SplitPane
- DetailPanel
- EmptyState
- CommandPalette
- Modal / Drawer / Toast

### 视觉策略
- 不是“更花”，而是“更像产品”。
- 先建立稳定的布局框架，再填功能。
- 让每个版本都看起来像同一产品线，而不是不同 demo。

---

## 6. 状态模型建议

```ts
type ProductMode = 'lite' | 'standard' | 'pro'
type WorkspaceMode = 'vault' | 'terminal' | 'team'

interface UiState {
  productMode: ProductMode
  workspaceMode: WorkspaceMode
  sidebarCollapsed: boolean
  rightPanelOpen: boolean
  activeLayoutId: string | null
}

interface DomainState {
  servers: Server[]
  groups: Group[]
  sessions: Session[]
  secrets: Secret[]
  team?: TeamState
}
```

### 关键变化
- `productMode` 只表示版本。
- `workspaceMode` 只表示当前工作区。
- UI 状态与业务状态分离。

---

## 7. 迁移路线

### Phase 1：重做骨架
- 统一布局
- 建立 design system
- 做出 workspace 框架

### Phase 2：拆业务域
- 拆 server / session / team / sync store
- 抽 terminal core
- 清掉 App.tsx 里的大块逻辑

### Phase 3：重定义版本边界
- Lite / Standard / Pro 变成真正的产品层
- 删除只靠 viewMode 维持的伪区分

### Phase 4：扩展多端
- 桌面端先稳定
- 再接移动端与 Web 管理端

---

## 8. 结论

EasySSH 不应该继续像一个单页工具，而应该被重构成：

- **Lite：配置保险箱**
- **Standard：终端工作台**
- **Pro：团队控制台**

这三者共享同一套核心能力，但拥有不同的工作区、不同的交互重心、不同的产品叙事。