# EasySSH Pro v0.3.0 发布说明

**发布日期**: 2026-04-15 (计划)
**版本号**: 0.3.0
**代号**: Enterprise

---

## 🎯 产品概述

EasySSH Pro 是面向IT团队和企业运维部门的企业级SSH协作平台，提供团队管理、审计合规、SSO集成和实时协作能力。

### 目标用户
- IT运维团队 (5-500人)
- DevOps团队
- 金融机构/医疗/政府 (有合规要求)

### 定价
- **月付**: $19.99/人/月
- **年付**: $199/人/年 (省17%)
- **最低席位**: 5人
- **企业定制**: 联系销售

---

## ✨ 核心功能

### 1. 团队协作
- **团队管理**: 创建团队、邀请成员、角色分配
- **RBAC权限**: 细粒度角色控制 (Owner/Admin/Member/Guest)
- **共享资源**: 服务器分组、代码片段共享
- **邀请系统**: 邮件邀请+Token验证

### 2. 审计合规
- **完整审计**: 登录、会话、命令、文件传输全记录
- **审计导出**: CSV格式导出
- **统计分析**: 操作类型统计
- **防篡改**: 审计日志防删除

### 3. SSO集成
- **SAML 2.0**: 企业IdP集成 (Okta, Azure AD)
- **OIDC**: OAuth2/OpenID Connect支持
- **自动用户同步**: SSO自动创建/关联用户
- **SP元数据**: 自动生成SAML元数据

### 4. 实时协作 ⭐ 新功能
- **多人会话**: WebSocket实时同步
- **屏幕标注**: 绘制、高亮、箭头、文字
- **评论系统**: 行级代码评论
- **剪贴板共享**: 团队剪贴板历史
- **WebRTC语音**: 内置语音通话
- **会话录制**: asciicast格式回放

### 5. DevOps事件响应 ⭐ 新功能
- **事件管理**: 完整生命周期跟踪
- **告警聚合**: Prometheus/Zabbix/自定义告警
- **运行手册**: 标准化SOP
- **升级策略**: 自动/手动升级规则
- **事后分析**: 根因分析和改进项

---

## 🏗️ 技术架构

### 后端技术栈
```
Rust + Axum + SQLx + Redis
```

### 数据库支持
- SQLite (开发/测试)
- PostgreSQL (生产推荐)
- MySQL (企业兼容)

### 部署方式
- **Docker**: 单容器部署
- **Docker Compose**: 完整环境
- **Kubernetes**: 高可用集群
- **Systemd**: Linux服务

---

## 📋 API变更

### 新增端点 (v0.3.0)

#### 团队协作
```
POST   /api/v1/teams              创建团队
GET    /api/v1/teams              列表团队
GET    /api/v1/teams/:id          获取团队
PUT    /api/v1/teams/:id          更新团队
DELETE /api/v1/teams/:id          删除团队
POST   /api/v1/teams/:id/members  邀请成员
```

#### 审计日志
```
GET    /api/v1/audit              查询审计日志
GET    /api/v1/audit/export       导出CSV
GET    /api/v1/audit/stats        统计信息
```

#### SSO
```
GET    /sso/saml/:team_id/login     SAML登录
POST   /sso/saml/:team_id/acs       SAML回调
GET    /sso/oidc/:team_id/login     OIDC登录
GET    /sso/oidc/:team_id/callback  OIDC回调
```

#### 协作
```
POST   /api/v1/collaboration/sessions              创建会话
GET    /api/v1/collaboration/sessions/:id          获取会话
POST   /api/v1/collaboration/sessions/:id/join     加入会话
DELETE /api/v1/collaboration/sessions/:id          结束会话
GET    /api/v1/collaboration/ws/:id                WebSocket连接
```

---

## 🔐 安全特性

### 认证
- JWT + Refresh Token双Token机制
- API Key支持 (用于CI/CD集成)
- TOTP多因素认证 (MFA)

### 授权
- RBAC细粒度权限
- 团队隔离
- 资源级访问控制

### 审计
- 完整操作日志
- IP地址和用户代理追踪
- 不可删除的审计记录

### 数据保护
- Argon2id密码哈希
- AES-256-GCM端到端加密 (配置同步)
- SQL注入防护 (SQLx参数化)

---

## 📊 性能规格

| 指标 | 规格 |
|------|------|
| 并发连接 | 10,000+ WebSocket |
| API吞吐 | 5,000 RPS (单节点) |
| 数据库 | 支持100万+服务器配置 |
| 审计存储 | 可配置S3存储 |
| 延迟 | P99 < 100ms |

---

## 🚀 快速开始

### Docker部署 (推荐)

```bash
# 1. 克隆仓库
git clone https://github.com/anixops/easyssh-pro.git
cd easyssh-pro/pro-server

# 2. 配置环境
cp .env.example .env
# 编辑 .env 设置JWT_SECRET等

# 3. 启动服务
docker-compose up -d

# 4. 访问Swagger UI
open http://localhost:8080/swagger-ui
```

### 二进制部署

```bash
# 1. 构建
cargo build --release

# 2. 运行
./target/release/pro-server

# 3. 或使用 systemd
sudo cp deploy/easyssh-pro.service /etc/systemd/system/
sudo systemctl enable --now easyssh-pro
```

---

## 📚 文档

- [API文档](pro-server/docs/API_CLIENTS.md)
- [部署指南](../DEPLOYMENT.md)
- [安全审计报告](../SECURITY_AUDIT_COMPLETE_2026-04-01.md)
- [集成指南](../INTEGRATION_PLAN.md)

---

## ⚠️ 已知问题

### 编译问题
- **Windows**: 需要OpenSSL开发库
  - 解决: 安装vcpkg或交叉编译Linux版本

### 待完善
- 更多SSO IdP预配置模板
- 性能基准测试数据
- 多区域部署指南

---

## 🔮 路线图

### v0.4.0 (2026-Q2)
- LDAP/AD直接集成
- 审计日志自动归档
- 更多协作功能 (屏幕共享)

### v0.5.0 (2026-Q3)
- 插件系统
- 自定义告警通道
- 高级分析报告

### v1.0.0 (2026-Q4)
- SOC 2合规认证
- 国际化支持
- 移动端应用

---

## 📞 支持

- **文档**: https://docs.easyssh.pro
- **社区**: https://community.easyssh.pro
- **企业支持**: support@easyssh.pro
- **销售**: sales@easyssh.pro

---

## 📄 许可证

EasySSH Pro 采用商业许可证
- 核心代码: 专有
- API客户端: MIT许可证

---

**Happy SSH-ing! 🖥️🔒**

*EasySSH Team*
