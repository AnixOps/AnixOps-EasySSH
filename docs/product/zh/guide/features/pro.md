# Pro 版功能详解

EasySSH Pro 是团队协作平台，提供团队管理、RBAC 权限控制、审计日志和 SSO 集成。

## 产品定位

```
┌──────────────────────────────────────────────────────────────┐
│                      EasySSH Pro                             │
│                    团队协作控制台                             │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  核心价值：团队管理 + 合规审计 + SSO 集成                     │
│                                                              │
│  • 集中式团队服务器配置管理                                   │
│  • 基于角色的细粒度访问控制                                   │
│  • 完整的操作审计日志                                         │
│  • 企业级 SSO (SAML/OIDC) 集成                               │
│  • 共享 Snippets 和配置模板                                   │
│  • 自动化审批工作流                                           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## 架构概览

```
┌──────────────────────────────────────────────────────────────┐
│                       客户端层                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                    │
│  │ 用户 A   │  │ 用户 B   │  │ 用户 C   │                    │
│  │ Standard │  │ Standard │  │ Standard │                    │
│  │ / Pro    │  │ / Pro    │  │ / Pro    │                    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                    │
│       │             │             │                          │
│       └─────────────┴─────────────┘                          │
│                     │                                        │
│              ┌──────┴──────┐                                 │
│              │   WebSocket  │                                │
│              │   E2EE Sync   │                                │
│              └──────┬──────┘                                 │
└─────────────────────┼────────────────────────────────────────┘
                      │
┌─────────────────────┼────────────────────────────────────────┐
│                     ▼                                        │
│                  Pro 服务端                                   │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  API Gateway (REST + GraphQL)                            │  │
│  ├────────────────────────────────────────────────────────┤  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │  │
│  │  │ Team     │  │ RBAC     │  │ Audit    │  │ Sync   │  │  │
│  │  │ Service  │  │ Service  │  │ Service  │  │ Service│  │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └────────┘  │  │
│  ├────────────────────────────────────────────────────────┤  │
│  │  ┌──────────┐  ┌──────────┐  ┌────────────────────┐  │  │
│  │  │PostgreSQL│  │  Redis   │  │    S3/MinIO        │  │  │
│  │  │  (数据)   │  │ (缓存/队列)│  │  (文件/备份)      │  │  │
│  │  └──────────┘  └──────────┘  └────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              SSO 集成 (SAML/OIDC)                       │  │
│  │     Okta / Azure AD / Google Workspace / 企业 AD      │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## 团队管理

### 创建团队

```bash
# 通过管理后台
easyssh admin team create \
  --name "Engineering" \
  --slug "eng" \
  --admin "admin@company.com"

# 或通过 API
curl -X POST https://easyssh.company.com/api/v1/teams \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "Engineering",
    "settings": {
      "require_2fa": true,
      "session_timeout": 3600
    }
  }'
```

### 邀请成员

```bash
# 批量邀请
easyssh admin member invite \
  --team "Engineering" \
  --emails "alice@company.com,bob@company.com" \
  --role "developer"

# 生成邀请链接
easyssh admin member invite-link \
  --team "Engineering" \
  --role "developer" \
  --expires 7d
```

### 成员角色

| 角色 | 权限 |
|------|------|
| **Owner** | 团队设置、账单、删除团队 |
| **Admin** | 成员管理、角色分配、全局配置 |
| **Manager** | 服务器管理、分组管理、查看审计 |
| **Developer** | 连接服务器、查看配置、管理个人密钥 |
| **Observer** | 只读访问、查看日志、不能连接 |

### 部门/项目组

```
团队: Engineering
├── 部门: Platform
│   ├── 组: Infrastructure
│   └── 组: DevOps
├── 部门: Backend
│   ├── 组: API Team
│   └── 组: Data Team
└── 部门: Frontend
    ├── 组: Web
    └── 组: Mobile
```

```bash
# 创建部门
easyssh admin department create \
  --team "Engineering" \
  --name "Platform"

# 创建组并分配资源
easyssh admin group create \
  --department "Platform" \
  --name "Infrastructure" \
  --servers "prod-*,staging-*"
```

## RBAC 权限系统

### 资源类型

| 资源 | 说明 |
|------|------|
| `server` | 服务器连接配置 |
| `group` | 服务器分组 |
| `snippet` | 命令片段 |
| `key` | SSH 密钥 |
| `session` | 活动会话 |
| `audit_log` | 审计日志 |
| `team_config` | 团队设置 |

### 权限定义

```json
{
  "role": "developer",
  "permissions": [
    {
      "resource": "server",
      "actions": ["read", "connect"],
      "conditions": {
        "tag:environment": "development,staging",
        "not": {
          "tag:environment": "production"
        }
      }
    },
    {
      "resource": "snippet",
      "actions": ["read", "execute"]
    },
    {
      "resource": "key",
      "actions": ["read", "use"],
      "conditions": {
        "owned_by": "self"
      }
    }
  ]
}
```

### 策略模板

```bash
# 生产环境保护策略
easyssh admin policy create \
  --name "Production Protection" \
  --rules '{
    "resource": "server",
    "tag:environment": "production",
    "actions": {
      "connect": {
        "require_approval": true,
        "require_2fa": true,
        "session_recording": true
      }
    }
  }'

# 工作时间限制
easyssh admin policy create \
  --name "Working Hours" \
  --rules '{
    "time_restrictions": {
      "allowed_hours": "09:00-18:00",
      "timezone": "Asia/Shanghai",
      "weekdays_only": true
    }
  }'
```

### 动态权限

```yaml
# 基于属性的访问控制 (ABAC)
rules:
  - if:
      user.department: "DevOps"
      server.tag.criticality: "high"
    then:
      allow: true
      require:
        - approval_from: "manager"
        - session_recording: true

  - if:
      user.role: "contractor"
      server.tag.environment: "production"
    then:
      allow: false
```

## 审计日志

### 记录内容

```json
{
  "event_id": "evt_1234567890",
  "timestamp": "2026-01-15T10:30:00Z",
  "event_type": "session.connect",
  "severity": "info",
  "actor": {
    "type": "user",
    "id": "usr_123",
    "email": "alice@company.com",
    "ip": "192.168.1.100",
    "user_agent": "EasySSH/1.0.0"
  },
  "target": {
    "type": "server",
    "id": "srv_456",
    "name": "prod-web-01"
  },
  "context": {
    "session_id": "sess_789",
    "connection_method": "key",
    "mfa_verified": true,
    "via_jump_host": "bastion-01"
  },
  "result": {
    "success": true,
    "duration_ms": 1500
  }
}
```

### 事件类型

| 类别 | 事件 |
|------|------|
| **认证** | login, logout, mfa_verify, session_refresh |
| **连接** | session.connect, session.disconnect, session.timeout |
| **配置** | server.create, server.update, server.delete |
| **权限** | policy.update, role.assign, permission.denied |
| **密钥** | key.upload, key.use, key.revoke |
| **团队** | member.invite, member.join, member.remove |
| **系统** | config.change, backup.complete, alert.trigger |

### 审计查询

```bash
# 查询用户活动
easyssh audit query \
  --actor "alice@company.com" \
  --from "2026-01-01" \
  --to "2026-01-31" \
  --type "session.connect"

# 查询生产环境访问
easyssh audit query \
  --target "tag:environment=production" \
  --type "session.connect" \
  --group-by day

# 导出审计报告
easyssh audit export \
  --from "2026-01-01" \
  --to "2026-01-31" \
  --format csv \
  --output audit-report.csv
```

### 实时监控

```bash
# 实时告警
easyssh admin alert create \
  --name "Suspicious Activity" \
  --condition '{
    "event": "session.connect",
    "target.tag.environment": "production",
    "actor.not_in": "oncall_team",
    "time.not_in_business_hours": true
  }' \
  --action "notify:security@company.com,suspend_session"
```

## SSO 集成

### SAML 配置

**Okta 示例：**

```yaml
# EasySSH 配置
sso:
  provider: saml
  config:
    entrypoint: "https://company.okta.com/app/easyssh/sso/saml"
    issuer: "easyssh"
    cert: |
      -----BEGIN CERTIFICATE-----
      ...
      -----END CERTIFICATE-----
    attribute_mapping:
      email: "user.email"
      name: "user.firstName user.lastName"
      groups: "groups"

  role_mapping:
    "Engineering": "developer"
    "DevOps": "manager"
    "IT-Security": "admin"
```

**Azure AD 示例：**

```yaml
sso:
  provider: saml
  config:
    entrypoint: "https://login.microsoftonline.com/{tenant}/saml2"
    issuer: "https://easyssh.company.com"
    cert: "/path/to/azure-cert.pem"
```

### OIDC 配置

**Google Workspace：**

```yaml
sso:
  provider: oidc
  config:
    issuer: "https://accounts.google.com"
    client_id: "YOUR_CLIENT_ID"
    client_secret: "YOUR_CLIENT_SECRET"
    scopes: ["openid", "email", "profile"]
```

**通用 OIDC：**

```yaml
sso:
  provider: oidc
  config:
    issuer: "https://auth.company.com"
    client_id: "easyssh"
    client_secret: "..."
    authorization_endpoint: "/oauth2/auth"
    token_endpoint: "/oauth2/token"
    userinfo_endpoint: "/userinfo"
```

### 自动配置 (SCIM)

```yaml
# Okta SCIM 配置
scim:
  enabled: true
  endpoint: "/scim/v2"
  token: "scim-token-here"

  provisioning:
    create_user: true
    update_user: true
    deactivate_user: true

  mapping:
    userName: "email"
    name.formatted: "displayName"
    groups: "groups"
```

## 共享资源

### 共享服务器配置

```bash
# 创建共享服务器
easyssh admin server create \
  --name "Production DB" \
  --host "db.prod.internal" \
  --shared \
  --visibility "team" \
  --allowed-groups "DevOps,Backend"

# 分级可见性
easyssh admin server update \
  --id "srv_123" \
  --visibility "department" \
  --department "Platform"
```

### Snippets

```bash
# 创建共享 Snippet
easyssh snippet create \
  --name "Deploy App" \
  --content "cd /app && git pull && docker-compose up -d" \
  --shared \
  --tags "deploy,docker"

# 使用 Snippet
easyssh snippet run "Deploy App" --on "prod-web-01"
```

### 配置模板

```yaml
# 服务器配置模板
templates:
  - name: "Web Server"
    description: "Standard web server configuration"
    config:
      port: 22
      auth_type: "key"
      jump_host: "bastion"
      tags:
        - "type:web"
    variables:
      - name: "hostname"
        required: true
      - name: "environment"
        options: ["dev", "staging", "prod"]
```

```bash
# 从模板创建服务器
easyssh server create-from-template \
  --template "Web Server" \
  --vars 'hostname=web-05,environment=staging'
```

## 审批工作流

### 配置审批流程

```yaml
workflows:
  - name: "Production Access"
    trigger:
      event: "session.connect"
      conditions:
        - "target.tag.environment == 'production'"
    steps:
      - type: "approval"
        approvers:
          - role: "manager"
          - user: "team-lead@company.com"
        timeout: 30m

      - type: "notification"
        channels:
          - "slack:#security-alerts"
          - "email:security@company.com"

      - type: "condition"
        if: "time.hour not in 9..18"
        then:
          - type: "require_mfa"
          - type: "start_recording"
```

```bash
# 提交访问申请
easyssh request access \
  --server "prod-db-01" \
  --reason "Database migration for ticket PROJ-123" \
  --duration 4h

# 查看待审批请求
easyssh request list --pending

# 审批请求
easyssh request approve "req_123" --comment "Approved for migration"
```

## 会话管理

### 会话留痕

```bash
# 启用会话录制
easyssh admin config set session.recording.enabled true
easyssh admin config set session.recording.retention 90d

# 查看录制
easyssh session recording play "sess_123"
easyssh session recording export "sess_123" --output session.cast
```

### 实时会话监控

```
管理员面板:
┌─────────────────────────────────────────────────────────┐
│  🔴 实时监控 - 3 个活动会话                               │
├─────────────────────────────────────────────────────────┤
│  User           Server           Connected   Status       │
│  ─────────────────────────────────────────────────────  │
│  alice@co...    prod-web-01      10:30       🟢 正常     │
│  bob@comp...    prod-db-01       10:35       🟡 待观察   │
│  carol@co...    staging-web      10:40       🟢 正常     │
├─────────────────────────────────────────────────────────┤
│  [查看] [强制断开] [开始录制] [标记]                      │
└─────────────────────────────────────────────────────────┘
```

### 会话策略

```yaml
session_policies:
  max_concurrent: 5
  max_duration: 8h
  idle_timeout: 30m

  restrictions:
    - condition: "user.role == 'contractor'"
      max_duration: 4h
      require_approval: true

    - condition: "target.tag.criticality == 'high'"
      require_recording: true
      allow_file_transfer: false
```

## 部署架构

### 单节点部署

```yaml
# docker-compose.yml
version: '3.8'
services:
  easyssh:
    image: easyssh/pro:latest
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://easyssh:${DB_PASSWORD}@db:5432/easyssh
      - REDIS_URL=redis://redis:6379
      - JWT_SECRET=${JWT_SECRET}
      - ENCRYPTION_KEY=${ENCRYPTION_KEY}
    depends_on:
      - db
      - redis

  db:
    image: postgres:15-alpine
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine

volumes:
  postgres_data:
```

### Kubernetes 部署

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-pro
spec:
  replicas: 3
  selector:
    matchLabels:
      app: easyssh-pro
  template:
    metadata:
      labels:
        app: easyssh-pro
    spec:
      containers:
        - name: easyssh
          image: easyssh/pro:latest
          ports:
            - containerPort: 8080
          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: easyssh-secrets
                  key: database-url
            - name: REDIS_URL
              valueFrom:
                secretKeyRef:
                  name: easyssh-secrets
                  key: redis-url
```

### 高可用架构

```
                    ┌─────────────┐
                    │   Load      │
                    │  Balancer   │
                    │   (HAProxy) │
                    └──────┬──────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
    ┌──────┴──────┐ ┌──────┴──────┐ ┌──────┴──────┐
    │  EasySSH    │ │  EasySSH    │ │  EasySSH    │
    │  Instance 1 │ │  Instance 2 │ │  Instance 3 │
    └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
           │               │               │
           └───────────────┼───────────────┘
                           │
    ┌──────────────────────┴──────────────────────┐
    │                                             │
    │  PostgreSQL Primary-Replica + Redis Cluster │
    │                                             │
    └─────────────────────────────────────────────┘
```

## 管理命令

```bash
# 团队管理
easyssh admin team create|list|update|delete
easyssh admin member invite|list|update|remove
easyssh admin department create|list|update|delete
easyssh admin group create|list|update|delete

# 权限管理
easyssh admin role create|list|update|delete
easyssh admin policy create|list|update|delete
easyssh admin permission grant|revoke|check

# 审计
easyssh admin audit query|export|archive
easyssh admin alert create|list|update|delete

# 配置
easyssh admin config get|set|list
easyssh admin backup create|list|restore
easyssh admin maintenance on|off|status

# 监控
easyssh admin stats sessions|users|servers
easyssh admin report generate --type usage --period monthly
```

## 安全合规

### 认证方式

| 方式 | 支持 | 说明 |
|------|:----:|------|
| 密码 | ✅ | 仅用于本地账户 |
| SSH 密钥 | ✅ | 推荐用于服务器连接 |
| TOTP/HOTP | ✅ | 双因素认证 |
| WebAuthn | ✅ | 硬件安全密钥 |
| SAML | ✅ | 企业 SSO |
| OIDC | ✅ | 现代 SSO |
| LDAP | ✅ | 传统目录服务 |

### 合规认证

- **SOC 2 Type II**: 服务组织控制
- **ISO 27001**: 信息安全管理
- **GDPR**: 数据保护合规
- **HIPAA**: 医疗数据合规（可选）

### 数据安全

```
传输加密:
- TLS 1.3 (外部通信)
- WebSocket Secure (WSS)
- 内部 mTLS (服务间)

存储加密:
- AES-256-GCM 数据库加密
- 字段级加密 (密钥、密码)
- 备份加密 (GPG)

密钥管理:
- HSM 支持 (AWS KMS, Azure Key Vault)
- 定期密钥轮换
- 密钥访问审计
```

## 定价与许可

### 订阅层级

| 层级 | 用户/月 | 功能 |
|------|--------:|------|
| **Team** | $19.9 | 最多 25 用户，基础 RBAC |
| **Business** | $14.9 | 最多 100 用户，高级审计 |
| **Enterprise** | 定制 | 无限用户，完整功能 |

### 附加组件

- **SSO 集成**: +$5/人/月
- **高级审计**: +$3/人/月
- **HSM 支持**: +$200/月
- **专属支持**: +$500/月

## 故障排查

### 连接服务端失败

```bash
# 检查服务端健康
curl https://easyssh.company.com/health

# 查看客户端日志
easyssh --verbose --log-level debug

# 重置服务端配置
easyssh config reset server
```

### 同步问题

```bash
# 强制全量同步
easyssh sync --force

# 解决冲突
easyssh sync --resolve-conflicts

# 查看同步状态
easyssh sync --status
```

## 最佳实践

### 团队配置建议

```
1. 强制启用 2FA
2. 生产环境启用会话录制
3. 配置最小权限原则
4. 定期审查访问权限
5. 设置异常活动告警
6. 定期导出审计日志
7. 配置自动备份
8. 制定灾难恢复计划
```

## 下一步

- [企业部署指南](/zh/deploy/enterprise)
- [SSO 配置详解](/zh/deploy/sso)
- [审计日志分析](/zh/guide/audit-analysis)
- [API 参考](/zh/api/pro/team)
