# EasySSH Pro Server

EasySSH Pro后端API服务 - 支持团队协作、审计日志和SSO集成的RESTful API。

## 功能特性

- **团队管理**: 创建团队、邀请成员、角色分配
- **RBAC权限系统**: 细粒度的角色和权限管理
- **审计日志**: 完整的操作审计和导出
- **SSO集成**: 支持SAML 2.0和OIDC
- **共享资源**: 服务器和代码片段共享
- **WebSocket通知**: 实时协作通知
- **双模式认证**: JWT和API Key
- **速率限制**: 防止API滥用
- **OpenAPI文档**: 自动生成的API文档

## 技术栈

- **Rust** + **Axum** - 高性能Web框架
- **SQLx** - 类型安全的数据库操作
- **Redis** - 缓存和会话存储
- **JWT** - 无状态认证
- **OpenAPI/Swagger** - API文档

## 快速开始

### 环境变量

复制`.env.example`到`.env`并配置：

```bash
# 服务器配置
HOST=0.0.0.0
PORT=8080

# 数据库
DATABASE_URL=sqlite:./pro_server.db

# Redis
REDIS_URL=redis://127.0.0.1:6379

# 安全
JWT_SECRET=your-super-secret-jwt-key-change-in-production
JWT_EXPIRY_HOURS=24
REFRESH_TOKEN_EXPIRY_DAYS=7
ENCRYPTION_KEY=your-32-byte-encryption-key-here!

# 速率限制
RATE_LIMIT_REQUESTS=100
RATE_LIMIT_WINDOW_SECS=60

# 邮件配置 (用于邀请)
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=your-username
SMTP_PASSWORD=your-password
SMTP_FROM=noreply@easyssh.io

# SAML SSO (可选)
SAML_ISSUER=https://easyssh.io/sso/saml
SAML_IDP_SSO_URL=https://idp.example.com/sso
SAML_IDP_CERT=MIIDXTCC...base64-encoded-cert

# OIDC SSO (可选)
OIDC_CLIENT_ID=your-client-id
OIDC_CLIENT_SECRET=your-client-secret
OIDC_AUTHORIZATION_URL=https://idp.example.com/oauth2/authorize
OIDC_TOKEN_URL=https://idp.example.com/oauth2/token
OIDC_USERINFO_URL=https://idp.example.com/oauth2/userinfo
OIDC_REDIRECT_URL=https://easyssh.io/sso/oidc/callback
```

### 运行

```bash
# 开发模式
cd pro-server
cargo run

# 生产模式
cargo run --release
```

访问Swagger UI: http://localhost:8080/swagger-ui

## API端点

### 认证
- `POST /api/v1/auth/register` - 用户注册
- `POST /api/v1/auth/login` - 用户登录
- `POST /api/v1/auth/refresh` - 刷新Token
- `POST /api/v1/auth/logout` - 登出
- `GET /api/v1/auth/me` - 获取当前用户
- `POST /api/v1/auth/api-keys` - 创建API Key

### 团队管理
- `POST /api/v1/teams` - 创建团队
- `GET /api/v1/teams` - 列表团队
- `GET /api/v1/teams/:id` - 获取团队详情
- `PUT /api/v1/teams/:id` - 更新团队
- `DELETE /api/v1/teams/:id` - 删除团队
- `POST /api/v1/teams/:id/members` - 邀请成员
- `GET /api/v1/teams/:id/members` - 列表成员
- `POST /api/v1/teams/invitations/:token/accept` - 接受邀请

### 审计日志
- `GET /api/v1/audit` - 查询审计日志
- `GET /api/v1/audit/export` - 导出审计日志(CSV)
- `GET /api/v1/audit/stats` - 审计统计

### RBAC
- `GET /api/v1/rbac/roles` - 列表角色
- `POST /api/v1/rbac/roles` - 创建角色
- `GET /api/v1/rbac/permissions` - 列表权限
- `POST /api/v1/rbac/check` - 检查权限

### 共享资源
- `POST /api/v1/resources/servers` - 共享服务器
- `GET /api/v1/resources/servers` - 列表共享服务器
- `POST /api/v1/resources/snippets` - 创建代码片段
- `GET /api/v1/resources/snippets` - 列表代码片段

### SSO
- `GET /sso/saml/:team_id/login` - SAML登录入口
- `POST /sso/saml/:team_id/acs` - SAML断言消费服务
- `GET /sso/oidc/:team_id/login` - OIDC登录入口
- `GET /sso/oidc/:team_id/callback` - OIDC回调

## 数据库架构

支持SQLite、PostgreSQL、MySQL。

主要表：
- `users` - 用户信息
- `teams` - 团队信息
- `team_members` - 团队成员关系
- `invitations` - 团队邀请
- `roles` - RBAC角色
- `permissions` - RBAC权限
- `role_permissions` - 角色权限关联
- `api_keys` - API密钥
- `audit_logs` - 审计日志
- `shared_servers` - 共享服务器
- `snippets` - 代码片段
- `sso_configs` - SSO配置

## 安全特性

1. **密码安全**: Argon2id哈希
2. **API Key**: SHA256哈希存储，前缀显示
3. **Token撤销**: Redis存储黑名单
4. **速率限制**: 基于Redis的滑动窗口限流
5. **SQL注入防护**: SQLx参数化查询
6. **CORS**: 可配置的跨域策略

## 开发

```bash
# 运行测试
cargo test

# 代码检查
cargo clippy

# 格式化
cargo fmt
```

## 部署

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=builder /app/target/release/pro-server /usr/local/bin/
EXPOSE 8080
CMD ["pro-server"]
```

### Systemd

```ini
[Unit]
Description=EasySSH Pro Server
After=network.target

[Service]
Type=simple
User=easyssh
WorkingDirectory=/opt/easyssh-pro
EnvironmentFile=/opt/easyssh-pro/.env
ExecStart=/usr/local/bin/pro-server
Restart=always

[Install]
WantedBy=multi-user.target
```

## 许可证

MIT
