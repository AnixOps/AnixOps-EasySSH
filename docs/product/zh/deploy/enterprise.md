# 企业部署指南

本文档面向 IT 管理员和 DevOps 工程师，介绍 EasySSH Pro 服务端的企业级部署方案。

## 部署架构

### 单节点部署

适合小型团队（< 50 人）：

```
┌─────────────────────────────────────┐
│           单节点部署                 │
├─────────────────────────────────────┤
│                                     │
│  ┌─────────────────────────────┐    │
│  │     EasySSH Pro Server    │    │
│  │     (Docker Compose)       │    │
│  ├─────────────────────────────┤    │
│  │  ┌─────────┐  ┌─────────┐  │    │
│  │  │   API   │  │  Web    │  │    │
│  │  │ Service │  │   UI    │  │    │
│  │  └────┬────┘  └────┬────┘  │    │
│  │       └──────┬─────┘       │    │
│  │       ┌──────┴─────┐       │    │
│  │       │ PostgreSQL │       │    │
│  │       │   + Redis  │       │    │
│  │       └────────────┘       │    │
│  └─────────────────────────────┘    │
│                                     │
└─────────────────────────────────────┘
```

### 高可用部署

适合中型团队（50-500 人）：

```
┌─────────────────────────────────────────────────────────────┐
│                     负载均衡层 (HAProxy/NGINX)               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  API Pod 1  │  │  API Pod 2  │  │  API Pod 3  │         │
│  │  (Replica)  │  │  (Replica)  │  │  (Replica)  │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         └─────────────────┼─────────────────┘               │
│                           │                                 │
│  ┌────────────────────────┴────────────────────────┐         │
│  │              数据层                              │         │
│  │  ┌─────────────┐    ┌─────────────────────┐    │         │
│  │  │ PostgreSQL  │    │      Redis          │    │         │
│  │  │  Primary    │◄──►│   Cluster           │    │         │
│  │  │  + Replica  │    │                     │    │         │
│  │  └─────────────┘    └─────────────────────┘    │         │
│  └────────────────────────────────────────────────┘         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 大规模部署

适合大型企业（500+ 人）：

```
┌────────────────────────────────────────────────────────────────┐
│                         全局负载均衡                            │
│                    (CloudFlare / AWS ALB)                      │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │   Region 1      │  │   Region 2      │  │   Region 3   │ │
│  │   (Primary)     │  │   (Standby)     │  │   (Read)     │ │
│  │                 │  │                 │  │              │ │
│  │ ┌───────────┐  │  │ ┌───────────┐  │  │ ┌──────────┐ │ │
│  │ │  K8s      │  │  │ │  K8s      │  │  │ │  K8s     │ │ │
│  │ │ Cluster   │  │  │ │ Cluster   │  │  │ │ Cluster  │ │ │
│  │ └─────┬─────┘  │  │ └─────┬─────┘  │  │ └────┬─────┘ │ │
│  │       │        │  │       │        │  │      │       │ │
│  │ ┌─────┴─────┐  │  │ ┌─────┴─────┐  │  │ ┌────┴────┐ │ │
│  │ │   DB      │◄─┼──┼►│   DB      │  │  │ │   DB    │ │ │
│  │ │ Primary   │  │  │ │ Replica   │  │  │ │ Replica │ │ │
│  │ └───────────┘  │  │ └───────────┘  │  │ └─────────┘ │ │
│  └─────────────────┘  └─────────────────┘  └─────────────┘ │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## 部署要求

### 硬件要求

| 部署规模 | CPU | 内存 | 存储 | 网络 |
|----------|-----|------|------|------|
| 单节点 (<50人) | 4核 | 8GB | 100GB SSD | 100Mbps |
| 高可用 (50-500人) | 8核×3 | 16GB×3 | 500GB SSD | 1Gbps |
| 大规模 (500+人) | 16核×3×3 | 32GB×3×3 | 2TB SSD | 10Gbps |

### 软件要求

| 组件 | 版本 | 说明 |
|------|------|------|
| Docker | 20.10+ | 容器运行时 |
| Kubernetes | 1.24+ | 编排平台（可选） |
| PostgreSQL | 14+ | 主数据库 |
| Redis | 7+ | 缓存/队列 |
| nginx/HAProxy | 1.20+ | 负载均衡 |

## 部署方式

### Docker Compose（推荐起步）

1. **创建部署目录**

```bash
mkdir easyssh-pro && cd easyssh-pro
```

2. **创建 docker-compose.yml**

```yaml
version: '3.8'

services:
  easyssh:
    image: easyssh/pro:latest
    container_name: easyssh-pro
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      - NODE_ENV=production
      - DATABASE_URL=postgres://easyssh:${DB_PASSWORD}@db:5432/easyssh
      - REDIS_URL=redis://redis:6379
      - JWT_SECRET=${JWT_SECRET}
      - ENCRYPTION_KEY=${ENCRYPTION_KEY}
      - ADMIN_EMAIL=${ADMIN_EMAIL}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD}
    depends_on:
      - db
      - redis
    volumes:
      - ./data/uploads:/app/uploads
      - ./data/logs:/app/logs
      - ./data/backups:/app/backups

  db:
    image: postgres:15-alpine
    container_name: easyssh-db
    restart: unless-stopped
    environment:
      POSTGRES_USER: easyssh
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: easyssh
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init:/docker-entrypoint-initdb.d
    command:
      - "postgres"
      - "-c"
      - "wal_level=replica"
      - "-c"
      - "max_wal_senders=10"
      - "-c"
      - "max_replication_slots=10"

  redis:
    image: redis:7-alpine
    container_name: easyssh-redis
    restart: unless-stopped
    volumes:
      - redis_data:/data
    command: redis-server --appendonly yes --maxmemory 256mb --maxmemory-policy allkeys-lru

  nginx:
    image: nginx:alpine
    container_name: easyssh-nginx
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
      - ./data/uploads:/var/www/uploads:ro
    depends_on:
      - easyssh

volumes:
  postgres_data:
  redis_data:
```

3. **创建环境变量文件**

```bash
cat > .env << 'EOF'
# 数据库配置
DB_PASSWORD=your-secure-db-password-here

# 密钥配置（使用 openssl rand -base64 32 生成）
JWT_SECRET=your-jwt-secret-here
ENCRYPTION_KEY=your-encryption-key-here

# 管理员配置
ADMIN_EMAIL=admin@company.com
ADMIN_PASSWORD=your-secure-admin-password
EOF
```

4. **配置 nginx**

```nginx
# nginx.conf
user nginx;
worker_processes auto;

events {
    worker_connections 1024;
}

http {
    upstream easyssh {
        server easyssh:8080;
    }

    server {
        listen 80;
        server_name easyssh.company.com;
        return 301 https://$server_name$request_uri;
    }

    server {
        listen 443 ssl http2;
        server_name easyssh.company.com;

        ssl_certificate /etc/nginx/ssl/cert.pem;
        ssl_certificate_key /etc/nginx/ssl/key.pem;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;
        ssl_prefer_server_ciphers on;

        client_max_body_size 100M;

        location / {
            proxy_pass http://easyssh;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_read_timeout 86400;
        }
    }
}
```

5. **启动服务**

```bash
# 生成密钥
export DB_PASSWORD=$(openssl rand -base64 32)
export JWT_SECRET=$(openssl rand -base64 32)
export ENCRYPTION_KEY=$(openssl rand -base64 32)

# 写入 .env
echo "DB_PASSWORD=$DB_PASSWORD" > .env
echo "JWT_SECRET=$JWT_SECRET" >> .env
echo "ENCRYPTION_KEY=$ENCRYPTION_KEY" >> .env
echo "ADMIN_EMAIL=admin@company.com" >> .env
echo "ADMIN_PASSWORD=$(openssl rand -base64 16)" >> .env

# 启动
docker-compose up -d

# 查看日志
docker-compose logs -f easyssh

# 初始化数据库
docker-compose exec easyssh npm run db:migrate
```

### Kubernetes 部署

1. **创建命名空间**

```bash
kubectl create namespace easyssh
```

2. **创建 ConfigMap**

```yaml
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: easyssh-config
  namespace: easyssh
data:
  NODE_ENV: "production"
  DATABASE_URL: "postgres://easyssh:$(DB_PASSWORD)@db:5432/easyssh"
  REDIS_URL: "redis://redis:6379"
```

3. **创建 Secret**

```bash
kubectl create secret generic easyssh-secrets \
  --from-literal=JWT_SECRET=$(openssl rand -base64 32) \
  --from-literal=ENCRYPTION_KEY=$(openssl rand -base64 32) \
  --from-literal=DB_PASSWORD=$(openssl rand -base64 32) \
  -n easyssh
```

4. **创建 Deployment**

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-pro
  namespace: easyssh
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
          envFrom:
            - configMapRef:
                name: easyssh-config
            - secretRef:
                name: easyssh-secrets
          resources:
            requests:
              memory: "512Mi"
              cpu: "500m"
            limits:
              memory: "1Gi"
              cpu: "1000m"
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 30
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
```

5. **创建 Service 和 Ingress**

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: easyssh-pro
  namespace: easyssh
spec:
  selector:
    app: easyssh-pro
  ports:
    - port: 80
      targetPort: 8080
  type: ClusterIP

---
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: easyssh-pro
  namespace: easyssh
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/proxy-read-timeout: "86400"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "86400"
spec:
  tls:
    - hosts:
        - easyssh.company.com
      secretName: easyssh-tls
  rules:
    - host: easyssh.company.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: easyssh-pro
                port:
                  number: 80
```

6. **部署**

```bash
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secret.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f ingress.yaml
```

## 安全配置

### TLS 配置

**使用 Let's Encrypt（推荐）：**

```yaml
# cert-manager issuer
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@company.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
```

**使用自签名证书：**

```bash
# 生成证书
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout ssl/key.pem -out ssl/cert.pem \
  -subj "/CN=easyssh.company.com"
```

### 网络安全

**防火墙规则：**

```bash
# 允许的必要端口
- 22 (SSH，管理用途)
- 80 (HTTP，重定向到 HTTPS)
- 443 (HTTPS)
- 5432 (PostgreSQL，仅内部)
- 6379 (Redis，仅内部)

# 拒绝其他所有入站连接
```

**WAF 规则（CloudFlare/AWS WAF）：**

```
- 速率限制: 100 请求/分钟/IP
- SQL 注入防护
- XSS 防护
- 黑名单恶意 IP
```

### 数据加密

**静态加密：**

```yaml
# 数据库加密
services:
  db:
    environment:
      POSTGRES_INITDB_ARGS: "--auth-host=scram-sha-256"
    volumes:
      - type: bind
        source: ./encryption/keyfile
        target: /etc/postgres/keyfile
        read_only: true
```

**传输加密：**

```yaml
# 强制 TLS 1.3
ssl_protocols TLSv1.3;
ssl_ciphers TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256;
ssl_prefer_server_ciphers off;
```

## 备份与恢复

### 自动备份

```yaml
# backup-cronjob.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: easyssh-backup
  namespace: easyssh
spec:
  schedule: "0 2 * * *"  # 每天凌晨 2 点
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: backup
              image: postgres:15-alpine
              command:
                - /bin/sh
                - -c
                - |
                  pg_dump $DATABASE_URL | gzip > /backups/easyssh-$(date +%Y%m%d).sql.gz
                  aws s3 cp /backups/easyssh-$(date +%Y%m%d).sql.gz s3://company-backups/easyssh/
              envFrom:
                - secretRef:
                    name: easyssh-secrets
              volumeMounts:
                - name: backups
                  mountPath: /backups
          volumes:
            - name: backups
              persistentVolumeClaim:
                claimName: backup-pvc
          restartPolicy: OnFailure
```

### 恢复数据

```bash
# 从备份恢复
aws s3 cp s3://company-backups/easyssh/easyssh-20260115.sql.gz /tmp/
gunzip /tmp/easyssh-20260115.sql.gz
docker-compose exec -T db psql -U easyssh < /tmp/easyssh-20260115.sql
```

## 监控与告警

### 健康检查端点

```
GET /health     # 基础健康检查
GET /ready      # 就绪检查
GET /metrics    # Prometheus 指标
```

### Prometheus 监控

```yaml
# servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: easyssh-pro
  namespace: monitoring
spec:
  selector:
    matchLabels:
      app: easyssh-pro
  namespaceSelector:
    matchNames:
      - easyssh
  endpoints:
    - port: http
      path: /metrics
      interval: 30s
```

### Grafana 仪表板

导入官方仪表板：
- 仪表板 ID: `easyssh-pro`
- 下载: https://grafana.com/dashboards/easyssh

### 告警规则

```yaml
# alertrules.yaml
groups:
  - name: easyssh
    rules:
      - alert: EasySSHHighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "EasySSH error rate is high"

      - alert: EasySSHDiskSpaceLow
        expr: (node_filesystem_avail_bytes / node_filesystem_size_bytes) < 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "EasySSH disk space is low"
```

## 升级流程

### 滚动升级（Kubernetes）

```bash
# 1. 更新镜像
kubectl set image deployment/easyssh-pro easyssh=easyssh/pro:v1.2.0 -n easyssh

# 2. 监控滚动更新
kubectl rollout status deployment/easyssh-pro -n easyssh

# 3. 如有问题回滚
kubectl rollout undo deployment/easyssh-pro -n easyssh
```

### 数据库迁移

```bash
# 1. 备份
docker-compose exec db pg_dump -U easyssh easyssh > backup.sql

# 2. 应用迁移
docker-compose exec easyssh npm run db:migrate

# 3. 验证
# 测试关键功能
```

## 故障排查

### 常见问题

**服务无法启动：**

```bash
# 检查日志
docker-compose logs easyssh

# 检查依赖
docker-compose ps

# 检查端口占用
netstat -tlnp | grep 8080
```

**数据库连接失败：**

```bash
# 测试连接
docker-compose exec easyssh pg_isready -h db -p 5432

# 检查凭据
docker-compose exec db psql -U easyssh -c "SELECT 1"
```

**性能问题：**

```bash
# 查看资源使用
docker stats

# 查看慢查询
docker-compose exec db psql -U easyssh -c "SELECT * FROM pg_stat_statements ORDER BY total_time DESC LIMIT 10"
```

## 灾难恢复

### RPO/RTO 目标

- **RPO (恢复点目标)**: < 1 小时
- **RTO (恢复时间目标)**: < 4 小时

### 故障转移流程

```
1. 检测故障
   └── 监控系统告警

2. 启动故障转移
   └── kubectl drain node-x
   └── 流量切换到备用区域

3. 恢复服务
   └── 数据库提升备库为主库
   └── 启动新实例

4. 验证恢复
   └── 健康检查
   └── 冒烟测试

5. 事后分析
   └── 根因分析
   └── 改进措施
```

## 合规要求

### SOC 2

- 访问日志保留 1 年
- 定期备份测试
- 变更管理流程

### GDPR

- 数据加密存储
- 用户数据可导出
- 数据删除支持

## 支持与联系

- **文档**: https://docs.easyssh.dev/deploy
- **支持邮箱**: support@easyssh.dev
- **紧急联系**: +1-555-EASYSHH

## 参考

- [Docker Compose 文档](https://docs.docker.com/compose/)
- [Kubernetes 文档](https://kubernetes.io/docs/)
- [PostgreSQL 高可用](https://www.postgresql.org/docs/high-availability.html)
