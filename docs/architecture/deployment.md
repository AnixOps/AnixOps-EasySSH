# EasySSH 部署架构

> 多环境部署与运维架构设计
> 版本: 1.0 | 日期: 2026-04-01

---

## 目录

1. [部署概览](#1-部署概览)
2. [桌面端部署](#2-桌面端部署)
3. [Pro云端部署](#3-pro云端部署)
4. [CI/CD流水线](#4-cicd流水线)
5. [监控与告警](#5-监控与告警)
6. [灾备与恢复](#6-灾备与恢复)

---

## 1. 部署概览

### 1.1 部署架构总图

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                              EasySSH 部署架构全景                                        │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                         │
│   ┌───────────────────────────────────────────────────────────────────────────────┐  │
│   │                          客户端部署 (Desktop Apps)                             │  │
│   │                                                                                │  │
│   │   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐              │  │
│   │   │   Lite (egui)   │  │ Standard (Tauri)│  │  Pro (Tauri)    │              │  │
│   │   │                 │  │                 │  │                 │              │  │
│   │   │ • Windows (.exe)│  │ • Windows (.exe)│  │ • Windows (.exe)│              │  │
│   │   │ • macOS (.app) │  │ • macOS (.app) │  │ • macOS (.app) │              │  │
│   │   │ • Linux (AppImg│  │ • Linux (.AppImg│  │ • Linux (.AppImg│              │  │
│   │   │                 │  │                 │  │                 │              │  │
│   │   │ [GitHub Releases]│ [GitHub Releases]│ [GitHub Releases]│              │  │
│   │   │ [Auto Updater]  │  │ [Auto Updater]  │  │ [Auto Updater]  │              │  │
│   │   └─────────────────┘  └─────────────────┘  └────────┬────────┘              │  │
│   │                                                     │                        │  │
│   └─────────────────────────────────────────────────────┼────────────────────────┘  │
│                                                       │                           │
│                                                       │ HTTPS/WSS                 │
│                                                       ▼                           │
│   ┌─────────────────────────────────────────────────────────────────────────────┐ │
│   │                        Pro Cloud 部署 (Kubernetes)                           │ │
│   │                                                                             │ │
│   │   ┌───────────────────────────────────────────────────────────────────────┐  │ │
│   │   │                        入口层 (Ingress)                              │  │ │
│   │   │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                   │  │ │
│   │   │  │   CDN       │  │  API GW     │  │  WAF        │                   │  │ │
│   │   │  │  (Static)   │  │  (Traefik)  │  │  (Rate)     │                   │  │ │
│   │   │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                   │  │ │
│   │   │         └─────────────────┴─────────────────┘                          │  │ │
│   │   └───────────────────────────────────────────────────────────────────────┘  │ │
│   │                                    │                                          │ │
│   │   ┌───────────────────────────────────────────────────────────────────────┐  │ │
│   │   │                        服务层 (Services)                             │  │ │
│   │   │                                                                      │  │ │
│   │   │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │  │ │
│   │   │   │  API Server │  │  WebSocket  │  │  Sync       │  │  Webhook    │ │  │ │
│   │   │   │  (Actix)    │  │  Server     │  │  Worker     │  │  Handler    │ │  │ │
│   │   │   │  3 replicas │  │  2 replicas │  │  2 replicas │  │  1 replica  │ │  │ │
│   │   │   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘ │  │ │
│   │   │          └─────────────────┴─────────────────┴─────────────────┘       │  │ │
│   │   │                                                                      │  │ │
│   │   └───────────────────────────────────────────────────────────────────────┘  │ │
│   │                                    │                                          │ │
│   │   ┌───────────────────────────────────────────────────────────────────────┐  │ │
│   │   │                        数据层 (Data Layer)                           │  │ │
│   │   │                                                                      │  │ │
│   │   │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │  │ │
│   │   │   │ PostgreSQL  │  │  Redis      │  │  MinIO      │  │  Elasticsearch│ │ │
│   │   │   │ (Primary)   │  │  (Cluster)  │  │  (S3 API)   │  │  (Logs)     │ │  │ │
│   │   │   │  + Replica  │  │             │  │             │  │             │ │  │ │
│   │   │   └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │  │ │
│   │   │                                                                      │  │ │
│   │   └───────────────────────────────────────────────────────────────────────┘  │ │
│   │                                                                             │ │
│   └─────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                      │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 部署环境矩阵

| 环境 | 用途 | 配置 | 可用性 |
|------|------|------|--------|
| **Development** | 本地开发 | Docker Compose | 单节点 |
| **Staging** | 预发布测试 | K8s (1 replica) | 单可用区 |
| **Production** | 生产环境 | K8s (3+ replicas) | 多可用区 |
| **Enterprise** | 私有化部署 | K8s / VM | 客户自定 |

---

## 2. 桌面端部署

### 2.1 构建流程

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                       桌面端构建流水线 (GitHub Actions)                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐         │
│  │   Trigger   │   │   Build     │   │   Sign      │   │   Release   │         │
│  │   (Push)    │──>│   (Matrix)  │──>│   (Codesign)│──>│   (Deploy)  │         │
│  └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘         │
│        │                  │                  │                  │               │
│        │                  │                  │                  │               │
│        ▼                  ▼                  ▼                  ▼               │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐         │
│  │ • Tag push  │   │ • Lint      │   │ • macOS     │   │ • GitHub    │         │
│  │ • Release   │   │ • Test      │   │   (Apple ID)│   │   Releases  │         │
│  │   branch    │   │ • Build     │   │ • Windows   │   │ • CDN       │         │
│  │             │   │   - Win     │   │   (Cert)    │   │   Upload    │         │
│  │             │   │   - Mac     │   │ • Linux     │   │ • Update    │         │
│  │             │   │   - Linux   │   │   (GPG)     │   │   Server    │         │
│  └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘         │
│                                                                                 │
│  构建矩阵:                                                                       │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┐                     │
│  │   OS        │   Target    │   Runner    │   Time      │                     │
│  ├─────────────┼─────────────┼─────────────┼─────────────┤                     │
│  │ Windows     │ x86_64      │ windows-2022│ ~8 min      │                     │
│  │ macOS (x64) │ x86_64      │ macos-13    │ ~10 min     │                     │
│  │ macOS (arm) │ aarch64     │ macos-14    │ ~12 min     │                     │
│  │ Linux       │ x86_64      │ ubuntu-22.04│ ~6 min      │                     │
│  └─────────────┴─────────────┴─────────────┴─────────────┘                     │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 自动更新架构

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        自动更新架构 (Auto-Update)                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│   客户端                                    更新服务                            │
│     │                                         │                                 │
│     │ 1. 启动时检查                             │                                 │
│     │──────────┐                              │                                 │
│     │            ▼                              │                                 │
│     │     ┌─────────────┐                       │                                 │
│     │     │  Check Interval                      │                                 │
│     │     │  (24 hours) │                       │                                 │
│     │     └──────┬──────┘                       │                                 │
│     │            │                              │                                 │
│     │ 2. 请求更新元数据                         │                                 │
│     ├──────────────────────────────────────────>│                                 │
│     │            │                              │                                 │
│     │            │ 3. 返回 update.json         │                                 │
│     │<──────────────────────────────────────────┤                                 │
│     │            │                              │                                 │
│     │     ┌──────┴──────┐                       │                                 │
│     │     │  Compare    │                       │                                 │
│     │     │  Versions   │                       │                                 │
│     │     └──────┬──────┘                       │                                 │
│     │            │                              │                                 │
│     │     ┌──────┴──────┐                       │                                 │
│     │     │  New Version│                       │                                 │
│     │     │  Available? │                       │                                 │
│     │     └──────┬──────┘                       │                                 │
│     │            │                              │                                 │
│     │      ┌─────┴─────┐                        │                                 │
│     │      ▼           ▼                        │                                 │
│     │   [No]        [Yes]                       │                                 │
│     │     │            │                        │                                 │
│     │     │     4. 提示用户                      │                                 │
│     │     │     (可选后台)                       │                                 │
│     │     │            │                        │                                 │
│     │     │     5. 下载更新包                    │                                 │
│     │     │     ├───────────────────────────────>│                                 │
│     │     │            │                        │                                 │
│     │     │     6. 签名验证                      │                                 │
│     │     │     ┌─────────────┐                  │                                 │
│     │     │     │  Verify     │                  │                                 │
│     │     │     │  Signature  │                  │                                 │
│     │     │     └──────┬──────┘                  │                                 │
│     │     │            │                        │                                 │
│     │     │     7. 应用更新                      │                                 │
│     │     │     (下次启动)                       │                                 │
│     │     │            │                        │                                 │
│     │     │     8. 重启应用                      │                                 │
│     │     │     ├───────────────────────────────>│                                 │
│     │     │            │                        │                                 │
│     └─────┴────────────┴────────────────────────┘                                 │
│                                                                                 │
│  更新元数据格式 (update.json):                                                   │
│  {                                                                              │
│    "version": "2.1.0",                                                          │
│    "notes": "Security fix for CVE-2026-XXXX",                                   │
│    "pub_date": "2026-04-01T00:00:00Z",                                          │
│    "platforms": {                                                               │
│      "darwin-x86_64": {                                                         │
│        "signature": "dW50cnVzdGVkIGNvbW1lbnQ...",                               │
│        "url": "https://cdn.easyssh.pro/updates/2.1.0/mac-x64.tar.gz"            │
│      },                                                                         │
│      "darwin-aarch64": { ... },                                                 │
│      "windows-x86_64": { ... },                                                 │
│      "linux-x86_64": { ... }                                                    │
│    }                                                                            │
│  }                                                                              │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 2.3 安装包规格

| 版本 | 平台 | 格式 | 大小限制 | 签名方式 |
|------|------|------|----------|----------|
| **Lite** | Windows | .exe (NSIS) | < 5 MB | Authenticode |
| | macOS | .dmg | < 5 MB | Apple ID |
| | Linux | AppImage | < 5 MB | GPG |
| **Standard** | Windows | .exe (NSIS) | < 30 MB | Authenticode |
| | macOS | .dmg | < 30 MB | Apple ID |
| | Linux | AppImage | < 30 MB | GPG |
| **Pro** | Windows | .exe (NSIS) | < 30 MB | Authenticode |
| | macOS | .dmg | < 30 MB | Apple ID |
| | Linux | AppImage | < 30 MB | GPG |

---

## 3. Pro云端部署

### 3.1 Kubernetes架构

```yaml
# ===========================================
# Pro Cloud K8s 部署配置
# ===========================================

# API Server Deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-api
  namespace: production
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: easyssh-api
  template:
    metadata:
      labels:
        app: easyssh-api
    spec:
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchExpressions:
                    - key: app
                      operator: In
                      values:
                        - easyssh-api
                topologyKey: kubernetes.io/hostname
      containers:
        - name: api
          image: easyssh/pro-api:latest
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
            - name: JWT_SECRET
              valueFrom:
                secretKeyRef:
                  name: easyssh-secrets
                  key: jwt-secret
          resources:
            requests:
              memory: "256Mi"
              cpu: "250m"
            limits:
              memory: "512Mi"
              cpu: "500m"
          livenessProbe:
            httpGet:
              path: /health/live
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
---
# API Server Service
apiVersion: v1
kind: Service
metadata:
  name: easyssh-api
  namespace: production
spec:
  selector:
    app: easyssh-api
  ports:
    - port: 80
      targetPort: 8080
  type: ClusterIP
---
# WebSocket Server Deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-ws
  namespace: production
spec:
  replicas: 2
  selector:
    matchLabels:
      app: easyssh-ws
  template:
    metadata:
      labels:
        app: easyssh-ws
    spec:
      containers:
        - name: ws
          image: easyssh/pro-ws:latest
          ports:
            - containerPort: 8081
          env:
            - name: REDIS_URL
              valueFrom:
                secretKeyRef:
                  name: easyssh-secrets
                  key: redis-url
          resources:
            requests:
              memory: "128Mi"
              cpu: "100m"
            limits:
              memory: "256Mi"
              cpu: "250m"
---
# Horizontal Pod Autoscaler
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: easyssh-api-hpa
  namespace: production
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: easyssh-api
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
        - type: Percent
          value: 100
          periodSeconds: 15
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
        - type: Percent
          value: 10
          periodSeconds: 60
```

### 3.2 数据层架构

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        Pro Cloud 数据层架构                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │                        PostgreSQL 集群                                     ││
│  │                                                                             ││
│  │   ┌─────────────┐                    ┌─────────────┐                       ││
│  │   │   Primary   │<──────Stream──────>│   Replica   │                       ││
│  │   │   (读写)    │      Replication   │   (只读)    │                       ││
│  │   │   r6g.large │                    │   r6g.large │                       ││
│  │   └──────┬──────┘                    └──────┬──────┘                       ││
│  │          │                                   │                              ││
│  │          │          ┌─────────────┐          │                              ││
│  │          └─────────>│  pgBouncer  │<─────────┘                              ││
│  │                     │  (Pool)     │                                         ││
│  │                     └──────┬──────┘                                         ││
│  │                            │                                                ││
│  │                            ▼                                                ││
│  │                     ┌─────────────┐                                         ││
│  │                     │  Apps       │                                         ││
│  │                     └─────────────┘                                         ││
│  │                                                                             ││
│  │  配置:                                                                       ││
│  │  • 存储: 100GB (可扩展到 1TB)                                               ││
│  │  • 备份: 每日自动备份 + 30天保留                                            ││
│  │  • 加密: 存储加密 (AWS KMS)                                                 ││
│  │  • 多AZ: 跨可用区部署                                                        ││
│  │                                                                             ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │                        Redis 集群                                            ││
│  │                                                                             ││
│  │   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                   ││
│  │   │  Master     │<──>│  Replica 1  │    │  Replica 2  │                   ││
│  │   │  cache.r6g  │    │  cache.r6g  │    │  cache.r6g  │                   ││
│  │   │  .large     │    │  .large     │    │  .large     │                   ││
│  │   └──────┬──────┘    └─────────────┘    └─────────────┘                   ││
│  │          │                                                                  ││
│  │          │    用途:                                                          ││
│  │          ├── 会话缓存 (WebSocket状态)                                        ││
│  │          ├── 速率限制 (API throttling)                                       ││
│  │          ├── 实时消息 (Pub/Sub)                                              ││
│  │          └── 临时数据 (Token黑名单)                                          ││
│  │                                                                             ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │                        MinIO / S3 对象存储                                   ││
│  │                                                                             ││
│  │   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                   ││
│  │   │  Bucket:    │    │  Bucket:    │    │  Bucket:    │                   ││
│  │   │  sessions   │    │  avatars    │    │  exports    │                   ││
│  │   │             │    │             │    │             │                   ││
│  │   │ 会话录制     │    │ 用户头像     │    │ 数据导出     │                   ││
│  │   │ (asciicast) │    │             │    │             │                   ││
│  │   │ 生命周期:   │    │ 生命周期:   │    │ 生命周期:   │                   ││
│  │   │ 1-7年      │    │ 永久        │    │ 7天         │                   ││
│  │   └─────────────┘    └─────────────┘    └─────────────┘                   ││
│  │                                                                             ││
│  │  所有数据客户端加密 (E2EE)                                                  ││
│  │                                                                             ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Docker Compose 开发环境

```yaml
# docker-compose.yml
# 本地开发环境一键启动

version: '3.8'

services:
  # PostgreSQL 主数据库
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: easyssh
      POSTGRES_PASSWORD: easyssh_dev
      POSTGRES_DB: easyssh_pro
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U easyssh -d easyssh_pro"]
      interval: 5s
      timeout: 5s
      retries: 5

  # Redis 缓存
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    command: redis-server --appendonly yes

  # MinIO S3兼容存储
  minio:
    image: minio/minio:latest
    environment:
      MINIO_ROOT_USER: easyssh
      MINIO_ROOT_PASSWORD: easyssh_dev_secret
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - minio_data:/data
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 30s
      timeout: 20s
      retries: 3

  # 初始化MinIO bucket
  minio-init:
    image: minio/mc:latest
    depends_on:
      - minio
    entrypoint: >
      /bin/sh -c "
      /usr/bin/mc alias set local http://minio:9000 easyssh easyssh_dev_secret;
      /usr/bin/mc mb local/easyssh-sessions local/easyssh-avatars local/easyssh-exports;
      /usr/bin/mc policy set private local/easyssh-sessions;
      exit 0;
      "

  # MailHog 邮件测试
  mailhog:
    image: mailhog/mailhog:latest
    ports:
      - "1025:1025"  # SMTP
      - "8025:8025"  # Web UI

  # Pro Backend API
  api:
    build:
      context: ./pro-backend
      dockerfile: Dockerfile.dev
    environment:
      DATABASE_URL: postgres://easyssh:easyssh_dev@postgres:5432/easyssh_pro
      REDIS_URL: redis://redis:6379
      S3_ENDPOINT: http://minio:9000
      S3_ACCESS_KEY: easyssh
      S3_SECRET_KEY: easyssh_dev_secret
      SMTP_HOST: mailhog
      SMTP_PORT: 1025
      RUST_LOG: debug
    ports:
      - "8080:8080"
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_started
      minio:
        condition: service_healthy
    volumes:
      - ./pro-backend:/app
      - cargo_cache:/usr/local/cargo/registry
    command: cargo watch -x run

  # WebSocket Server
  websocket:
    build:
      context: ./pro-backend
      dockerfile: Dockerfile.dev
    environment:
      REDIS_URL: redis://redis:6379
      MODE: websocket
    ports:
      - "8081:8081"
    depends_on:
      - redis

volumes:
  postgres_data:
  redis_data:
  minio_data:
  cargo_cache:
```

---

## 4. CI/CD流水线

### 4.1 GitHub Actions 工作流

```yaml
# .github/workflows/release.yml
name: Release Build

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_INCREMENTAL: 0
  RUST_BACKTRACE: short

jobs:
  # 前端构建
  build-frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'

      - name: Install dependencies
        run: npm ci

      - name: Build frontend
        run: npm run build

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: frontend-dist
          path: dist/

  # Rust测试
  test-rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-action@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests
        run: cargo test --all-features

      - name: Run clippy
        run: cargo clippy --all-features -- -D warnings

  # 桌面端构建矩阵
  build-desktop:
    needs: [build-frontend, test-rust]
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest
            target: x86_64-apple-darwin
            args: '--target x86_64-apple-darwin'
          - platform: macos-14
            target: aarch64-apple-darwin
            args: '--target aarch64-apple-darwin'
          - platform: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
            args: ''
          - platform: windows-latest
            target: x86_64-pc-windows-msvc
            args: ''

    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Setup Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install dependencies (Ubuntu)
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Download frontend artifacts
        uses: actions/download-artifact@v4
        with:
          name: frontend-dist
          path: dist/

      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
          WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
          WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'EasySSH ${{ github.ref_name }}'
          releaseBody: 'See the assets to download this version.'
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.args }}

  # 后端镜像构建
  build-backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Login to DockerHub
        uses: docker/login-action@v3
        with:
          username: ${{ secrets.DOCKER_USERNAME }}
          password: ${{ secrets.DOCKER_PASSWORD }}

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: ./pro-backend
          push: true
          tags: |
            easyssh/pro-api:${{ github.ref_name }}
            easyssh/pro-api:latest
          cache-from: type=gha
          cache-to: type=gha,mode=max

  # 部署到Staging
  deploy-staging:
    needs: [build-backend]
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - uses: actions/checkout@v4

      - name: Setup kubectl
        uses: azure/setup-kubectl@v3

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-east-1

      - name: Update kubeconfig
        run: aws eks update-kubeconfig --name easyssh-staging

      - name: Deploy to staging
        run: |
          kubectl set image deployment/easyssh-api api=easyssh/pro-api:${{ github.ref_name }} -n staging
          kubectl rollout status deployment/easyssh-api -n staging
```

---

## 5. 监控与告警

### 5.1 监控架构

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        监控与可观测性架构                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                        指标收集 (Metrics)                              │  │
│   │                                                                         │  │
│   │   客户端                 后端服务              基础设施               │  │
│   │      │                      │                    │                     │  │
│   │      │ 启动时间             │ HTTP请求           │ CPU/内存            │  │
│   │      │ 内存占用             │ 错误率             │ 磁盘I/O             │  │
│   │      │ 崩溃统计             │ 延迟P99            │ 网络流量            │  │
│   │      │ 功能使用             │ 活跃连接           │ DB性能              │  │
│   │      │                      │                    │                     │  │
│   │      └──────────────────────┴────────────────────┘                     │  │
│   │                              │                                          │  │
│   │                              ▼                                          │  │
│   │   ┌─────────────────────────────────────────────────────────────────┐  │  │
│   │   │                      Prometheus / Grafana                        │  │  │
│   │   │                                                                 │  │  │
│   │   │  • 应用指标: API延迟、错误率、吞吐量                            │  │  │
│   │   │  • 业务指标: DAU、同步成功率、团队数                            │  │  │
│   │   │  • 资源指标: CPU、内存、磁盘、网络                              │  │  │
│   │   │                                                                 │  │  │
│   │   └─────────────────────────────────────────────────────────────────┘  │  │
│   │                                                                         │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                        日志聚合 (Logging)                                │  │
│   │                                                                         │  │
│   │   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐               │  │
│   │   │   App       │    │   Audit     │    │   Access    │               │  │
│   │   │   Logs      │───>│   Logs      │───>│   Logs      │               │  │
│   │   │   (JSON)    │    │   (Structured│   │   (Nginx)   │               │  │
│   │   └─────────────┘    └─────────────┘    └─────────────┘               │  │
│   │           │                  │                  │                       │  │
│   │           └──────────────────┴──────────────────┘                       │  │
│   │                              │                                          │  │
│   │                              ▼                                          │  │
│   │   ┌─────────────────────────────────────────────────────────────────┐  │  │
│   │   │                    Elasticsearch / Kibana                       │  │  │
│   │   │                                                                 │  │  │
│   │   │  • 全文搜索: 错误追踪、用户查询                                  │  │  │
│   │   │  • 聚合分析: 错误模式、访问模式                                  │  │  │
│   │   │  • 保留策略: 7天热存储 + 30天冷存储                              │  │  │
│   │   │                                                                 │  │  │
│   │   └─────────────────────────────────────────────────────────────────┘  │  │
│   │                                                                         │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                        链路追踪 (Tracing)                                │  │
│   │                                                                         │  │
│   │   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐               │  │
│   │   │   Client    │    │   API GW    │    │   Service   │               │  │
│   │   │   (Tauri)   │───>│   (Traefik) │───>│   (Actix)   │               │  │
│   │   │             │    │             │    │             │               │  │
│   │   └─────────────┘    └─────────────┘    └─────────────┘               │  │
│   │           │                  │                  │                       │  │
│   │           └──────────────────┴──────────────────┘                       │  │
│   │                              │                                          │  │
│   │                              ▼                                          │  │
│   │   ┌─────────────────────────────────────────────────────────────────┐  │  │
│   │   │                      Jaeger / Zipkin                          │  │  │
│   │   │                                                                 │  │  │
│   │   │  • 请求链路: 端到端延迟分解                                     │  │  │
│   │   │  • 依赖关系: 服务拓扑图                                         │  │  │
│   │   │  • 性能瓶颈: 慢查询识别                                         │  │  │
│   │   │                                                                 │  │  │
│   │   └─────────────────────────────────────────────────────────────────┘  │  │
│   │                                                                         │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 告警规则

```yaml
# Prometheus 告警规则
# prometheus/alerts.yml

groups:
  - name: easyssh-api
    rules:
      # 高错误率告警
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.01
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} errors per second"

      # 高延迟告警
      - alert: HighLatency
        expr: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 0.5
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "High latency detected"
          description: "P99 latency is {{ $value }}s"

      # 数据库连接池耗尽
      - alert: DatabasePoolExhausted
        expr: sql_connections_open / sql_connections_max > 0.8
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Database connection pool nearly exhausted"
          description: "{{ $value }}% of connections in use"

      # 磁盘空间不足
      - alert: DiskSpaceLow
        expr: (node_filesystem_avail_bytes / node_filesystem_size_bytes) < 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Disk space low"
          description: "Less than 10% disk space remaining"

  - name: easyssh-business
    rules:
      # 同步失败率
      - alert: SyncFailureRateHigh
        expr: rate(sync_operations_total{status="failed"}[5m]) / rate(sync_operations_total[5m]) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High sync failure rate"
          description: "{{ $value }}% of sync operations are failing"

      # 异常登录
      - alert: SuspiciousLoginActivity
        expr: rate(login_attempts_total{status="failed"}[5m]) > 10
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Suspicious login activity"
          description: "{{ $value }} failed logins per second"
```

---

## 6. 灾备与恢复

### 6.1 备份策略

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        数据备份与恢复策略                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐  │
│  │                        备份层级                                          │  │
│  ├─────────────────────────────────────────────────────────────────────────┤  │
│  │                                                                          │  │
│  │  Level 1: 实时复制                                                        │  │
│  │  ├── PostgreSQL Streaming Replication (主 -> 从)                           │  │
│  │  ├── Redis AOF持久化 + 主从复制                                          │  │
│  │  └── 应用层双写 (关键操作)                                               │  │
│  │                                                                          │  │
│  │  Level 2: 定期快照                                                        │  │
│  │  ├── 数据库: 每日自动备份 (pg_dump)                                      │  │
│  │  ├── S3存储: 跨区域复制                                                  │  │
│  │  └── 配置: Git版本控制                                                   │  │
│  │                                                                          │  │
│  │  Level 3: 灾难恢复                                                        │  │
│  │  ├── 冷备份: 每周全量备份到不同区域                                      │  │
│  │  ├── 归档存储: Glacier Deep Archive (7年保留)                          │  │
│  │  └── 恢复演练: 季度DR演练                                                │  │
│  │                                                                          │  │
│  └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐  │
│  │                        恢复时间目标 (RTO/RPO)                            │  │
│  ├─────────────────────────────────────────────────────────────────────────┤  │
│  │                                                                          │  │
│  │  服务          RTO        RPO         策略                               │  │
│  │  ─────────────────────────────────────────────────────────────────────   │  │
│  │  API服务       5 min      0 (无数据丢失)  多副本 + 自动故障转移           │  │
│  │  数据库        15 min     < 1 min       主从复制 + 自动切换               │  │
│  │  缓存          5 min      < 1 min       多副本 + 数据预热                  │  │
│  │  对象存储      0 min      0             跨区域复制 (已冗余)                │  │
│  │  完整站点      1 hour     < 1 hour      备用区域激活                       │  │
│  │                                                                          │  │
│  └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐  │
│  │                        灾难恢复流程                                        │  │
│  ├─────────────────────────────────────────────────────────────────────────┤  │
│  │                                                                          │  │
│  │  场景1: 单服务故障                                                        │  │
│  │  ─────────────────────────────────────────────────────────────────────   │  │
│  │  1. 监控告警触发                                                          │  │
│  │  2. K8s自动重启/迁移Pod                                                   │  │
│  │  3. 流量自动切换到健康实例                                                │  │
│  │  4. 人工验证恢复                                                           │  │
│  │  5. 根因分析与修复                                                         │  │
│  │                                                                          │  │
│  │  场景2: 数据库主节点故障                                                   │  │
│  │  ─────────────────────────────────────────────────────────────────────   │  │
│  │  1. 自动故障检测 (patroni/哨兵)                                            │  │
│  │  2. 提升从节点为主节点 (自动)                                             │  │
│  │  3. 更新应用连接池配置 (自动)                                              │  │
│  │  4. 修复原主节点并重新加入复制                                             │  │
│  │                                                                          │  │
│  │  场景3: 可用区故障                                                        │  │
│  │  ─────────────────────────────────────────────────────────────────────   │  │
│  │  1. 检测到AZ故障                                                          │  │
│  │  2. 激活备用区域 (手动/自动)                                               │  │
│  │  3. 更新DNS指向备用区域                                                   │  │
│  │  4. 通知用户可能的服务降级                                                │  │
│  │  5. 修复主区域并恢复复制                                                   │  │
│  │                                                                          │  │
│  │  场景4: 数据损坏/误删                                                      │  │
│  │  ─────────────────────────────────────────────────────────────────────   │  │
│  │  1. 停止相关服务防止进一步损坏                                             │  │
│  │  2. 从备份恢复 (point-in-time recovery)                                  │  │
│  │  3. 验证数据完整性                                                         │  │
│  │  4. 逐步恢复服务                                                          │  │
│  │  5. 事后分析，加强预防措施                                                 │  │
│  │                                                                          │  │
│  └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 附录

### A. 端口分配

| 服务 | 端口 | 协议 | 说明 |
|------|------|------|------|
| API Server | 8080 | HTTP | REST API |
| WebSocket | 8081 | WS/WSS | 实时通信 |
| PostgreSQL | 5432 | TCP | 主数据库 |
| Redis | 6379 | TCP | 缓存 |
| MinIO | 9000 | HTTP | S3 API |
| MinIO Console | 9001 | HTTP | 管理界面 |

### B. 域名规划

| 环境 | 域名 |
|------|------|
| Production | api.easyssh.pro, ws.easyssh.pro |
| Staging | api-staging.easyssh.pro |
| Enterprise (Self-hosted) | 客户自定义 |

### C. 参考文档

- [系统架构](./system-architecture.md)
- [数据流设计](./data-flow.md)
- [API设计](./api-design.md)
