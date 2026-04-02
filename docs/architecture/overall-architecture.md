# EasySSH 全平台重构架构

> 目标：仿 Termius 的产品体验，但保留 EasySSH 的安全、克制和本地优先。

---

## 1. 架构目标

### 重构原则
- **先统一体验，再拆版本**：先做一致的设计系统和信息架构，再区分 Lite / Standard / Pro。
- **先共享核心，再做多端壳**：SSH、加密、同步、会话、权限等能力必须共享。
- **UI 不是壳，工作区才是产品**：桌面端必须从“页面集合”升级为“工作区”。
- **版本差异要体现在能力边界，而不是按钮数量**。

### 需要解决的现状问题
- 前端展示弱，页面像 demo，不像成熟产品。
- `Lite / Standard / Pro` 只是 viewMode，不是产品级分层。
- 组件耦合严重，布局、状态、连接逻辑混在一起。
- 终端区、服务器区、团队区没有清晰边界。

---

## 2. 重构后的产品分层

```
┌──────────────────────────────────────────────────────────────┐
│                       EasySSH 产品层                        │
├──────────────────────────────────────────────────────────────┤
│ Lite      → SSH 配置保险箱 + 快速连接 + 原生终端唤起         │
│ Standard  → 嵌入式终端工作台 + 分屏 + 多会话 + SFTP         │
│ Pro       → 团队控制台 + RBAC + 审计 + SSO + 共享资源        │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌──────────────────────────────────────────────────────────────┐
│                     统一设计系统层                          │
├──────────────────────────────────────────────────────────────┤
│ tokens / typography / spacing / color / motion / icons       │
│ sidebar / command palette / cards / tabs / panes / modals    │
│ terminal chrome / session tabs / empty states / onboarding   │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌──────────────────────────────────────────────────────────────┐
│                       统一业务核心层                          │
├──────────────────────────────────────────────────────────────┤
│ SSH Profile / Group / Session / Secret / Sync / Policy       │
│ Encryption / Keychain / Connection Manager / History         │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌──────────────────────────────────────────────────────────────┐
│                      平台适配层                               │
├──────────────────────────────────────────────────────────────┤
│ Desktop Shell (egui/GTK4/WinUI/SwiftUI)                       │
│ Mobile Shell (React Native)                                   │
│ Optional Web Admin / Pro Console                               │
└──────────────────────────────────────────────────────────────┘
```

---

## 3. 信息架构

### 主工作区
桌面端不再以“单页应用”思路组织，而是拆为四个稳定区域：

1. **全局顶栏**
   - 快速搜索
   - 连接入口
   - 全局命令面板
   - 主题 / 帐号 / 同步状态

2. **左侧导航**
   - 服务器空间
   - 分组树
   - 最近会话
   - 团队空间（Pro）

3. **中央工作区**
   - Lite：配置卡片 + 快速连接
   - Standard：终端标签 + 分屏工作台
   - Pro：团队控制台 / 审计 / 权限视图

4. **右侧详情面板**
   - 服务器属性
   - 连接状态
   - SFTP / 监控 / 审计详情

### 视觉层级
- 一级：当前工作区
- 二级：标签 / 分屏 / 侧栏
- 三级：详情和辅助信息

---

## 4. 三版本重新定义

### Lite
**定位**：SSH 配置保险箱

**必须保留**
- 服务器 CRUD
- 分组
- Keychain / 主密码
- 一键连接原生终端
- 导入 ~/.ssh/config

**明确不做**
- 内置终端模拟
- 分屏
- 云同步
- 团队协作
- 监控卡片

**界面形态**
- 轻量列表 + 卡片 + 快速操作
- 像“保险箱”，不是“控制台”

---

### Standard
**定位**：全功能个人工作台

**必须保留**
- 嵌入式终端
- 多标签页
- 分屏
- SFTP
- 命令历史
- 搜索 / 复制 / 粘贴 / 选择
- 本地加密存储

**界面形态**
- 类 Termius 的 workspace
- 左侧服务器树 + 中央终端区域 + 右侧工具面板
- 支持拖拽和布局保存

---

### Pro
**定位**：团队协作控制台

**必须保留**
- Team / RBAC
- 审计日志
- SSO
- 共享配置 / snippets / policy
- 会话留痕
- 管理后台视图

**界面形态**
- 更像“企业控制台”而不是“终端客户端”
- 强调治理、协作、合规

---

## 5. 前端架构

### 推荐目录结构

```text
src/
├── app/
│   ├── App.tsx
│   ├── routes/
│   └── providers/
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

### 拆分规则
- `app/` 只负责装配。
- `features/` 负责业务场景。
- `components/` 只放可复用 UI。
- `stores/` 按领域拆，不再放大一坨全局状态。
- `lib/` 放平台无关核心能力。

---

## 6. 状态模型重构

### 旧问题
当前 `viewMode` 只是本地状态切换，不能代表版本边界。

### 新模型

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
- `productMode` 只决定产品边界。
- `workspaceMode` 只决定当前工作区。
- 状态按“领域”拆分，不和 UI 混在一起。

---

## 7. 终端架构

### Standard / Pro
- 使用 `xterm.js + WebGL` 作为基础渲染层。
- 终端容器必须支持：
  - 标签页
  - 分屏
  - 会话恢复
  - 搜索
  - 复制历史
  - 布局保存

### Lite
- 不内嵌终端。
- 使用系统终端唤起，保持极简和低风险。

---

## 8. 全平台策略

### 桌面端
- 优先保证桌面体验完整。
- 桌面端是主战场，不做 Web 版阉割克隆。

### 移动端
- 移动端不是“缩小版桌面”，而是：
  - 快速查看连接
  - 一键连接
  - 资产浏览
  - 安全审批
  - 轻量操作

### 同步策略
- 本地优先。
- E2EE 同步只同步必要的领域数据。
- 不把 UI 状态当同步数据。

---

## 9. 重构里程碑

### Phase 1：UI 重新设计
- 替换当前单页式布局
- 引入统一设计 tokens
- 做出真正的 workspace 框架

### Phase 2：领域拆分
- 拆 server / session / team / sync store
- 抽出 terminal core
- 清理 App.tsx 的大块逻辑

### Phase 3：版本分层
- Lite / Standard / Pro 变成真正的产品层
- 删除只靠 viewMode 的伪区分

### Phase 4：多端扩展
- 桌面端稳定后再接移动端 / Web 管理端

---

## 10. 设计结论

EasySSH 不应再像一个“功能拼盘”。

它应该被重构成：
- **Lite：配置保险箱**
- **Standard：终端工作台**
- **Pro：团队控制台**

这三者共享同一套核心能力，但拥有完全不同的工作区和交互重心。

