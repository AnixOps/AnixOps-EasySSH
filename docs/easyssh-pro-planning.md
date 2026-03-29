# EasySSH Pro 版本规划

> 团队SSH协作平台 - 商用级

---

## 1. 版本定位

### 核心价值
- **团队协作**：共享配置 + 权限管理
- **安全合规**：审计日志 + SSO
- **高可用**：企业级稳定性

### 目标用户
- IT团队/运维部门（5-500人）
- 需要集中管理服务器访问的企业
- 有合规要求的金融机构/医疗/政府

### 定价策略
- **月付**: $19.99/人/月
- **年付**: $199/人/年（省17%）
- **最低5席位**
- **企业定制**: 联系销售

---

## 2. 功能规格

### Standard版所有功能 + 以下增量

| 模块 | 功能 | 优先级 |
|------|------|--------|
| **团队** | 团队创建/管理 | P0 |
| **团队** | 成员邀请(RBAC) | P0 |
| **团队** | 共享服务器分组 | P0 |
| **团队** | 团队Snippets | P1 |
| **审计** | 会话审计录制 | P0 |
| **审计** | 命令执行记录 | P0 |
| **审计** | 文件传输记录 | P1 |
| **审计** | 登录事件 | P0 |
| **SSO** | SAML 2.0 | P1 |
| **SSO** | OIDC | P1 |
| **SSO** | LDAP/AD | P2 |
| **协作** | 实时活动Feed | P2 |
| **协作** | 屏幕共享会话 | P3 |
| **GPU** | NVIDIA GPU监控 | P1 |
| **告警** | 自定义告警规则 | P1 |
| **告警** | 邮件/钉钉通知 | P2 |

---

## 3. 系统架构

### Pro架构概览

```
┌──────────────────────────────────────────────────────────────────────────┐
│                           EasySSH Pro 客户端                             │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                    React Frontend (Standard UI)                     │  │
│  │  ┌──────────┐  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐  │  │
│  │  │ Sidebar+ │  │  SplitView  │  │   Audit    │  │   Team      │  │  │
│  │  │ Servers  │  │ (Terminal)  │  │   Panel    │  │   Panel     │  │  │
│  │  └──────────┘  └─────────────┘  └─────────────┘  └──────────────┘  │  │
│  └────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ HTTPS/WSS
                                    ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                         EasySSH Pro Cloud                               │
│                                                                          │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐            │
│  │   API Gateway  │  │  WebSocket     │  │   Auth        │            │
│  │   (Actix-web)  │  │   Server       │  │   Service     │            │
│  └───────┬────────┘  └───────┬────────┘  └───────┬────────┘            │
│          │                    │                   │                     │
│  ┌───────┴────────────────────┴───────────────────┴────────┐            │
│  │                    Core Services                           │            │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │            │
│  │  │  Team    │  │  Audit   │  │  Sync   │  │  Notify  │  │            │
│  │  │  Service │  │  Service │  │  Service│  │  Service │  │            │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │            │
│  └───────────────────────────────────────────────────────────┘            │
│          │                    │                    │                     │
│  ┌───────┴────────────────────┴────────────────────┴────────┐           │
│  │                        Data Layer                           │           │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │           │
│  │  │PostgreSQL│  │  Redis   │  │   S3     │  │   SMTP   │  │           │
│  │  │(主数据)   │  │(会话/缓存)│  │(审计日志)│  │(通知)    │  │           │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │           │
│  └───────────────────────────────────────────────────────────┘           │
└──────────────────────────────────────────────────────────────────────────┘
```

### Pro Backend服务设计

```rust
// Pro Backend 技术栈
// - Runtime: tokio + actix-web
// - Database: PostgreSQL (sqlx)
// - Cache: Redis
// - Storage: S3兼容 (minio-local for dev)
// - Auth: JWT + Refresh Token

// 目录结构
pro-backend/
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs
│   │   └── migrations/
│   ├── services/
│   │   ├── auth/
│   │   ├── team/
│   │   ├── audit/
│   │   ├── sync/
│   │   └── notification/
│   ├── api/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── teams.rs
│   │   └── audit.rs
│   └── ws/
│       └── mod.rs
├── Dockerfile
└── docker-compose.yml
```

---

## 4. 审计系统设计

### 审计事件类型

```rust
enum AuditEvent {
    // 认证事件
    Login { user_id, team_id, ip, user_agent },
    Logout { user_id },
    LoginFailed { user_id, reason, ip },

    // 会话事件
    SessionStart { session_id, server_id },
    SessionEnd { session_id, duration },
    Session recording stored { session_id, storage_path },

    // 命令事件
    CommandExecuted { session_id, command, exit_code },

    // 文件传输
    FileUploaded { session_id, path, size },
    FileDownloaded { session_id, path, size },

    // 管理事件
    ServerAdded { server_id, team_id },
    ServerRemoved { server_id },
    MemberInvited { team_id, email, role },
    MemberRemoved { team_id, user_id },
}

impl AuditService {
    pub async fn log(&self, event: AuditEvent) -> Result<()> {
        // 异步写入，不阻塞主流程
        self.events.push(event).await;

        // 实时告警 (某些高危操作)
        if event.is_critical() {
            self.notify_admins(event).await?;
        }
    }
}
```

### 会话录制

```rust
// 会话录制存储格式
struct SessionRecording {
    id: Uuid,
    team_id: Uuid,
    server_id: Uuid,
    user_id: Uuid,

    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,

    // 终端回放数据 (asciicast格式)
    // https://asciinema.org/
    playback_data: S3Path,

    // 关键命令标记
    command_markers: Vec<CommandMarker>,
}

struct CommandMarker {
    timestamp: Duration,
    command: String,
    output_preview: String,
    tags: Vec<String>, // "sudo", "delete", "config" 等
}
```

---

## 5. 权限模型 (RBAC)

```rust
// 角色定义
enum Role {
    Owner,      // 团队所有者，全部权限
    Admin,      // 管理员，团队管理+服务器管理
    Operator,   // 操作员，连接服务器
    Viewer,     // 查看者，仅查看
}

struct Permission {
    // 服务器权限
    can_connect: bool,
    can_manage_server: bool,

    // 团队权限
    can_invite: bool,
    can_remove_member: bool,
    can_change_role: bool,

    // 审计权限
    can_view_audit_log: bool,
    can_view_recordings: bool,
}

impl Role {
    fn permissions(&self) -> Permission {
        match self {
            Role::Owner => Permission::all(),
            Role::Admin => Permission {
                can_connect: true,
                can_manage_server: true,
                can_invite: true,
                can_remove_member: false,
                can_change_role: false,
                can_view_audit_log: true,
                can_view_recordings: true,
            },
            Role::Operator => Permission {
                can_connect: true,
                can_manage_server: false,
                can_invite: false,
                can_remove_member: false,
                can_change_role: false,
                can_view_audit_log: false,
                can_view_recordings: false,
            },
            Role::Viewer => Permission {
                can_connect: false,
                can_manage_server: false,
                can_invite: false,
                can_remove_member: false,
                can_change_role: false,
                can_view_audit_log: true,
                can_view_recordings: false,
            },
        }
    }
}
```

---

## 6. SSO集成

### SAML 2.0流程

```
1. 用户访问 EasySSH Pro
           │
           ▼
2. 重定向到企业IdP (Okta/Azure AD/OneLogin)
           │
           ▼
3. 用户在IdP完成认证
           │
           ▼
4. IdP返回SAML Assertion
           │
           ▼
5. EasySSH Pro验证Assertion并创建会话
           │
           ▼
6. 同步用户信息和团队权限
```

### 支持的IdP

| IdP | 状态 | 配置难度 |
|-----|------|----------|
| Okta | P1 | ★★☆☆☆ |
| Azure AD | P1 | ★★☆☆☆ |
| Google Workspace | P1 | ★☆☆☆☆ |
| OneLogin | P2 | ★★☆☆☆ |
| Ping Identity | P2 | ★★★☆☆ |
| LDAP/AD | P2 | ★★★☆☆ |

---

## 7. 数据流设计

### 配置同步流程 (E2EE)

```
用户A 修改服务器配置
        │
        ▼
客户端加密 (用户密钥)
        │
        ▼
发送加密数据到Pro Backend
        │
        ▼
Pro Backend 存储 (无法解密)
        │
        ▼
其他设备拉取加密数据
        │
        ▼
客户端解密 (用户密钥)
        │
        ▼
用户B 看到同步后的配置
```

### 密钥派生

```rust
// 使用Argon2id从用户主密码派生加密密钥
fn derive_key(master_password: &str, salt: &[u8]) -> [u8; 32] {
    Argon2id::default()
        .hash_password(master_password.as_bytes(), salt)
        .unwrap()
        .hash.unwrap()
        .as_bytes()
        .try_into()
        .unwrap()
}

// 端到端加密
fn encrypt_for_sync(data: &[u8], user_key: &[u8]) -> EncryptedPayload {
    let cipher = Aes256Gcm::new(user_key.into());
    let nonce = Nonce::from_slice(rand::bytes(12).as_slice());

    EncryptedPayload {
        nonce: nonce.to_vec(),
        ciphertext: cipher.encrypt(nonce, data).unwrap(),
    }
}
```

---

## 8. 部署架构

### 开发环境
```
docker-compose up
├── easyssh-pro (Backend)
├── postgres (Database)
├── redis (Cache)
├── minio (S3)
└── mailhog (SMTP mock)
```

### 生产环境 (推荐K8s)

```yaml
# pro-backend-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-pro-backend
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: backend
          image: easyssh/pro-backend:latest
          env:
            - DATABASE_URL
            - REDIS_URL
            - S3_BUCKET
            - JWT_SECRET
          resources:
            requests:
              memory: "256Mi"
              cpu: "250m"
            limits:
              memory: "512Mi"
              cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: easyssh-pro-backend
spec:
  type: ClusterIP
  ports:
    - port: 8080
  selector:
    app: easyssh-pro-backend
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: easyssh-pro
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  rules:
    - host: api.easyssh.pro
      http:
        paths:
          - path: /
            backend:
              service:
                name: easyssh-pro-backend
                port:
                  number: 8080
  tls:
    - hosts:
        - api.easyssh.pro
      secretName: easyssh-pro-tls
```

---

## 9. 监控与告警

### Pro版应用内监控

| 指标 | 告警阈值 | 通知方式 |
|------|----------|----------|
| API错误率 | > 1% | 邮件+钉钉 |
| P99延迟 | > 500ms | 邮件 |
| 数据库连接 | > 80% | 钉钉 |
| S3存储 | > 70% | 邮件 |
| CPU使用率 | > 90% | 邮件+钉钉 |

### 客户端上报

```rust
// 客户端使用体验监控
struct Telemetry {
    app_version: String,
    os: String,

    // 性能指标
    startup_time_ms: u64,
    memory_usage_mb: u64,
    crash_count: u32,

    // 功能使用统计
    sessions_created: u32,
    commands_executed: u64,
    files_transferred: u64,
}
```
