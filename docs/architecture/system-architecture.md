# EasySSH 系统架构设计

> 全平台SSH客户端产品线架构设计文档
> 版本: 2.0 | 日期: 2026-04-01

---

## 目录

1. [架构概览](#1-架构概览)
2. [系统分层架构](#2-系统分层架构)
3. [模块详细设计](#3-模块详细设计)
4. [版本差异化架构](#4-版本差异化架构)
5. [技术栈矩阵](#5-技术栈矩阵)
6. [架构决策记录](#6-架构决策记录)

---

## 1. 架构概览

### 1.1 架构目标

```
┌─────────────────────────────────────────────────────────────────┐
│                     EasySSH 架构目标                             │
├─────────────────────────────────────────────────────────────────┤
│  • 三版本产品分层清晰 (Lite/Standard/Pro)                         │
│  • 核心能力共享，UI体验差异化                                    │
│  • 本地优先，端到端加密                                         │
│  • 跨平台一致体验 (Win/Mac/Linux)                                │
│  • 渐进式功能升级路径                                           │
│  • 企业级安全与审计 (Pro)                                        │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 整体架构图

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                    EasySSH 产品矩阵                                      │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                         │
│  ┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────────────────┐ │
│  │     EasySSH Lite    │  │  EasySSH Standard   │  │       EasySSH Pro               │ │
│  │                     │  │                     │  │                                 │ │
│  │  • SSH配置保险箱     │  │  • 全功能终端工作台  │  │  • 团队协作控制台                │ │
│  │  • 原生终端唤起      │  │  • 分屏 + WebGL     │  │  • RBAC权限管理                  │ │
│  │  • 本地加密存储      │  │  • 监控组件         │  │  • SSO + 审计                    │ │
│  │                     │  │                     │  │                                 │ │
│  │  [独立应用]          │  │  [独立应用]          │  │  [客户端 + Cloud]                │ │
│  └──────────┬──────────┘  └──────────┬──────────┘  └────────────────┬────────────────┘ │
│             │                        │                              │                  │
│             └────────────────────────┼──────────────────────────────┘                  │
│                                      │                                                 │
│                                      ▼                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────────────┐  │
│  │                           统一业务核心层 (Shared Core)                             │  │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌────────────┐ │  │
│  │  │  SSH Engine │ │  Encryption │ │  Keychain   │ │  Database   │ │  Protocol  │ │  │
│  │  │             │ │  (E2EE)     │ │  Manager    │ │  (SQLite)   │ │  Handler   │ │  │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └────────────┘ │  │
│  └─────────────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                                 │
│                                      ▼                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────────────┐  │
│  │                           平台适配层 (Platform Abstraction)                      │  │
│  │                                                                                  │  │
│  │   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌────────────┐  │  │
│  │   │   Desktop    │    │   Mobile     │    │    Web       │    │   Cloud    │  │  │
│  │   │  (Tauri)     │    │  (Native)    │    │  (Admin)     │    │  (Pro)     │  │  │
│  │   │  Win/Mac/Lin │    │  iOS/Android │    │  React       │    │  Rust/Go   │  │  │
│  │   └──────────────┘    └──────────────┘    └──────────────┘    └────────────┘  │  │
│  └─────────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                         │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. 系统分层架构

### 2.1 四层架构模型

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Layer 4: 产品呈现层 (Presentation Layer)                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐   │
│  │   Lite Shell    │  │  Standard Shell │  │        Pro Shell            │   │
│  │                 │  │                 │  │                             │   │
│  │  • 服务器列表    │  │  • 分屏工作区    │  │  • 团队控制台               │   │
│  │  • 配置卡片      │  │  • 终端标签页    │  │  • 审计面板                 │   │
│  │  • 快速连接      │  │  • SFTP浏览器   │  │  • 成员管理                 │   │
│  │  • 原生唤起      │  │  • 监控组件      │  │  • SSO配置                  │   │
│  │                 │  │                 │  │                             │   │
│  │  [egui/Native]  │  │  [Tauri+React]  │  │  [Tauri+React+Cloud]        │   │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘   │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 3: 业务逻辑层 (Business Logic Layer)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌────────┐ │
│  │   Server    │ │  Session    │ │    Sync     │ │    Team     │ │  Audit │ │
│  │   Manager   │ │   Manager   │ │   Service   │ │   Service   │ │Service │ │
│  ├─────────────┤ ├─────────────┤ ├─────────────┤ ├─────────────┤ ├────────┤ │
│  │ • CRUD      │ │ • Lifecycle │ │ • E2EE      │ │ • RBAC      │ │ • Log  │ │
│  │ • Groups    │ │ • Reconnect│ │ • Conflict │ │ • Invite    │ │ • Query│ │
│  │ • Import    │ │ • History  │ │ • Delta    │ │ • Policy   │ │ • Alert│ │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └────────┘ │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 2: 核心能力层 (Core Capability Layer)                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                         SSH Engine                                     │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │  │
│  │  │   ssh2/     │  │   Channel   │  │   Agent     │  │   ProxyJump │  │  │
│  │  │   russh     │  │   Manager   │  │   Forward   │  │   Handler   │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                      Security & Crypto                               │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │  │
│  │  │  Argon2id   │  │ AES-256-GCM │  │  Keyring    │  │   Vault     │  │  │
│  │  │  KDF        │  │  Cipher     │  │  (OS)       │  │   Manager   │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 1: 基础设施层 (Infrastructure Layer)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌────────┐ │
│  │   SQLite    │ │   SQLite    │ │   System    │ │   WebSocket │ │  REST  │ │
│  │   (Local)   │ │   (Remote)  │ │   Process   │ │   Client    │ │  API   │ │
│  │             │ │   (Pro)     │ │   Executor  │ │             │ │        │ │
│  │ • Profiles  │ │ • Teams     │ │ • Terminal  │ │ • Realtime  │ │ • Sync │ │
│  │ • History   │ │ • Audit     │ │ • SFTP      │ │ • Notify    │ │ • Auth │ │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 组件依赖关系

```
┌─────────────────────────────────────────────────────────────────┐
│                    组件依赖关系图                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                        ┌─────────────┐                         │
│                        │   UI Layer  │                         │
│                        │  (Shells)   │                         │
│                        └──────┬──────┘                         │
│                               │                                 │
│              ┌────────────────┼────────────────┐               │
│              │                │                │               │
│              ▼                ▼                ▼               │
│       ┌────────────┐   ┌────────────┐   ┌────────────┐       │
│       │   Server   │   │  Terminal  │   │   Team     │       │
│       │   Store    │   │   Store    │   │   Store    │       │
│       └──────┬─────┘   └──────┬─────┘   └──────┬─────┘       │
│              │                │                │               │
│              └────────────────┼────────────────┘               │
│                               │                                 │
│                               ▼                                 │
│                        ┌─────────────┐                         │
│                        │  Core Lib   │                         │
│                        │  (Rust)     │                         │
│                        └──────┬──────┘                         │
│                               │                                 │
│         ┌─────────────────────┼─────────────────────┐         │
│         │                     │                     │         │
│         ▼                     ▼                     ▼         │
│   ┌───────────┐         ┌───────────┐         ┌───────────┐  │
│   │   SSH2    │         │   Crypto  │         │   DB      │  │
│   │  (Conn)   │         │  (E2EE)   │         │ (SQLite)  │  │
│   └───────────┘         └───────────┘         └───────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. 模块详细设计

### 3.1 核心模块矩阵

| 模块 | 职责 | 技术实现 | 版本支持 |
|------|------|----------|----------|
| **SSH Engine** | 连接管理、协议处理 | ssh2/russh crate | All |
| **Terminal** | 终端模拟、渲染 | xterm.js + WebGL | Std/Pro |
| **Crypto** | 加密/解密、密钥派生 | Argon2id + AES-256-GCM | All |
| **Keychain** | 凭据安全存储 | keyring crate | All |
| **Database** | 本地数据持久化 | SQLite + rusqlite | All |
| **Sync** | 云端同步 (E2EE) | Pro Backend API | Pro |
| **Audit** | 操作审计、会话录制 | Pro Backend + S3 | Pro |
| **Team** | 团队管理、RBAC | Pro Backend | Pro |

### 3.2 SSH Engine 模块

```rust
// SSH Engine 核心接口设计
pub mod ssh_engine {
    use async_trait::async_trait;

    /// SSH连接配置
    pub struct ConnectionConfig {
        pub host: String,
        pub port: u16,
        pub username: String,
        pub auth: AuthMethod,
        pub proxy_jump: Option<Box<ConnectionConfig>>,
        pub timeout: Duration,
    }

    /// 认证方式
    pub enum AuthMethod {
        Password(String),
        PrivateKey { path: PathBuf, passphrase: Option<String> },
        Agent,
    }

    /// SSH会话 trait
    #[async_trait]
    pub trait SshSession: Send + Sync {
        async fn connect(config: &ConnectionConfig) -> Result<Self, SshError>;
        async fn disconnect(&mut self) -> Result<(), SshError>;
        async fn is_connected(&self) -> bool;

        // 终端交互
        async fn open_terminal(&self) -> Result<Channel, SshError>;
        async fn exec(&self, command: &str) -> Result<String, SshError>;

        // SFTP
        async fn open_sftp(&self) -> Result<SftpSession, SshError>;

        // 连接复用
        async fn create_mux_channel(&self) -> Result<Channel, SshError>;
    }

    /// 连接池管理
    pub struct ConnectionPool {
        sessions: Arc<RwLock<HashMap<String, Arc<dyn SshSession>>>>,
        max_idle: usize,
        keepalive_interval: Duration,
    }
}
```

### 3.3 加密模块

```rust
// 加密模块核心设计
pub mod crypto {
    use argon2::{Argon2, Params, Algorithm, Version};
    use aes_gcm::{Aes256Gcm, Key, Nonce};

    /// 加密套件
    pub struct CryptoSuite {
        argon2: Argon2<'static>,
    }

    impl CryptoSuite {
        pub fn new() -> Self {
            let params = Params::new(
                64 * 1024,  // m_cost: 64MB
                3,          // t_cost: 3 iterations
                4,          // p_cost: 4 parallelism
                Some(32),   // output length
            ).unwrap();

            Self {
                argon2: Argon2::new(Algorithm::Argon2id, Version::V0x13, params),
            }
        }

        /// 从主密码派生加密密钥
        pub fn derive_key(&self, master_password: &str, salt: &[u8; 16]) -> [u8; 32] {
            let mut key = [0u8; 32];
            self.argon2
                .hash_password_into(master_password.as_bytes(), salt, &mut key)
                .expect("Key derivation failed");
            key
        }

        /// 加密数据
        pub fn encrypt(&self, plaintext: &[u8], key: &[u8; 32]) -> EncryptedPayload {
            let cipher = Aes256Gcm::new(Key::from_slice(key));
            let nonce = Nonce::from_slice(&self.generate_nonce());

            let ciphertext = cipher.encrypt(nonce, plaintext)
                .expect("Encryption failed");

            EncryptedPayload {
                nonce: nonce.as_slice().to_vec(),
                ciphertext,
            }
        }

        /// 解密数据
        pub fn decrypt(&self, payload: &EncryptedPayload, key: &[u8; 32]) -> Vec<u8> {
            let cipher = Aes256Gcm::new(Key::from_slice(key));
            let nonce = Nonce::from_slice(&payload.nonce);

            cipher.decrypt(nonce, payload.ciphertext.as_ref())
                .expect("Decryption failed")
        }
    }

    /// 加密载荷
    pub struct EncryptedPayload {
        pub nonce: Vec<u8>,
        pub ciphertext: Vec<u8>,
    }
}
```

### 3.4 数据库模块

```rust
// 数据库模块核心设计
pub mod database {
    use rusqlite::{Connection, params};
    use serde::{Serialize, Deserialize};

    /// 数据库管理器
    pub struct DatabaseManager {
        conn: Connection,
        cipher: Option< CryptoSuite>,
    }

    impl DatabaseManager {
        /// 初始化数据库 (带加密)
        pub fn new(db_path: &Path, master_key: Option<&[u8; 32]>) -> Result<Self, DbError> {
            let conn = Connection::open(db_path)?;

            // 启用WAL模式
            conn.execute("PRAGMA journal_mode=WAL", [])?;
            conn.execute("PRAGMA foreign_keys=ON", [])?;

            // 运行迁移
            Self::run_migrations(&conn)?;

            Ok(Self {
                conn,
                cipher: master_key.map(|k| CryptoSuite::with_key(k)),
            })
        }

        /// 服务器CRUD
        pub fn create_server(&self, server: &Server) -> Result<String, DbError> {
            let id = Uuid::new_v4().to_string();
            let encrypted = self.encrypt_sensitive_fields(server)?;

            self.conn.execute(
                "INSERT INTO servers (id, name, host, port, username, auth_encrypted, group_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &id,
                    &server.name,
                    &server.host,
                    server.port,
                    &server.username,
                    &encrypted,
                    server.group_id.as_ref()
                ],
            )?;

            Ok(id)
        }

        pub fn get_servers(&self, group_id: Option<&str>) -> Result<Vec<Server>, DbError> {
            let mut stmt = self.conn.prepare(
                "SELECT * FROM servers WHERE group_id IS ?1 ORDER BY name"
            )?;

            let servers = stmt.query_map([group_id], |row| {
                let encrypted: String = row.get("auth_encrypted")?;
                let decrypted = self.decrypt_sensitive_fields(&encrypted)?;

                Ok(Server {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    host: row.get("host")?,
                    port: row.get("port")?,
                    username: row.get("username")?,
                    auth: decrypted,
                    group_id: row.get("group_id")?,
                })
            })?;

            servers.collect()
        }
    }
}
```

### 3.5 同步服务模块 (Pro)

```rust
// 同步服务模块 (仅Pro版本)
pub mod sync_service {
    use crate::crypto::CryptoSuite;

    /// 同步服务
    pub struct SyncService {
        client: SyncApiClient,
        crypto: CryptoSuite,
        device_id: String,
        last_sync: DateTime<Utc>,
    }

    impl SyncService {
        /// 推送到云端 (E2EE)
        pub async fn push(&self, data: &SyncPayload) -> Result<SyncResult, SyncError> {
            // 1. 序列化
            let serialized = serde_json::to_vec(data)?;

            // 2. 压缩
            let compressed = zstd::encode_all(&serialized, 3)?;

            // 3. 加密 (客户端加密，服务端无法解密)
            let encrypted = self.crypto.encrypt_for_sync(&compressed);

            // 4. 上传
            let response = self.client.upload(encrypted).await?;

            Ok(SyncResult {
                revision: response.revision,
                timestamp: Utc::now(),
            })
        }

        /// 从云端拉取
        pub async fn pull(&self, since: Option<DateTime<Utc>>) -> Result<SyncPayload, SyncError> {
            // 1. 下载加密数据
            let encrypted = self.client.download(since).await?;

            // 2. 解密
            let compressed = self.crypto.decrypt_from_sync(&encrypted)?;

            // 3. 解压
            let serialized = zstd::decode_all(&compressed)?;

            // 4. 反序列化
            let payload = serde_json::from_slice(&serialized)?;

            Ok(payload)
        }

        /// 冲突解决
        pub fn resolve_conflicts(
            &self,
            local: &SyncPayload,
            remote: &SyncPayload,
        ) -> Result<SyncPayload, SyncError> {
            let resolver = ConflictResolver::new(ResolutionStrategy::LastWriteWins);
            resolver.resolve(local, remote)
        }
    }
}
```

---

## 4. 版本差异化架构

### 4.1 功能边界定义

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        三版本功能边界架构                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                           EasySSH Lite                                   │   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐             │   │
│  │  │   Server      │  │   Keychain    │  │   Native      │             │   │
│  │  │   Manager     │  │   Storage     │  │   Terminal    │             │   │
│  │  │   (CRUD)      │  │   (Secrets)   │  │   Launcher    │             │   │
│  │  └───────────────┘  └───────────────┘  └───────────────┘             │   │
│  │  ┌───────────────┐  ┌───────────────┐                                  │   │
│  │  │   Group       │  │   Import/     │                                  │   │
│  │  │   Manager     │  │   Export      │                                  │   │
│  │  └───────────────┘  └───────────────┘                                  │   │
│  │                                                                          │   │
│  │  [无内嵌终端] [无云同步] [无团队功能]                                     │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    ▲                                            │
│                                    │ 扩展                                        │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                        EasySSH Standard                                  │   │
│  │                                                                          │   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌───────────┐ │   │
│  │  │   Terminal    │  │   Split       │  │   SFTP        │  │  Monitor  │ │   │
│  │  │   (xterm.js)  │  │   Layout      │  │   Browser     │  │  Widgets  │ │   │
│  │  └───────────────┘  └───────────────┘  └───────────────┘  └───────────┘ │   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐               │   │
│  │  │   Multi-Tab   │  │   Session     │  │   Local       │               │   │
│  │  │   Manager     │  │   History     │  │   Encryption  │               │   │
│  │  └───────────────┘  └───────────────┘  └───────────────┘               │   │
│  │                                                                          │   │
│  │  [无云同步] [无团队功能] [审计仅限本地]                                   │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    ▲                                            │
│                                    │ 扩展                                        │
│                                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                          EasySSH Pro                                     │   │
│  │                                                                          │   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌───────────┐ │   │
│  │  │   Team        │  │   RBAC        │  │   Cloud       │  │   Audit   │ │   │
│  │  │   Management  │  │   System      │  │   Sync        │  │   Log     │ │   │
│  │  └───────────────┘  └───────────────┘  └───────────────┘  └───────────┘ │   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐               │   │
│  │  │   SSO         │  │   Session     │  │   Shared      │               │   │
│  │  │   (SAML/     │  │   Recording   │  │   Snippets    │               │   │
│  │  │    OIDC)      │  │               │  │               │               │   │
│  │  └───────────────┘  └───────────────┘  └───────────────┘               │   │
│  │                                                                          │   │
│  │  [包含Standard全部功能] [云后端服务] [企业级审计]                         │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 架构扩展策略

```
┌─────────────────────────────────────────────────────────────────┐
│                    版本扩展策略                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Lite 核心包                                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  core-lite (Server, Crypto, Keychain, DB)              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                              ▼                                  │
│  Standard 扩展包                                               │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  core-lite                                             │   │
│  │  + terminal (xterm.js, WebGL, SFTP)                    │   │
│  │  + layout (golden-layout)                              │   │
│  │  + monitor (/proc parser)                              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                              ▼                                  │
│  Pro 扩展包                                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  core-standard                                         │   │
│  │  + cloud-client (Sync API, Team API, Audit API)        │   │
│  │  + sso-client (SAML, OIDC)                             │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. 技术栈矩阵

### 5.1 各层技术选型

| 层次 | 组件 | Lite | Standard | Pro |
|------|------|------|----------|-----|
| **UI Framework** | 前端框架 | egui (Rust) | React 18 | React 18 |
| | 状态管理 | - | Zustand | Zustand |
| | 样式系统 | egui原生 | Tailwind | Tailwind |
| **Desktop Shell** | 运行时 | - | Tauri 2.x | Tauri 2.x |
| | 进程通信 | - | Tauri IPC | Tauri IPC |
| **Terminal** | 渲染引擎 | - | xterm.js + WebGL | xterm.js + WebGL |
| | 布局管理 | - | golden-layout | golden-layout |
| **Backend** | SSH库 | ssh2/russh | ssh2/russh | ssh2/russh |
| | 加密 | Argon2id + AES-256-GCM | Argon2id + AES-256-GCM | Argon2id + AES-256-GCM |
| | 密钥存储 | keyring | keyring | keyring |
| | 本地数据库 | SQLite | SQLite | SQLite |
| **Cloud (Pro)** | API框架 | - | - | Actix-web |
| | 数据库 | - | - | PostgreSQL |
| | 缓存 | - | - | Redis |
| | 存储 | - | - | S3兼容 |
| | 消息队列 | - | - | Redis Pub/Sub |

### 5.2 依赖关系图

```
┌─────────────────────────────────────────────────────────────────┐
│                     技术栈依赖关系                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐                                               │
│  │   React     │◄────────────────────────────────────┐       │
│  │   18.x      │                                      │       │
│  └──────┬──────┘                                      │       │
│         │                                             │       │
│         ▼                                             │       │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐       │
│  │   Zustand   │    │  Tailwind   │    │  Tauri API  │       │
│  │             │    │   CSS       │    │             │       │
│  └─────────────┘    └─────────────┘    └──────┬──────┘       │
│                                               │               │
│                                               ▼               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                    Rust Core (Tauri)                   │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │  │
│  │  │  ssh2   │  │ rusqlite│  │ keyring │  │ crypto  │  │  │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                               │               │
│                    ┌────────────────────────┘               │
│                    │                                          │
│                    ▼                                          │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                   Pro Cloud Backend                      │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │  │
│  │  │actix-web│  │PostgreSQL│  │  Redis  │  │   S3    │  │  │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. 架构决策记录

### 6.1 ADR 列表

| ID | 日期 | 决策 | 状态 |
|----|------|------|------|
| ADR-001 | 2026-03-28 | 采用三版本产品策略 (Lite/Standard/Pro) | 已接受 |
| ADR-002 | 2026-03-28 | Lite使用原生终端而非内嵌 | 已接受 |
| ADR-003 | 2026-03-28 | 采用Tauri 2.x作为桌面框架 | 已接受 |
| ADR-004 | 2026-03-28 | Argon2id + AES-256-GCM加密标准 | 已接受 |
| ADR-005 | 2026-03-28 | SQLite作为本地数据库 | 已接受 |
| ADR-006 | 2026-03-28 | PostgreSQL作为Pro后端数据库 | 已接受 |
| ADR-007 | 2026-03-28 | Pro采用Rust/Actix-web后端 | 已接受 |
| ADR-008 | 2026-03-28 | E2EE端到端加密同步 | 已接受 |
| ADR-009 | 2026-03-28 | xterm.js + WebGL终端渲染 | 已接受 |
| ADR-010 | 2026-03-28 | golden-layout分屏布局 | 已接受 |

### 6.2 关键决策详情

#### ADR-003: 采用Tauri 2.x作为桌面框架

**背景**: 需要为Standard/Pro版本选择跨平台桌面框架

**考虑的选项**:
- Electron: 成熟但bundle大
- Tauri 1.x: 已稳定但功能有限
- Tauri 2.x: 新版，支持移动端，更轻量
- Flutter: 学习成本高

**决策**: 选择Tauri 2.x

**理由**:
1. 轻量级runtime (~600KB vs ~100MB Electron)
2. 支持Windows/macOS/Linux
3. 未来可扩展移动端
4. Rust后端性能优异
5. 安全模型完善

**影响**:
- 需要团队学习Rust
- 前端使用React + TypeScript
- 与系统原生集成能力强

#### ADR-008: E2EE端到端加密同步

**背景**: Pro版本需要云端同步，但用户数据敏感

**决策**: 客户端加密，服务端仅存储密文

**实现**:
```
客户端                     服务端
  │                          │
  │  1. 用户修改配置          │
  │  2. 本地序列化              │
  │  3. 客户端加密 (用户密钥)   │
  │ ──────────────────────────>│
  │  4. 存储密文               │
  │                          │
  │ <─────────────────────────│
  │  5. 其他设备拉取密文        │
  │  6. 客户端解密              │
```

**密钥管理**:
- 主密码派生加密密钥 (Argon2id)
- 密钥从不离开客户端
- 忘记主密码 = 数据无法恢复

---

## 附录

### A. 术语表

| 术语 | 说明 |
|------|------|
| E2EE | End-to-End Encryption，端到端加密 |
| RBAC | Role-Based Access Control，基于角色的访问控制 |
| KDF | Key Derivation Function，密钥派生函数 |
| WAL | Write-Ahead Logging，预写式日志 |
| MUX | Connection Multiplexing，连接复用 |
| IdP | Identity Provider，身份提供商 |

### B. 参考文档

- [数据流设计](./data-flow.md)
- [部署架构](./deployment.md)
- [API设计](./api-design.md)
- [代码质量标准](../standards/code-quality.md)
