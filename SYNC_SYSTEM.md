# EasySSH 配置同步系统

## 概述

配置同步系统支持跨设备端到端加密同步，参考1Password和VS Code Settings Sync设计。

## 已实现功能

### 1. 端到端加密 (E2EE)
- 使用AES-256-GCM加密所有同步数据
- 基于Argon2id的密钥派生
- 数据在离开设备前已加密

### 2. 多设备实时同步
- 增量同步支持
- 向量时钟解决并发冲突
- 实时事件通知系统

### 3. 离线支持
- 本地SQLite数据库缓存
- 网络恢复后自动同步
- 离线模式完全可用

### 4. 冲突解决策略
```rust
pub enum SyncConflictResolution {
    UseLocal,      // 保留本地版本
    UseRemote,     // 使用远程版本
    Merge,         // 尝试智能合并
    KeepBoth,      // 保留两个版本
    Skip,          // 跳过此冲突
}
```

### 5. 同步历史与版本恢复
- 自动创建配置快照
- 支持回滚到任意历史版本
- 版本元数据追踪

### 6. 选择性同步
```rust
pub struct SyncScope {
    pub include_all: bool,
    pub included_groups: Vec<String>,     // 仅同步指定分组
    pub excluded_groups: Vec<String>,     // 排除指定分组
    pub include_identities: bool,         // 同步身份
    pub include_snippets: bool,           // 同步代码片段
    pub include_layouts: bool,              // 同步布局
    pub include_settings: bool,             // 同步设置
}
```

### 7. 云端提供者支持
- **iCloud** (macOS/iOS)
- **Google Drive**
- **OneDrive**
- **Dropbox**
- **自建服务器** (REST API)
- **本地文件路径** (测试/NAS)

### 8. 本地网络同步
- mDNS设备发现
- 同一WiFi下直接P2P同步
- 无需云端服务器

## 核心API

### SyncManager
```rust
// 创建同步管理器
let (manager, events) = SyncManager::new(db, config)?;

// 启动同步服务
manager.start().await?;

// 执行完整同步
let stats = manager.sync().await?;

// 创建版本快照
let version = manager.create_version(Some("Before migration")).await?;

// 恢复历史版本
manager.restore_version(&version.version_id).await?;

// 设置同步范围
manager.set_scope(scope).await?;
```

### 配置示例

#### iCloud同步
```rust
let config = SyncConfig {
    enabled: true,
    device_id: uuid::Uuid::new_v4().to_string(),
    device_name: "MacBook Pro".to_string(),
    encryption_key: Some("my-secure-key".to_string()),
    provider: SyncProvider::ICloud,
    scope: SyncScope::default(),
    auto_sync: true,
    sync_interval_secs: 300,
    conflict_resolution: SyncConflictResolution::UseLocal,
    local_sync_enabled: false,
    max_history_versions: 10,
    last_sync_at: None,
};
```

#### 自建服务器
```rust
let config = SyncConfig {
    enabled: true,
    device_id: device_id.clone(),
    device_name: "Work Laptop".to_string(),
    encryption_key: Some(encryption_key),
    provider: SyncProvider::SelfHosted {
        url: "https://sync.mycompany.com".to_string(),
        token: api_token,
    },
    scope: SyncScope {
        include_all: false,
        included_groups: vec!["production".to_string()],
        excluded_groups: vec!["personal".to_string()],
        include_identities: true,
        include_snippets: true,
        include_layouts: false,
        include_settings: true,
    },
    auto_sync: true,
    sync_interval_secs: 60,
    conflict_resolution: SyncConflictResolution::Merge,
    local_sync_enabled: true,
    max_history_versions: 50,
    last_sync_at: None,
};
```

## FFI接口

为跨平台UI提供C FFI接口：

```c
// 创建同步管理器
SyncManagerHandle* sync_manager_create(
    const char* device_id,
    const char* device_name,
    const char* encryption_key,
    int provider_type,
    const char* provider_config
);

// 执行同步
int sync_manager_sync(SyncManagerHandle* handle, void (*callback)(int, int));

// 获取同步状态
char* sync_manager_get_status(SyncManagerHandle* handle);

// 释放字符串
void sync_free_string(char* str);
```

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                       SyncManager                             │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  SyncConfig │  │ CryptoState │  │  Database   │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│                    SyncProviderImpl                          │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │
│  │  iCloud  │ │  GDrive  │ │ 自建服务器 │ │ 本地网络同步  │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 数据流

```
┌─────────┐    Encrypt    ┌─────────┐    Upload    ┌─────────┐
│  Local  │ ─────────────>│  Sync   │ ────────────>│  Cloud  │
│  Data   │               │ Bundle  │              │ Storage │
└─────────┘               └─────────┘              └─────────┘
     │                        │                        │
     ▼                        ▼                        ▼
┌─────────┐               ┌─────────┐              ┌─────────┐
│  SQLite │               │  E2EE   │              │ Version │
│  Cache  │               │  Crypto │              │ History │
└─────────┘               └─────────┘              └─────────┘
```

## 测试

```bash
# 运行同步模块测试
cargo test --features sync -p easyssh-core sync

# 检查编译
cargo check --features sync -p easyssh-core
```

## 后续优化

1. **增量同步优化**：仅传输变更字段
2. **压缩**：使用zstd压缩大文档
3. **带宽限制**：添加上传/下载限速
4. **冲突UI**：可视化冲突解决界面
5. **同步队列**：支持离线操作队列
6. **多因素认证**：同步前MFA验证

## 参考

- 1Password同步架构
- VS Code Settings Sync
- iCloud CloudKit
- Google Drive API
