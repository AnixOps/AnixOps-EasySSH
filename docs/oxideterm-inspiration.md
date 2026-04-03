# OxideTerm 灵感分析与 EasySSH 未来规划

> **参考项目**: [OxideTerm](https://github.com/AnalyseDeCircuit/oxideterm)
> **分析日期**: 2026-04-03
> **当前版本**: OxideTerm v1.0.11 | EasySSH v0.3.0-beta.1

---

## 一、OxideTerm 项目概览

### 1.1 基本信息

| 属性 | OxideTerm | EasySSH |
|------|-----------|---------|
| 技术栈 | Tauri 2 + React 19 + Rust | Rust Native (egui/GTK4/SwiftUI) |
| 包体积 | ~10MB | ~5MB (Lite) |
| License | GPL-3.0 | MIT/Apache-2.0 |
| Stars | 128 | - |
| 定位 | All-in-one 终端工作区 | 三版本 SSH 客户端 |

### 1.2 OxideTerm 核心功能

```
OxideTerm 功能矩阵:
├── 终端
│   ├── xterm.js + WebGL 加速
│   ├── 本地 PTY (portable-pty)
│   ├── WSLg 图形转发
│   ├── 终端搜索 (正则支持)
│   └── 会话录制
├── SSH
│   ├── russh (纯 Rust, 无 OpenSSL)
│   ├── 连接池
│   ├── 自动重连编排器
│   └── 跳板机级联
├── SFTP
│   ├── 文件传输 + 进度
│   ├── 归档支持 (zip/tar)
│   └── 文件类型检测
├── AI 集成
│   ├── 内联终端助手
│   ├── 侧边栏聊天 (40+ 工具)
│   ├── MCP 协议支持
│   ├── RAG 本地知识库 (BM25 + HNSW)
│   └── BYOK (支持 OpenAI/Ollama/DeepSeek)
├── 插件系统
│   ├── 运行时动态插件
│   ├── UI 视图扩展
│   ├── 终端钩子
│   └── 连接生命周期钩子
├── 远程 IDE
│   ├── CodeMirror 编辑器
│   ├── 语法高亮 (20+ 语言)
│   └── Git 集成
├── UI/UX
│   ├── 命令面板
│   ├── 30+ 主题
│   ├── 11 种语言
│   └── 拓扑视图
└── 其他
    ├── 端口转发
    ├── WebSocket 桥接
    └── 远程 Agent
```

---

## 二、架构对比分析

### 2.1 技术架构

| 维度 | OxideTerm | EasySSH | 分析 |
|------|-----------|---------|------|
| **UI 框架** | Tauri 2 + React | 纯原生 (egui/GTK4/SwiftUI) | EasySSH 更轻量，OxideTerm 更灵活 |
| **SSH 库** | russh (纯 Rust) | ssh2 / russh | OxideTerm 避免了 OpenSSL 依赖 |
| **终端** | xterm.js (WebGL) | 原生终端 / 嵌入式 | OxideTerm 功能更丰富 |
| **状态管理** | Zustand (多 Store) | 各平台原生 | OxideTerm 架构更统一 |
| **IPC** | Tauri IPC + WebSocket | FFI | OxideTerm 双平面架构更优雅 |

### 2.2 OxideTerm 双平面架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (React 19)                       │
├─────────────────────────────────────────────────────────────┤
│  控制平面 (Tauri IPC)    │    数据平面 (WebSocket Binary)    │
│  - 连接管理              │    - 终端 I/O (<1ms 延迟)         │
│  - 配置读写              │    - 绕过 JSON 序列化             │
│  - 插件 API              │    - 实时流式传输                 │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                    Backend (Rust/Tauri 2)                    │
├─────────────────────────────────────────────────────────────┤
│  Router → SSH Pool → russh → WebSocket Bridge               │
└─────────────────────────────────────────────────────────────┘
```

**优势**:
- 数据平面直连 WebSocket，无 IPC 序列化开销
- 控制平面使用标准 IPC，便于调试和扩展
- 分离关注点，易于维护

### 2.3 状态管理对比

**OxideTerm 多 Store 架构**:
```typescript
// 18 个专用 Store，各司其职
agentStore.ts         // AI Agent 状态
aiChatStore.ts        // AI 对话
appStore.ts           // 应用全局状态
commandPaletteStore.ts // 命令面板
ideStore.ts           // IDE 模式
launcherStore.ts      // 启动器
localTerminalStore.ts // 本地终端
pluginStore.ts        // 插件注册
ragStore.ts           // RAG 知识库
reconnectOrchestratorStore.ts // 重连编排
sessionTreeStore.ts   // 会话树
settingsStore.ts      // 设置
transferStore.ts      // 文件传输
// ...
```

**EasySSH 当前状态**:
- 各平台独立实现
- 缺乏统一的状态管理
- 需要建立跨平台抽象层

---

## 三、关键功能对比

### 3.1 终端功能

| 功能 | OxideTerm | EasySSH Lite | EasySSH Standard |
|------|-----------|--------------|------------------|
| 嵌入式终端 | ✅ xterm.js WebGL | ❌ | ✅ 计划中 |
| 本地 PTY | ✅ portable-pty | ❌ | ❌ |
| 终端搜索 | ✅ 正则支持 | ❌ | ❌ |
| 会话录制 | ✅ | ❌ | ❌ |
| 自动重连 | ✅ 编排器 | ❌ | ❌ |

### 3.2 AI 功能

| 功能 | OxideTerm | EasySSH |
|------|-----------|---------|
| 内联助手 | ✅ | ❌ 未规划 |
| 侧边栏聊天 | ✅ 40+ 工具 | ❌ 未规划 |
| MCP 协议 | ✅ | ❌ 未规划 |
| RAG 知识库 | ✅ BM25+HNSW | ❌ 未规划 |
| 错误诊断 | ✅ 自动 | ❌ 未规划 |

### 3.3 插件系统

| 功能 | OxideTerm | EasySSH |
|------|-----------|---------|
| 运行时插件 | ✅ ESM bundle | ❌ 未规划 |
| UI 扩展 | ✅ Tab/Panel | ❌ |
| 终端钩子 | ✅ 输入/输出 | ❌ |
| 连接钩子 | ✅ 生命周期 | ❌ |

---

## 四、EasySSH 未来规划

### 4.1 Phase 2: Standard 版本增强 (Q2 2026)

#### 4.1.1 终端子系统重构

**目标**: 实现高性能嵌入式终端

```
终端架构设计:
├── 后端 (Rust)
│   ├── terminal/pty.rs      # PTY 管理 (移植 portable-pty)
│   ├── terminal/scroll.rs   # 滚动缓冲区
│   ├── terminal/search.rs   # 搜索功能
│   └── terminal/recording.rs # 会话录制
├── 前端
│   ├── Windows: ConPTY → egui 渲染
│   ├── Linux: PTY → GTK4 TextView
│   └── macOS: PTY → SwiftUI
└── 通信
    └── FFI callbacks / WebSocket (未来)
```

**参考 OxideTerm**:
- `src-tauri/src/session/scroll_buffer.rs` - 滚动缓冲实现
- `src-tauri/src/session/search.rs` - 正则搜索

#### 4.1.2 连接自愈系统

```rust
// 参考 OxideTerm reconnect_orchestrator
pub struct ReconnectOrchestrator {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    current_attempts: AtomicU32,
}

impl ReconnectOrchestrator {
    pub async fn handle_disconnect(&self, conn: &mut Connection) {
        loop {
            let delay = self.calculate_backoff();
            tokio::time::sleep(delay).await;

            match conn.reconnect().await {
                Ok(_) => {
                    self.reset_attempts();
                    emit_event("connection:update", "reconnect_success");
                    break;
                }
                Err(_) if self.should_retry() => continue,
                Err(_) => {
                    emit_event("connection:update", "reconnect_failed");
                    break;
                }
            }
        }
    }
}
```

#### 4.1.3 SFTP 增强

```
SFTP 功能路线图:
├── 基础传输 (已完成)
├── 进度回调 (Phase 2.1)
├── 拖拽上传/下载 (Phase 2.2)
├── 归档支持 (Phase 2.3)
│   ├── zip 解压/压缩
│   └── tar.gz 支持
├── 文件预览 (Phase 2.4)
│   ├── 图片预览
│   ├── 代码高亮
│   └── Markdown 渲染
└── 批量操作 (Phase 2.5)
```

### 4.2 Phase 3: Pro 版本与 AI 集成 (Q3-Q4 2026)

#### 4.2.1 AI 助手集成

**设计原则**: BYOK (Bring Your Own Key)

```
AI 子系统架构:
├── core/ai/
│   ├── mod.rs           # 模块入口
│   ├── provider.rs      # Provider 抽象
│   ├── openai.rs        # OpenAI 兼容 API
│   ├── ollama.rs        # Ollama 本地模型
│   └── config.rs        # 配置管理
├── core/ai/context/
│   ├── terminal.rs      # 终端上下文提取
│   ├── error.rs         # 错误诊断
│   └── suggestion.rs    # 命令建议
└── UI 集成
    ├── 内联面板 (Ctrl+Shift+I)
    └── 侧边栏聊天
```

**核心接口**:
```rust
pub trait AIProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<String>;
    async fn chat_stream(&self, messages: Vec<Message>) -> Result<Receiver<String>>;
    fn models(&self) -> Vec<ModelInfo>;
}

pub struct AIContext {
    pub os_type: OsType,           // Local/Remote
    pub selected_text: Option<String>,
    pub last_error: Option<String>,
    pub command_history: Vec<String>,
}

pub fn build_prompt(ctx: &AIContext, query: &str) -> String {
    // 构建上下文感知提示词
}
```

#### 4.2.2 RAG 本地知识库

**参考 OxideTerm RAG 系统**:

```rust
// core/rag/
pub mod chunker;     // 文档分块 (Markdown 感知)
pub mod bm25;        // BM25 稀疏检索
pub mod hnsw;        // HNSW 向量搜索
pub mod store;       // 持久化 (SQLite)

pub struct RAGSystem {
    bm25_index: BM25Index,
    hnsw_index: Option<HNSWIndex>,  // 可选，需要嵌入模型
    store: RAGStore,
}

impl RAGSystem {
    pub fn index_document(&mut self, doc: &Document) -> Result<()>;
    pub fn search(&self, query: &str, k: usize) -> Vec<SearchResult>;
    pub fn hybrid_search(&self, query: &str, k: usize) -> Vec<SearchResult>;
}
```

**使用场景**:
- 运维手册自动检索
- API 文档即时查询
- 历史命令搜索

#### 4.2.3 MCP 协议支持

```
MCP (Model Context Protocol) 集成:
├── core/mcp/
│   ├── client.rs        # MCP 客户端
│   ├── tools.rs         # 工具注册
│   └── resources.rs     # 资源管理
├── 内置工具
│   ├── ssh_execute      # 执行远程命令
│   ├── sftp_read        # 读取远程文件
│   ├── sftp_write       # 写入远程文件
│   └── port_check       # 端口检查
└── 扩展工具
    └── 用户自定义工具
```

### 4.3 Phase 4: 插件系统 (2027 Q1)

#### 4.3.1 插件架构设计

**参考 OxideTerm 运行时插件系统**:

```
插件系统架构:
├── core/plugin/
│   ├── mod.rs           # 模块入口
│   ├── registry.rs      # 插件注册表
│   ├── loader.rs        # 插件加载器
│   ├── membrane.rs      # API 隔离层
│   └── api.rs           # 插件 API
├── 插件目录
│   ├── ~/.easyssh/plugins/
│   │   └── {plugin-id}/
│   │       ├── plugin.json
│   │       ├── index.js
│   │       └── locales/
└── 插件类型
    ├── 连接生命周期钩子
    ├── 终端输入/输出钩子
    ├── UI 视图扩展
    └── 命令扩展
```

**plugin.json 示例**:
```json
{
  "id": "com.easyssh.audit",
  "name": "SSH Audit",
  "version": "1.0.0",
  "description": "Security audit for SSH connections",
  "main": "./index.js",
  "engines": { "easyssh": ">=1.0.0" },
  "contributes": {
    "hooks": {
      "onConnect": "handleConnect",
      "onDisconnect": "handleDisconnect"
    },
    "commands": [{
      "id": "audit.export",
      "title": "Export Audit Log"
    }]
  }
}
```

#### 4.3.2 插件 API

```rust
// 插件可访问的 API (通过 FFI)
pub struct PluginAPI {
    // 连接管理
    pub get_connections: fn() -> Vec<Connection>,
    pub get_connection: fn(id: &str) -> Option<Connection>,

    // 终端操作
    pub send_keys: fn(id: &str, keys: &str),
    pub get_selected_text: fn(id: &str) -> Option<String>,

    // 文件操作
    pub read_remote_file: fn(id: &str, path: &str) -> Result<Vec<u8>>,
    pub write_remote_file: fn(id: &str, path: &str, data: &[u8]) -> Result<()>,

    // 事件订阅
    pub on: fn(event: &str, callback: Box<dyn Fn(Value)>),
    pub emit: fn(event: &str, data: Value),
}
```

### 4.4 技术债务与架构改进

#### 4.4.1 迁移到 russh

**原因**:
- ssh2 依赖 OpenSSL，存在安全问题
- russh 是纯 Rust 实现，更安全
- OxideTerm 已验证 russh 的生产可用性

```toml
# Cargo.toml
[dependencies]
russh = { version = "0.54", default-features = false, features = ["ring", "rsa"] }
russh-sftp = "2"
```

#### 4.4.2 统一状态管理

**创建跨平台状态抽象**:

```rust
// core/state/
pub trait StateStore: Send + Sync {
    fn get<T: Serialize + for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T>;
    fn set<T: Serialize + for<'de> Deserialize<'de>>(&self, key: &str, value: T);
    fn subscribe(&self, key: &str, callback: Box<dyn Fn(Value)>);
}

// 各平台实现
#[cfg(target_os = "windows")]
pub type PlatformStore = EguiStateStore;

#[cfg(target_os = "linux")]
pub type PlatformStore = GtkStateStore;

#[cfg(target_os = "macos")]
pub type PlatformStore = SwiftStateStore;
```

#### 4.4.3 双平面架构

**长期目标**: 实现类似 OxideTerm 的双平面架构

```
当前架构:
┌─────────────────────┐
│    UI (Native)      │
│  ─────────────────  │
│   FFI → Core Rust   │
└─────────────────────┘

目标架构:
┌─────────────────────┐
│    UI (Native)      │
├──────────┬──────────┤
│ 控制平面  │ 数据平面  │
│ (FFI)    │ (WebSocket) │
├──────────┴──────────┤
│     Core Rust       │
└─────────────────────┘
```

---

## 五、实施路线图

### 5.1 短期 (Q2 2026)

| 里程碑 | 内容 | 参考模块 |
|--------|------|----------|
| M2.1 | 嵌入式终端基础 | OxideTerm xterm.js |
| M2.2 | 自动重连系统 | OxideTerm reconnect_orchestrator |
| M2.3 | SFTP 进度与拖拽 | OxideTerm sftp/transfer.rs |
| M2.4 | 迁移到 russh | OxideTerm SSH 模块 |

### 5.2 中期 (Q3-Q4 2026)

| 里程碑 | 内容 | 参考模块 |
|--------|------|----------|
| M3.1 | AI 助手基础 | OxideTerm ai/agent |
| M3.2 | RAG 知识库 | OxideTerm rag/ |
| M3.3 | 终端搜索与录制 | OxideTerm session/search.rs |
| M3.4 | Pro 版本后端 | OxideTerm 架构参考 |

### 5.3 长期 (2027)

| 里程碑 | 内容 | 参考模块 |
|--------|------|----------|
| M4.1 | 插件系统 | OxideTerm plugin/ |
| M4.2 | MCP 协议 | OxideTerm MCP 集成 |
| M4.3 | 远程 IDE | OxideTerm ide/ |
| M4.4 | 团队协作增强 | - |

---

## 六、差异化定位

### 6.1 EasySSH 独特优势

| 维度 | EasySSH | OxideTerm |
|------|---------|-----------|
| **原生体验** | ✅ 纯原生 UI | Web 技术 (Tauri) |
| **轻量级** | ✅ ~5MB Lite | ~10MB |
| **版本分层** | ✅ Lite/Standard/Pro | 单版本 |
| **企业功能** | ✅ 团队协作/SSO | 个人使用 |
| **开源协议** | ✅ MIT/Apache | GPL-3.0 |

### 6.2 发展策略

1. **保持原生优势**: 继续使用 egui/GTK4/SwiftUI，追求极致性能
2. **选择性借鉴**: 从 OxideTerm 吸收最佳实践，而非全盘照搬
3. **差异化竞争**:
   - 企业版功能 (团队协作、审计、SSO)
   - 三版本分层满足不同用户
   - 更轻量的 Lite 版本
4. **开放生态**: 插件系统支持社区扩展

---

## 七、参考资料

### 7.1 OxideTerm 文档

- [ARCHITECTURE.md](https://github.com/AnalyseDeCircuit/oxideterm/blob/main/docs/reference/ARCHITECTURE.md) - 架构设计
- [RAG_SYSTEM.md](https://github.com/AnalyseDeCircuit/oxideterm/blob/main/docs/reference/RAG_SYSTEM.md) - RAG 系统
- [PLUGIN_SYSTEM.md](https://github.com/AnalyseDeCircuit/oxideterm/blob/main/docs/reference/PLUGIN_SYSTEM.md) - 插件系统
- [SYSTEM_INVARIANTS.md](https://github.com/AnalyseDeCircuit/oxideterm/blob/main/docs/reference/SYSTEM_INVARIANTS.md) - 系统不变量
- [AICHAT.md](https://github.com/AnalyseDeCircuit/oxideterm/blob/main/docs/reference/AICHAT.md) - AI 集成

### 7.2 关键源码目录

```
oxideterm/
├── src-tauri/src/
│   ├── agent/         # 远程 Agent
│   ├── rag/           # RAG 系统
│   ├── session/       # 会话管理
│   ├── ssh/           # SSH 实现
│   ├── sftp/          # SFTP 实现
│   └── forwarding/    # 端口转发
├── agent/             # 独立 Agent 二进制
├── src/
│   ├── store/         # Zustand Stores
│   ├── components/
│   │   ├── ai/        # AI 组件
│   │   ├── terminal/  # 终端组件
│   │   └── sftp/      # SFTP 组件
│   └── locales/       # 11 种语言
└── docs/reference/    # 详细文档
```

---

*文档创建: 2026-04-03*
*最后更新: 2026-04-03*