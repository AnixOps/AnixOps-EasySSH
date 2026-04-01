# EasySSH API 设计

> REST API 与 WebSocket 接口设计规范
> 版本: 1.0 | 日期: 2026-04-01

---

## 目录

1. [API概览](#1-api概览)
2. [认证与授权](#2-认证与授权)
3. [REST API 端点](#3-rest-api-端点)
4. [WebSocket API](#4-websocket-api)
5. [数据模型](#5-数据模型)
6. [错误处理](#6-错误处理)
7. [版本策略](#7-版本策略)

---

## 1. API概览

### 1.1 API架构

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                          EasySSH API 架构                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                        API Gateway (Traefik)                           │  │
│   │                                                                         │  │
│   │  • 路由: /api/v1/* → API Server                                        │  │
│   │  • 路由: /ws/* → WebSocket Server                                      │  │
│   │  • 限流: 100 req/s per IP                                              │  │
│   │  • SSL/TLS termination                                                 │  │
│   │                                                                         │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                          │
│           ┌──────────────────────────┼──────────────────────────┐               │
│           │                          │                          │               │
│           ▼                          ▼                          ▼               │
│   ┌───────────────┐          ┌───────────────┐          ┌───────────────┐       │
│   │  REST API     │          │  WebSocket    │          │  Webhook      │       │
│   │  Server       │          │  Server       │          │  Handler      │       │
│   │  (Actix-web)  │          │  (Actix-web)  │          │  (Actix-web)  │       │
│   │               │          │               │          │               │       │
│   │ • CRUD操作    │          │ • 实时通知    │          │ • 外部集成    │       │
│   │ • 同步API     │          │ • 会话中继    │          │ • 事件回调    │       │
│   │ • 管理API     │          │ • 协作功能    │          │               │       │
│   └───────┬───────┘          └───────┬───────┘          └───────┬───────┘       │
│           │                          │                          │               │
│           └──────────────────────────┼──────────────────────────┘               │
│                                      │                                          │
│                                      ▼                                          │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                         Service Layer                                  │  │
│   │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        │  │
│   │  │  Auth   │  │  Team   │  │  Sync   │  │  Audit  │  │ Notify  │        │  │
│   │  │ Service │  │ Service │  │ Service │  │ Service │  │ Service │        │  │
│   │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘        │  │
│   │       └─────────────┴─────────────┴─────────────┴────────────────┘        │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                          │
│                                      ▼                                          │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                         Data Layer                                     │  │
│   │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐                     │  │
│   │  │PostgreSQL│  │  Redis  │  │  MinIO  │  │Elasticsea│                     │  │
│   │  │         │  │         │  │  (S3)   │  │  rch     │                     │  │
│   │  └─────────┘  └─────────┘  └─────────┘  └─────────┘                     │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 API使用场景

| 场景 | API类型 | 说明 |
|------|---------|------|
| **客户端同步** | REST + WS | 配置同步、实时通知 |
| **团队管理** | REST | CRUD操作、成员管理 |
| **审计查询** | REST | 日志查询、导出 |
| **SSO集成** | REST | SAML/OIDC回调 |
| **Webhook** | REST | 外部系统通知 |
| **实时协作** | WebSocket | 共享终端、屏幕共享 |

---

## 2. 认证与授权

### 2.1 认证流程

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        认证流程 (OAuth2 + JWT)                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│   客户端                              API Server                        IdP      │
│     │                                   │                               │       │
│     │ 1. 登录请求                        │                               │       │
│     ├───────────────────────────────────>│                               │       │
│     │ (email, password)                  │                               │       │
│     │                                   │                               │       │
│     │                                   │ 2. 验证凭据 (SSO时转发到IdP)    │       │
│     │                                   │───────────────────────────────>│       │
│     │                                   │                               │       │
│     │                                   │<───────────────────────────────│       │
│     │                                   │ (SAML/OIDC响应)               │       │
│     │                                   │                               │       │
│     │                                   │ 3. 查询/创建用户               │       │
│     │                                   │──────┐                        │       │
│     │                                   │      ▼                        │       │
│     │                                   │  ┌─────────┐                 │       │
│     │                                   │  │   DB    │                 │       │
│     │                                   │  └─────────┘                 │       │
│     │                                   │                               │       │
│     │ 4. 返回Token对                     │                               │       │
│     │<───────────────────────────────────│                               │       │
│     │ {access_token, refresh_token}      │                               │       │
│     │                                   │                               │       │
│     │ . . . (后续请求)                   │                               │       │
│     │                                   │                               │       │
│     │ 5. API请求 (带Token)               │                               │       │
│     ├───────────────────────────────────>│                               │       │
│     │ Authorization: Bearer <token>        │                               │       │
│     │                                   │                               │       │
│     │                                   │ 6. 验证JWT                    │       │
│     │                                   │    (本地验证，无需DB查询)        │       │
│     │                                   │                               │       │
│     │                                   │ 7. 检查权限                    │       │
│     │                                   │                               │       │
│     │ 8. 返回数据                        │                               │       │
│     │<───────────────────────────────────│                               │       │
│     │                                   │                               │       │
│     │ 9. Token即将过期                   │                               │       │
│     ├───────────────────────────────────>│                               │       │
│     │ POST /auth/refresh                 │                               │       │
│     │ (refresh_token)                    │                               │       │
│     │                                   │                               │       │
│     │                                   │ 10. 验证refresh_token         │       │
│     │                                   │──────┐                        │       │
│     │                                   │      ▼                        │       │
│     │                                   │  ┌─────────┐                 │       │
│     │                                   │  │  Redis  │                 │       │
│     │                                   │  │(Token)  │                 │       │
│     │                                   │  └─────────┘                 │       │
│     │                                   │                               │       │
│     │ 11. 新Token对                      │                               │       │
│     │<───────────────────────────────────│                               │       │
│     │                                   │                               │       │
└─────────────────────────────────────────────────────────────────────────────────┘

Token结构:
┌───────────────────────────────────────────────────────────────────────────┐
│  Access Token (JWT)                                                        │
│  ──────────────────────────────────────────────────────────────────────   │
│  Header: { "alg": "RS256", "typ": "JWT" }                                  │
│  Payload: {                                                                │
│    "sub": "user_uuid",                                                     │
│    "iss": "easyssh.pro",                                                   │
│    "aud": "api.easyssh.pro",                                               │
│    "exp": 1711929600,   // 15分钟                                          │
│    "iat": 1711928700,                                                      │
│    "team_id": "team_uuid",                                                 │
│    "role": "admin"                                                         │
│  }                                                                         │
│  Signature: RS256签名                                                      │
│                                                                            │
│  Refresh Token (Opaque)                                                    │
│  ──────────────────────────────────────────────────────────────────────   │
│  随机字符串，存储于Redis，有效期7天                                          │
│  用于获取新的Access Token                                                  │
└───────────────────────────────────────────────────────────────────────────┘
```

### 2.2 权限模型

```rust
// RBAC 权限定义
pub enum Permission {
    // 服务器权限
    ServerRead,
    ServerCreate,
    ServerUpdate,
    ServerDelete,
    ServerConnect,      // 连接服务器

    // 团队权限
    TeamRead,
    TeamUpdate,
    MemberInvite,
    MemberRemove,
    RoleAssign,

    // 审计权限
    AuditRead,
    AuditExport,
    AuditDelete,        // 仅Owner

    // 管理权限
    BillingRead,
    BillingUpdate,
    SettingsRead,
    SettingsUpdate,
}

pub enum Role {
    Owner,      // 全部权限
    Admin,      // 除删除团队外的全部权限
    Operator,   // 连接、读取、创建服务器
    Viewer,     // 只读
}

impl Role {
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            Role::Owner => vec![
                Permission::ServerRead, Permission::ServerCreate,
                Permission::ServerUpdate, Permission::ServerDelete, Permission::ServerConnect,
                Permission::TeamRead, Permission::TeamUpdate,
                Permission::MemberInvite, Permission::MemberRemove, Permission::RoleAssign,
                Permission::AuditRead, Permission::AuditExport, Permission::AuditDelete,
                Permission::BillingRead, Permission::BillingUpdate,
                Permission::SettingsRead, Permission::SettingsUpdate,
            ],
            Role::Admin => vec![
                Permission::ServerRead, Permission::ServerCreate,
                Permission::ServerUpdate, Permission::ServerConnect,
                Permission::TeamRead, Permission::TeamUpdate,
                Permission::MemberInvite, Permission::RoleAssign,
                Permission::AuditRead, Permission::AuditExport,
                Permission::BillingRead, Permission::SettingsRead,
            ],
            Role::Operator => vec![
                Permission::ServerRead, Permission::ServerCreate,
                Permission::ServerUpdate, Permission::ServerConnect,
                Permission::TeamRead,
            ],
            Role::Viewer => vec![
                Permission::ServerRead,
                Permission::TeamRead,
            ],
        }
    }
}
```

---

## 3. REST API 端点

### 3.1 API端点总览

| 模块 | 端点前缀 | 说明 |
|------|----------|------|
| 认证 | `/api/v1/auth/*` | 登录、注册、Token刷新 |
| 用户 | `/api/v1/users/*` | 个人信息、设置 |
| 团队 | `/api/v1/teams/*` | 团队管理、成员 |
| 服务器 | `/api/v1/servers/*` | 服务器CRUD |
| 分组 | `/api/v1/groups/*` | 分组管理 |
| 同步 | `/api/v1/sync/*` | 配置同步 |
| 审计 | `/api/v1/audit/*` | 审计日志 |
| Webhook | `/api/v1/webhooks/*` | 外部集成 |

### 3.2 认证端点

```yaml
# 登录
POST /api/v1/auth/login
Request:
  Content-Type: application/json
  Body:
    email: string        # 邮箱
    password: string     # 密码
    mfa_code?: string    # MFA验证码 (如果启用)

Response 200:
  Body:
    access_token: string   # JWT, 15分钟有效
    refresh_token: string  # 7天有效
    expires_in: number     # 秒
    user:
      id: string
      email: string
      name: string
      teams: TeamPreview[]

Response 401:
  Body:
    error: "invalid_credentials"
    message: "Invalid email or password"

---

# Token刷新
POST /api/v1/auth/refresh
Request:
  Content-Type: application/json
  Body:
    refresh_token: string

Response 200:
  Body:
    access_token: string
    refresh_token: string  # 轮换refresh token
    expires_in: number

Response 401:
  Body:
    error: "invalid_refresh_token"

---

# 登出
POST /api/v1/auth/logout
Headers:
  Authorization: Bearer <access_token>

Request:
  Body:
    all_devices?: boolean  # 是否在所有设备上登出

Response 204:
  # 无内容

---

# SSO登录 (SAML/OIDC发起)
GET /api/v1/auth/sso/:provider
Parameters:
  provider: "saml" | "oidc"
  team_id: string          # 团队ID

Response 302:
  Location: https://idp.example.com/...  # 重定向到IdP

---

# SSO回调
POST /api/v1/auth/sso/callback/:provider
Request:
  Content-Type: application/x-www-form-urlencoded
  Body: (SAMLResponse or code)

Response 200:
  Body:
    access_token: string
    refresh_token: string
    user: User
```

### 3.3 服务器端点

```yaml
# 列表查询 (分页)
GET /api/v1/servers
Headers:
  Authorization: Bearer <token>
  X-Team-ID: <team_id>    # 可选，不指定则为个人服务器

Parameters:
  page?: number = 1       # 页码
  limit?: number = 20     # 每页数量 (max 100)
  group_id?: string       # 分组筛选
  search?: string         # 搜索关键词
  sort?: "name" | "created_at" | "last_connected" = "name"
  order?: "asc" | "desc" = "asc"

Response 200:
  Body:
    data: Server[]
    pagination:
      page: number
      limit: number
      total: number
      total_pages: number
      has_more: boolean

---

# 获取详情
GET /api/v1/servers/:id
Response 200:
  Body: Server

Response 404:
  Body:
    error: "server_not_found"

---

# 创建服务器
POST /api/v1/servers
Request:
  Content-Type: application/json
  Body:
    name: string
    host: string
    port?: number = 22
    username: string
    auth_type: "password" | "key" | "agent"
    auth_data?: EncryptedPayload  # 加密后的认证信息
    group_id?: string
    tags?: string[]
    notes?: string

Response 201:
  Body: Server

Response 400:
  Body:
    error: "validation_error"
    details:
      - field: "host"
        message: "Invalid IP address or hostname"

---

# 更新服务器
PATCH /api/v1/servers/:id
Request:
  Body:  # 部分更新，只传需要修改的字段
    name?: string
    host?: string
    ...

Response 200:
  Body: Server

---

# 删除服务器
DELETE /api/v1/servers/:id
Response 204:

Response 403:
  Body:
    error: "permission_denied"
    message: "You don't have permission to delete this server"

---

# 批量操作
POST /api/v1/servers/batch
Request:
  Body:
    operation: "delete" | "move" | "tag"
    server_ids: string[]
    payload?:  # 操作特定数据
      group_id?: string   # for move
      tags?: string[]     # for tag

Response 200:
  Body:
    succeeded: number
    failed: number
    errors: BatchError[]
```

### 3.4 同步端点

```yaml
# 获取同步状态
GET /api/v1/sync/status
Response 200:
  Body:
    last_sync_at: datetime
    revision: number
    pending_changes: number
    conflicts: number

---

# 推送变更
POST /api/v1/sync/push
Request:
  Content-Type: application/json
  Body:
    base_revision: number       # 基于哪个版本
    changes: SyncChange[]       # 变更列表
    encrypted_payload: string   # 加密后的完整数据

Response 200:
  Body:
    new_revision: number
    conflicts: SyncConflict[]   # 如果有冲突

Response 409:  # 冲突
  Body:
    error: "sync_conflict"
    server_revision: number
    conflicts: SyncConflict[]

---

# 拉取变更
GET /api/v1/sync/pull
Parameters:
  since_revision: number   # 从哪个版本之后

Response 200:
  Body:
    revision: number
    changes: SyncChange[]
    encrypted_payload: string
    has_more: boolean

---

# 解决冲突
POST /api/v1/sync/resolve
Request:
  Body:
    conflicts: ConflictResolution[]
      - conflict_id: string
        resolution: "local" | "remote" | "merged"
        merged_data?: Server  # 如果resolution为merged

Response 200:
  Body:
    new_revision: number
```

### 3.5 审计端点

```yaml
# 查询审计日志
GET /api/v1/audit/logs
Parameters:
  page?: number = 1
  limit?: number = 50
  start_date?: datetime
  end_date?: datetime
  event_types?: string[]     # 筛选事件类型
  user_id?: string           # 筛选用户
  server_id?: string         # 筛选服务器

Response 200:
  Body:
    data: AuditLog[]
    pagination: Pagination
    summary:
      total_events: number
      unique_users: number
      event_breakdown:  # 按类型统计
        - type: "login"
          count: 150
        - type: "session_start"
          count: 450

---

# 导出审计日志
POST /api/v1/audit/export
Request:
  Body:
    format: "json" | "csv"
    start_date: datetime
    end_date: datetime
    filters?: AuditFilter

Response 202:
  Body:
    export_id: string
    status: "queued"
    estimated_completion: datetime

---

# 获取导出状态
GET /api/v1/audit/export/:export_id
Response 200:
  Body:
    export_id: string
    status: "queued" | "processing" | "completed" | "failed"
    download_url?: string   # 如果completed
    expires_at?: datetime
```

---

## 4. WebSocket API

### 4.1 连接管理

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        WebSocket 连接架构                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  客户端                              WebSocket Server                           │
│     │                                       │                                   │
│     │ 1. 连接建立                            │                                   │
│     │ ws://ws.easyssh.pro/v1/realtime         │                                   │
│     ├───────────────────────────────────────>│                                   │
│     │                                       │                                   │
│     │ 2. 认证握手                            │                                   │
│     │ { "type": "auth", "token": "..." }      │                                   │
│     ├───────────────────────────────────────>│                                   │
│     │                                       │ 3. 验证JWT                         │
│     │                                       │──────┐                             │
│     │                                       │      ▼                             │
│     │                                       │  ┌─────────┐                       │
│     │                                       │  │  Auth   │                       │
│     │                                       │  │ Service │                       │
│     │                                       │  └────┬────┘                       │
│     │                                       │       │                            │
│     │ 4. 连接确认                            │<──────┘                            │
│     │<───────────────────────────────────────│ { "type": "connected" }             │
│     │                                       │                                   │
│     │ 5. 订阅频道                            │                                   │
│     │ { "type": "subscribe", "channels":     │                                   │
│     │   ["team:123", "user:456"] }          │                                   │
│     ├───────────────────────────────────────>│                                   │
│     │                                       │                                   │
│     │ . . . (保持连接)                        │                                   │
│     │                                       │                                   │
│     │ 6. 收到推送                            │                                   │
│     │<───────────────────────────────────────│ { "type": "sync_update",         │
│     │                                       │   "channel": "team:123", ... }     │
│     │                                       │                                   │
│     │ 7. 心跳                                │                                   │
│     │ { "type": "ping" } ──────────────────>│                                   │
│     │<───────────────────────────────────────│ { "type": "pong" }               │
│     │                                       │                                   │
│     │ 8. 断开连接                            │                                   │
│     │ { "type": "disconnect" } ─────────────>│                                   │
│     │                                       │                                   │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 消息协议

```typescript
// WebSocket 消息类型定义

// 基础消息结构
interface WSMessage {
  id: string;           // 消息唯一ID
  type: MessageType;    // 消息类型
  timestamp: number;      // Unix timestamp (ms)
  payload: unknown;     // 消息载荷
}

type MessageType =
  // 连接管理
  | "auth"              // 认证请求
  | "auth_success"      // 认证成功
  | "auth_error"        // 认证失败
  | "ping" | "pong"     // 心跳
  | "subscribe"         // 订阅频道
  | "unsubscribe"       // 取消订阅
  | "connected"         // 连接确认

  // 同步相关
  | "sync_request"      // 请求同步
  | "sync_update"       // 同步更新通知
  | "sync_conflict"     // 同步冲突通知

  // 团队协作
  | "member_online"     // 成员上线
  | "member_offline"    // 成员离线
  | "activity_feed"     // 实时活动

  // 审计
  | "audit_alert"       // 审计告警 (高危操作)

  // 系统
  | "maintenance"       // 维护通知
  | "rate_limit";       // 限流警告

// 认证消息
interface AuthMessage {
  type: "auth";
  payload: {
    token: string;           // Access token
    device_id: string;       // 设备标识
    client_version: string;  // 客户端版本
    capabilities: string[]; // 支持的协议特性
  };
}

// 同步更新消息
interface SyncUpdateMessage {
  type: "sync_update";
  payload: {
    channel: string;         // 频道ID，如 "team:123"
    revision: number;        // 新版本号
    changes: SyncChange[];   // 变更列表
    summary: {
      servers_added: number;
      servers_updated: number;
      servers_deleted: number;
    };
  };
}

// 实时活动消息
interface ActivityMessage {
  type: "activity_feed";
  payload: {
    actor: UserPreview;     // 触发者
    action: string;          // 动作类型
    resource: ResourceRef;   // 操作对象
    metadata: Record<string, unknown>;
    team_id: string;
  };
}
```

---

## 5. 数据模型

### 5.1 核心模型

```rust
// 服务器模型
pub struct Server {
    pub id: String,                    // UUID v4
    pub name: String,                  // 显示名称
    pub host: String,                  // IP或域名
    pub port: u16,                     // SSH端口，默认22
    pub username: String,              // 登录用户名
    pub auth_type: AuthType,           // 认证类型
    pub auth_data_encrypted: Vec<u8>,  // 加密后的认证数据
    pub group_id: Option<String>,      // 所属分组
    pub tags: Vec<String>,             // 标签列表
    pub notes: Option<String>,         // 备注
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_connected_at: Option<DateTime<Utc>>,
    pub team_id: Option<String>,       // Pro版团队ID
    pub created_by: String,            // 创建者ID
}

pub enum AuthType {
    Password,
    PrivateKey,
    Agent,
}

// 分组模型
pub struct Group {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,     // 嵌套分组支持
    pub color: Option<String>,         // Hex颜色
    pub icon: Option<String>,          // 图标名称
    pub sort_order: i32,
    pub team_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

// 团队模型
pub struct Team {
    pub id: String,
    pub name: String,
    pub slug: String,                  // URL友好的标识
    pub owner_id: String,
    pub settings: TeamSettings,
    pub billing: BillingInfo,
    pub created_at: DateTime<Utc>,
    pub member_count: u32,
    pub server_count: u32,
}

pub struct TeamSettings {
    pub allow_public_sharing: bool,    // 是否允许公开分享
    pub require_mfa: bool,             // 是否强制MFA
    pub session_timeout_minutes: u32,  // 会话超时时间
    pub audit_retention_days: u32,   // 审计日志保留
    pub allowed_auth_methods: Vec<AuthType>,
}

// 审计日志模型
pub struct AuditLog {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub event_type: AuditEventType,
    pub resource_type: String,         // "server", "team", "member"
    pub resource_id: String,
    pub details: serde_json::Value,    // 事件详情
    pub ip_address: String,
    pub user_agent: String,
    pub severity: Severity,            // info, warning, critical
    pub created_at: DateTime<Utc>,
}

pub enum AuditEventType {
    LoginSuccess,
    LoginFailed,
    Logout,
    SessionStart,
    SessionEnd,
    CommandExecuted,
    ServerCreated,
    ServerUpdated,
    ServerDeleted,
    MemberInvited,
    MemberJoined,
    MemberRemoved,
    SettingsChanged,
}

// 同步变更模型
pub struct SyncChange {
    pub id: String,
    pub operation: ChangeOperation,    // create, update, delete
    pub entity_type: String,           // "server", "group"
    pub entity_id: String,
    pub revision: u64,
    pub timestamp: DateTime<Utc>,
    pub payload_encrypted: Vec<u8>,    // 加密的变更数据
    pub device_id: String,
}

pub struct SyncConflict {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub local_revision: u64,
    pub remote_revision: u64,
    pub local_data: Option<EncryptedPayload>,
    pub remote_data: Option<EncryptedPayload>,
}
```

### 5.2 API响应模型

```typescript
// 标准响应包装
interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: ApiError;
  meta?: ResponseMeta;
}

interface ApiError {
  code: string;           // 错误码
  message: string;        // 用户可读消息
  details?: unknown;      // 额外详情
  request_id: string;     // 用于追踪的请求ID
}

interface ResponseMeta {
  request_id: string;
  timestamp: string;      // ISO 8601
  pagination?: PaginationMeta;
}

interface PaginationMeta {
  page: number;
  limit: number;
  total: number;
  total_pages: number;
  has_more: boolean;
}

// 示例：服务器列表响应
interface ListServersResponse extends ApiResponse<{
  data: Server[];
}> {
  meta: {
    request_id: "req_abc123";
    timestamp: "2026-04-01T10:30:00Z";
    pagination: {
      page: 1;
      limit: 20;
      total: 156;
      total_pages: 8;
      has_more: true;
    };
  };
}
```

---

## 6. 错误处理

### 6.1 错误码体系

| HTTP状态码 | 错误码 | 说明 | 处理建议 |
|------------|--------|------|----------|
| **400** | `validation_error` | 请求参数验证失败 | 检查参数，参考`details` |
| | `bad_request` | 请求格式错误 | 检查JSON格式 |
| **401** | `unauthorized` | 未提供认证信息 | 提供有效Token |
| | `token_expired` | Token已过期 | 使用refresh_token刷新 |
| | `invalid_token` | Token无效 | 重新登录 |
| **403** | `permission_denied` | 权限不足 | 检查用户角色 |
| | `team_limit_reached` | 达到团队限制 | 升级套餐 |
| **404** | `not_found` | 资源不存在 | 检查ID是否正确 |
| | `endpoint_not_found` | API端点不存在 | 检查URL |
| **409** | `sync_conflict` | 同步冲突 | 解决冲突后重试 |
| | `duplicate_resource` | 资源重复 | 检查唯一性字段 |
| **429** | `rate_limit_exceeded` | 请求过于频繁 | 降低请求频率 |
| **500** | `internal_error` | 服务器内部错误 | 联系支持团队 |
| **503** | `service_unavailable` | 服务不可用 | 稍后重试 |
| | `maintenance_mode` | 维护中 | 等待维护完成 |

### 6.2 错误响应示例

```json
// 验证错误 (400)
{
  "success": false,
  "error": {
    "code": "validation_error",
    "message": "Request validation failed",
    "details": [
      {
        "field": "host",
        "message": "Invalid IP address or hostname format",
        "value": "999.999.999.999"
      },
      {
        "field": "port",
        "message": "Port must be between 1 and 65535",
        "value": 70000
      }
    ],
    "request_id": "req_abc123xyz"
  }
}

// 认证错误 (401)
{
  "success": false,
  "error": {
    "code": "token_expired",
    "message": "Access token has expired",
    "details": {
      "expired_at": "2026-04-01T10:00:00Z",
      "refreshable": true
    },
    "request_id": "req_def456uvw"
  }
}

// 权限错误 (403)
{
  "success": false,
  "error": {
    "code": "permission_denied",
    "message": "You don't have permission to delete this server",
    "details": {
      "resource": "server",
      "resource_id": "srv_abc123",
      "required_permission": "server:delete",
      "your_role": "operator"
    },
    "request_id": "req_ghi789rst"
  }
}

// 同步冲突 (409)
{
  "success": false,
  "error": {
    "code": "sync_conflict",
    "message": "Sync conflict detected. Please resolve conflicts and retry.",
    "details": {
      "server_revision": 156,
      "your_revision": 154,
      "conflicts": [
        {
          "id": "conf_001",
          "entity_type": "server",
          "entity_id": "srv_abc123",
          "field": "name",
          "local_value": "Production Web",
          "remote_value": "Web Server Prod"
        }
      ]
    },
    "request_id": "req_jkl012opq"
  }
}

// 限流错误 (429)
{
  "success": false,
  "error": {
    "code": "rate_limit_exceeded",
    "message": "Too many requests. Please slow down.",
    "details": {
      "limit": 100,
      "window": "1m",
      "retry_after": 45
    },
    "request_id": "req_mno345fgh"
  }
}
```

---

## 7. 版本策略

### 7.1 API版本管理

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        API 版本管理策略                                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  URL 版本控制: /api/v1/, /api/v2/                                              │
│                                                                                 │
│  版本生命周期:                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  当前版本 (v1)    ────────►  稳定版本 (v1)   ────────►  弃用版本       │   │
│  │  • 新功能开发       12个月        • 仅修复bug       6个月      • 停止支持 │   │
│  │  • 可能breaking                 • 保持兼容                   • 返回410  │   │
│  │                                                                             │   │
│  │  同时支持版本数: 2 (当前 + 前一个)                                          │   │
│  │  总支持周期: 18个月                                                        │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  弃用通知:                                                                       │
│  • 弃用时: 邮件通知 + 响应头 Warning                                           │
│  • 停止前30天: 额外日志告警                                                     │
│  • 停止后: 返回 410 Gone                                                       │
│                                                                                 │
│  响应头示例:                                                                     │
│  Warning: 299 - "API version v1 is deprecated. Please migrate to v2 by        │
│           2026-12-01. See https://docs.easyssh.pro/api/migration"             │
│  Sunset: Sat, 01 Dec 2026 00:00:00 GMT                                          │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 向后兼容规则

| 变更类型 | 兼容性 | 处理方式 |
|----------|--------|----------|
| 新增字段 | 兼容 | 直接添加，客户端忽略未知字段 |
| 新增可选参数 | 兼容 | 添加新query参数 |
| 新增端点 | 兼容 | 新增URL路径 |
| 修改字段类型 | 不兼容 | 新版本或新端点 |
| 删除字段 | 不兼容 | 新版本，旧版本返回空值 |
| 修改行为 | 不兼容 | 新版本，文档说明 |

### 7.3 版本协商

```
// 请求版本协商
GET /api/servers
Headers:
  Accept: application/json
  Api-Version: 2        // 显式请求版本

// 响应包含实际版本
HTTP/1.1 200 OK
Content-Type: application/json
Api-Version: 1          // 实际返回版本 (如果请求的不可用)
Warning: 299 - "Requested API version 2 not available, falling back to v1"
```

---

## 附录

### A. OpenAPI 规范

完整的 OpenAPI 3.0 规范文件: `openapi.yaml`

```yaml
# openapi.yaml 片段
openapi: 3.0.3
info:
  title: EasySSH Pro API
  description: |
    EasySSH Pro 云端服务 API。

    ## 认证
    所有API请求需要在 `Authorization` header 中提供 Bearer token:
    ```
    Authorization: Bearer <access_token>
    ```

    ## 分页
    列表接口支持分页，使用 `page` 和 `limit` 参数。
    响应包含 `meta.pagination` 字段。
  version: 1.0.0
  contact:
    name: EasySSH Support
    email: api@easyssh.pro

servers:
  - url: https://api.easyssh.pro
    description: Production
  - url: https://api-staging.easyssh.pro
    description: Staging

security:
  - bearerAuth: []

paths:
  /api/v1/servers:
    get:
      summary: 获取服务器列表
      operationId: listServers
      parameters:
        - name: page
          in: query
          schema:
            type: integer
            default: 1
        - name: limit
          in: query
          schema:
            type: integer
            default: 20
            maximum: 100
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ServerListResponse'

components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT

  schemas:
    Server:
      type: object
      required: [id, name, host, port, username, auth_type]
      properties:
        id:
          type: string
          format: uuid
        name:
          type: string
          maxLength: 100
        host:
          type: string
          pattern: '^[a-zA-Z0-9.-]+$'
        port:
          type: integer
          minimum: 1
          maximum: 65535
        username:
          type: string
          maxLength: 32
        auth_type:
          type: string
          enum: [password, key, agent]
        # ... more fields
```

### B. SDK与工具

| 语言 | 包名 | 安装 |
|------|------|------|
| TypeScript | `@easyssh/api-client` | `npm install @easyssh/api-client` |
| Python | `easyssh-api` | `pip install easyssh-api` |
| Rust | `easyssh-api` | `cargo add easyssh-api` |
| Go | `github.com/easyssh/api-go` | `go get github.com/easyssh/api-go` |

### C. 参考文档

- [系统架构](./system-architecture.md)
- [数据流设计](./data-flow.md)
- [部署架构](./deployment.md)
- [开发者文档](https://docs.easyssh.pro/developers)
