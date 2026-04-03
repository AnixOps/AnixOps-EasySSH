# EasySSH 测试指南

> 全面的测试策略、工具和实践方法

---

## 目录

1. [测试概述](#1-测试概述)
2. [当前测试状态](#2-当前测试状态)
3. [运行测试](#3-运行测试)
4. [单元测试](#4-单元测试)
5. [集成测试](#5-集成测试)
6. [端到端测试](#6-端到端测试)
7. [性能测试](#7-性能测试)
8. [安全测试](#8-安全测试)
9. [测试工具](#9-测试工具)
10. [CI/CD 集成](#10-cicd-集成)
11. [测试最佳实践](#11-测试最佳实践)
12. [故障排除](#12-故障排除)
13. [相关文档](#13-相关文档)

---

## 1. 测试概述

### 1.1 测试金字塔

```
        /\
       /  \     E2E 测试 (少量)
      /____\    Playwright
     /      \
    /        \   集成测试 (中等)
   /__________\  SSH/SFTP/DB 测试
  /            \
 /              \ 单元测试 (大量)
/________________\ 核心逻辑测试
```

### 1.2 测试目标矩阵

| 测试类型 | 目标覆盖率 | 执行时间 | 执行频率 |
|----------|-----------|----------|----------|
| 单元测试 | 80%+ | < 30s | 每次提交 |
| 集成测试 | 60%+ | < 5min | 每次 PR |
| E2E 测试 | 40%+ | < 15min | 每日构建 |
| 性能测试 | 关键路径 | < 30min | 每周 |
| 安全测试 | 100% | < 10min | 每次发布 |

---

## 2. 当前测试状态

### 2.1 测试统计 (2026-04-03)

| 指标 | 数值 |
|------|------|
| **总测试数** | 803 |
| **通过** | 793 |
| **失败** | 0 |
| **忽略** | 10 |
| **执行时间** | ~6.5s |

### 2.2 测试覆盖模块

| 模块 | 测试文件测试数 | 内嵌测试 | 覆盖状态 |
|------|---------------|----------|----------|
| Crypto (加密) | 18 | ~180 | 完整覆盖 |
| Database (数据库) | 16 + 14 (集成) | ~120 | 完整覆盖 |
| SSH (连接配置) | 23 + 13 (集成) | ~30 | 完整覆盖 |
| Services (业务逻辑) | 19 | ~40 | 完整覆盖 |
| Search (搜索) | 14 | ~15 | 完整覆盖 |
| Models (数据模型) | - | ~200 | 完整覆盖 |
| Version (版本管理) | - | ~50 | 完整覆盖 |
| Vault (密码管理) | - | ~15 | 完整覆盖 |
| Security (安全测试) | 13 | - | 完整覆盖 |
| Performance (性能) | 7 | - | 关键路径 |
| Terminal (终端) | - | ~30 | 完整覆盖 |
| Config (配置) | - | ~80 | 完整覆盖 |
| Backup (备份) | - | ~40 | 完整覆盖 |
| Integration (集成) | 34 | - | 核心流程 |
| Fuzz (模糊测试) | 8 | - | 属性测试 |

> **注**: "内嵌测试"指在源码文件中使用 `#[cfg(test)] mod tests` 模块的测试。

### 2.3 测试文件结构

```
crates/easyssh-core/tests/
├── common/                    # 共享测试工具
│   ├── mod.rs                # 测试助手、fixtures、断言
│   └── data_generator.rs     # 测试数据生成工具
├── fixtures/                  # 测试数据文件
│   ├── test_data.json        # 示例服务器、分组、身份
│   └── comprehensive_test_data.json # 完整测试数据集
├── unit/                      # 单元测试
│   ├── crypto_tests.rs       # 加密测试 (18 tests)
│   ├── database_tests.rs     # 数据库CRUD测试 (16 tests)
│   ├── ssh_tests.rs          # SSH配置测试 (23 tests)
│   ├── server_service_tests.rs # 业务逻辑测试 (19 tests)
│   ├── search_tests.rs       # 搜索功能测试 (14 tests)
│   ├── security_tests.rs     # 安全测试 (13 tests)
│   ├── performance_tests.rs  # 性能测试 (7 tests)
│   └── fuzz_tests.rs         # 模糊/属性测试 (8 tests)
├── integration/               # 集成测试
│   ├── workflow_tests.rs     # 端到端工作流测试 (7 tests)
│   ├── database_integration_tests.rs # 数据库集成测试 (14 tests)
│   └── ssh_integration_tests.rs # SSH集成测试 (13 tests)
└── database_compiles.rs      # 数据库编译验证 (1 test)
```

> **注**: 额外的测试内嵌在源码文件的 `#[cfg(test)] mod tests` 模块中。

---

## 3. 运行测试

### 3.1 基础测试命令

```bash
# 运行所有测试 (推荐方式 - 跳过examples)
cargo test -p easyssh-core --lib --tests

# 运行Lite版本测试 (跳过examples编译)
cargo test -p easyssh-core --lib --tests --no-default-features --features "lite"

# 运行Lite + Standard版本测试
cargo test -p easyssh-core --lib --tests --no-default-features --features "lite,standard"

# 运行完整测试 (Lite + Standard + Pro, 需要Rust 1.91+)
cargo test -p easyssh-core --lib --tests --all-features
```

> **注意**: 使用 `--lib --tests` 标志可以避免examples编译错误。examples依赖完整运行环境，在测试时通常不需要编译。

### 3.2 按版本运行测试

| 版本 | 命令 | 测试数 |
|------|------|--------|
| **Lite** | `cargo test -p easyssh-core --lib --tests --features "lite"` | ~793 |
| **Standard** | `cargo test -p easyssh-core --lib --tests --features "lite,standard"` | ~793 |
| **Pro** | `cargo test -p easyssh-core --lib --tests --all-features` | ~793+ |

> **注意**: Pro版本需要Rust 1.91+以支持aws-config等依赖。测试数差异主要来自feature gate控制的测试。

### 3.3 按测试类型运行

```bash
# 仅单元测试
cargo test -p easyssh-core --lib

# 仅集成测试
cargo test -p easyssh-core --tests

# 特定测试文件
cargo test -p easyssh-core --test crypto_tests
cargo test -p easyssh-core --test security_tests
cargo test -p easyssh-core --test performance_tests

# 按名称过滤
cargo test -p easyssh-core crypto
cargo test -p easyssh-core security
cargo test -p easyssh-core database
```

### 3.4 显示测试输出

```bash
# 显示println输出
cargo test -p easyssh-core -- --nocapture

# 显示测试执行顺序
cargo test -p easyssh-core -- --test-threads=1

# 详细输出
cargo test -p easyssh-core -- --verbose
```

### 3.5 运行基准测试

```bash
# 运行所有基准测试
cargo bench -p easyssh-core

# 运行特定基准测试
cargo bench -p easyssh-core --bench crypto_bench
cargo bench -p easyssh-core --bench db_bench
cargo bench -p easyssh-core --bench search_bench
```

### 3.6 运行其他包的测试

```bash
# TUI终端测试
cargo test -p easyssh-tui

# Windows UI测试 (需要Windows环境)
cargo test -p easyssh-winui

# Linux GTK4测试 (需要Linux + GTK4)
cargo test -p easyssh-gtk4

# Pro服务器测试
cargo test -p easyssh-pro-server
```

---

## 4. 单元测试

### 4.1 Rust 单元测试

```rust
// core/src/crypto.rs
#[cfg(test)]
mod tests {
    use super::*;

    // 基本测试
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let data = b"sensitive data";
        let key = derive_key("password", &[1u8; 16]);

        let encrypted = encrypt(data, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();

        assert_eq!(data.as_slice(), decrypted.as_slice());
    }

    // 错误处理测试
    #[test]
    fn test_decrypt_with_wrong_key() {
        let data = b"test data";
        let key1 = derive_key("password1", &[1u8; 16]);
        let key2 = derive_key("password2", &[2u8; 16]);

        let encrypted = encrypt(data, &key1).unwrap();
        let result = decrypt(&encrypted, &key2);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CryptoError::DecryptionFailed));
    }

    // 边界条件测试
    #[test]
    fn test_empty_data_encryption() {
        let data = b"";
        let key = derive_key("password", &[1u8; 16]);

        let encrypted = encrypt(data, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();

        assert!(decrypted.is_empty());
    }

    // 属性测试 (使用 proptest)
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_encryption_deterministic_with_same_nonce(
            data in any::<Vec<u8>>(),
            password in "[a-zA-Z0-9]{8,32}"
        ) {
            let salt = [1u8; 16];
            let key = derive_key(&password, &salt);

            let encrypted1 = encrypt(&data, &key).unwrap();
            let decrypted = decrypt(&encrypted1, &key).unwrap();

            prop_assert_eq!(data, decrypted);
        }
    }
}
```

### 4.2 异步测试

```rust
#[tokio::test]
async fn test_async_connection() {
    let config = SshConfig {
        host: "localhost".to_string(),
        port: 2222,  // 测试服务器端口
        username: "test".to_string(),
        auth: AuthMethod::Password("test".to_string()),
    };

    let result = timeout(Duration::from_secs(5), connect(&config)).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_concurrent_connections() {
    let configs = vec![
        create_test_config(1),
        create_test_config(2),
        create_test_config(3),
    ];

    let results = futures::future::join_all(
        configs.into_iter().map(|c| connect(&c))
    ).await;

    assert!(results.iter().all(|r| r.is_ok()));
}
```

### 4.3 Mock 测试

```rust
use mockall::{mock, predicate::*};

mock! {
    SshClient {}

    #[async_trait]
    impl SshClientTrait for SshClient {
        async fn connect(&self, config: &SshConfig) -> Result<Session, SshError>;
        async fn execute(&self, cmd: &str) -> Result<CommandOutput, SshError>;
        async fn disconnect(&self) -> Result<(), SshError>;
    }
}

#[tokio::test]
async fn test_server_manager_with_mock() {
    let mut mock = MockSshClient::new();

    mock.expect_connect()
        .with(always())
        .times(1)
        .returning(|_| Ok(create_mock_session()));

    let manager = ServerManager::new(Box::new(mock));
    let result = manager.connect_to_server(&test_config()).await;

    assert!(result.is_ok());
}
```

### 4.4 TypeScript 单元测试

```typescript
// src/stores/__tests__/serverStore.test.ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { createServerStore } from '../serverStore';

describe('ServerStore', () => {
  let store: ReturnType<typeof createServerStore>;

  beforeEach(() => {
    store = createServerStore();
  });

  it('should add a server', () => {
    const server = {
      name: 'Test Server',
      host: '192.168.1.1',
      port: 22,
      username: 'admin',
    };

    store.getState().addServer(server);

    expect(store.getState().servers).toHaveLength(1);
    expect(store.getState().servers[0].name).toBe('Test Server');
    expect(store.getState().servers[0].id).toBeDefined();
  });

  it('should update server color', () => {
    const testServer = {
      name: 'Test Server',
      host: '192.168.1.1',
      port: 22,
      username: 'admin',
    };
    store.getState().addServer(testServer);
    const id = store.getState().servers[0].id;

    store.getState().updateServer(id, { color: '#FF0000' });

    expect(store.getState().servers[0].color).toBe('#FF0000');
  });

  it('should handle invalid updates', () => {
    const testServer = {
      name: 'Test Server',
      host: '192.168.1.1',
      port: 22,
      username: 'admin',
    };
    store.getState().addServer(testServer);

    expect(() => {
      store.getState().updateServer('invalid-id', { name: 'New Name' });
    }).not.toThrow();

    expect(store.getState().servers[0].name).toBe(testServer.name);
  });
});
```

---

## 5. 集成测试

### 5.1 SSH 集成测试

```rust
// tests/ssh_integration.rs
use easyssh_core::{SshClient, SshConfig, AuthMethod};

#[tokio::test]
#[ignore = "requires Docker test container"]
async fn test_ssh_connection_to_container() {
    // 启动测试容器
    let container = TestContainer::new("test-sshd")
        .with_port_mapping(2222, 22)
        .start()
        .await;

    // 等待服务就绪
    container.wait_for_port(22, Duration::from_secs(30)).await;

    let config = SshConfig {
        host: container.host(),
        port: container.mapped_port(22),
        username: "testuser".to_string(),
        auth: AuthMethod::Password("testpass".to_string()),
    };

    let client = SshClient::new();
    let session = client.connect(&config).await;

    assert!(session.is_ok());

    // 执行命令
    let output = client.execute("echo 'Hello World'").await.unwrap();
    assert_eq!(output.stdout.trim(), "Hello World");

    // 清理
    container.stop().await;
}

// 使用 testcontainers 库
use testcontainers::{clients::Cli, images::generic::GenericImage};

fn setup_test_env() -> (Cli, Container<GenericImage>) {
    let docker = clients::Cli::default();
    let image = GenericImage::new("linuxserver/openssh-server", "latest")
        .with_env_var("PUID", "1000")
        .with_env_var("PGID", "1000")
        .with_env_var("TZ", "Asia/Shanghai")
        .with_env_var("USER_NAME", "test")
        .with_env_var("USER_PASSWORD", "test");

    let container = docker.run(image);
    (docker, container)
}
```

### 5.2 数据库集成测试

```rust
// tests/db_integration.rs
use easyssh_core::db::{Database, Connection};
use tempfile::TempDir;

#[tokio::test]
async fn test_database_migrations() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await.unwrap();

    // 验证所有表已创建
    let tables = db.list_tables().await.unwrap();
    assert!(tables.contains(&"servers".to_string()));
    assert!(tables.contains(&"groups".to_string()));
    assert!(tables.contains(&"sessions".to_string()));
}

#[tokio::test]
async fn test_server_crud_operations() {
    let db = setup_test_db().await;

    // Create
    let server = Server::new("Test", "192.168.1.1", 22);
    let id = db.add_server(&server).await.unwrap();

    // Read
    let retrieved = db.get_server(&id).await.unwrap();
    assert_eq!(retrieved.name, "Test");

    // Update
    db.update_server(&id, &ServerUpdate {
        name: Some("Updated".to_string()),
        ..Default::default()
    }).await.unwrap();

    let updated = db.get_server(&id).await.unwrap();
    assert_eq!(updated.name, "Updated");

    // Delete
    db.delete_server(&id).await.unwrap();
    assert!(db.get_server(&id).await.is_err());
}
```

### 5.3 API 集成测试

```rust
// tests/api_integration.rs
use easyssh_core::api::ServerApi;

#[tokio::test]
async fn test_api_server_lifecycle() {
    let api = setup_test_api().await;

    // Create
    let response = api
        .create_server(json!({
            "name": "API Test",
            "host": "10.0.0.1",
            "port": 22,
            "username": "admin"
        }))
        .await;

    assert_eq!(response.status(), 201);
    let server: Server = response.json().await.unwrap();

    // Get
    let response = api.get_server(&server.id).await;
    assert_eq!(response.status(), 200);

    // List
    let response = api.list_servers().await;
    assert_eq!(response.status(), 200);
    let servers: Vec<Server> = response.json().await.unwrap();
    assert!(servers.iter().any(|s| s.id == server.id));

    // Delete
    let response = api.delete_server(&server.id).await;
    assert_eq!(response.status(), 204);
}
```

---

## 6. 端到端测试

### 6.1 Playwright E2E 测试

```typescript
// e2e/server-management.spec.ts
import { test, expect } from '@playwright/test';

test.describe('服务器管理', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('添加服务器并连接', async ({ page }) => {
    // 打开添加对话框
    await page.click('[data-testid="add-server-button"]');

    // 填写表单
    await page.fill('[data-testid="server-name"]', 'Test Server');
    await page.fill('[data-testid="server-host"]', '192.168.1.1');
    await page.fill('[data-testid="server-port"]', '22');
    await page.fill('[data-testid="server-username"]', 'admin');
    await page.selectOption('[data-testid="server-auth-type"]', 'password');
    await page.fill('[data-testid="server-password"]', 'password123');

    // 保存
    await page.click('[data-testid="save-server-button"]');

    // 验证出现在列表中
    await expect(page.locator('text=Test Server')).toBeVisible();

    // 点击连接
    await page.click('[data-testid="connect-server-button"]');

    // 验证终端打开
    await expect(page.locator('[data-testid="terminal-view"]')).toBeVisible();
  });

  test('搜索过滤服务器', async ({ page }) => {
    // 添加测试数据
    await addTestServers(page, 10);

    // 搜索
    await page.fill('[data-testid="search-input"]', 'Production');

    // 验证过滤结果
    const count = await page.locator('[data-testid="server-item"]').count();
    expect(count).toBeLessThan(10);
  });

  test('分组管理', async ({ page }) => {
    // 创建分组
    await page.click('[data-testid="add-group-button"]');
    await page.fill('[data-testid="group-name"]', 'Production');
    await page.click('[data-testid="save-group-button"]');

    // 移动服务器到分组
    await page.dragAndDrop(
      '[data-testid="server-item"]:first-child',
      '[data-testid="group-production"]'
    );

    // 验证分组
    const count = await page.locator('[data-testid="group-production"] [data-testid="server-item"]').count();
    expect(count).toBeGreaterThan(0);
  });
});
```

### 6.2 视觉回归测试

```typescript
// e2e/visual.spec.ts
import { test, expect } from '@playwright/test';

test.describe('视觉回归', () => {
  test('主界面截图', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    expect(await page.screenshot()).toMatchSnapshot('main-interface.png');
  });

  test('暗色主题', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="theme-toggle"]');

    expect(await page.screenshot()).toMatchSnapshot('dark-theme.png');
  });

  test('添加服务器对话框', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="add-server-button"]');

    expect(await page.locator('[role="dialog"]').screenshot())
      .toMatchSnapshot('add-server-dialog.png');
  });
});
```

### 6.3 可访问性测试

```typescript
// e2e/accessibility.spec.ts
import { test, expect } from '@playwright/test';
import { injectAxe, checkA11y } from 'axe-playwright';

test.describe('可访问性', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await injectAxe(page);
  });

  test('首页可访问性检查', async ({ page }) => {
    await checkA11y(page, undefined, {
      axeOptions: {
        rules: {
          'color-contrast': { enabled: true },
          'heading-order': { enabled: true },
        },
      },
    });
  });
});
```

---

## 7. 性能测试

### 7.1 负载测试

```rust
// benches/connection_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use easyssh_core::SshClient;

fn bench_connection_establishment(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = create_test_config();

    c.bench_function("ssh_connect", |b| {
        b.to_async(&rt).iter(|| async {
            let client = SshClient::new();
            black_box(client.connect(&config).await)
        });
    });
}

fn bench_encryption(c: &mut Criterion) {
    let data = vec![0u8; 1024 * 1024]; // 1MB
    let key = derive_key("password", &[1u8; 16]);

    c.bench_function("encrypt_1mb", |b| {
        b.iter(|| {
            black_box(encrypt(&data, &key));
        });
    });
}

criterion_group!(benches, bench_connection_establishment, bench_encryption);
criterion_main!(benches);
```

### 7.2 并发测试

```rust
// tests/concurrent_tests.rs
#[tokio::test]
async fn test_concurrent_connections() {
    let client = Arc::new(SshClient::new());
    let config = Arc::new(create_test_config());

    let tasks: Vec<_> = (0..100)
        .map(|i| {
            let client = client.clone();
            let config = config.clone();
            tokio::spawn(async move {
                let result = timeout(
                    Duration::from_secs(10),
                    client.connect(&config)
                ).await;
                (i, result)
            })
        })
        .collect();

    let results = futures::future::join_all(tasks).await;

    let success_count = results.iter()
        .filter(|r| r.is_ok() && r.as_ref().unwrap().1.is_ok())
        .count();

    // 至少 95% 成功率
    assert!(success_count >= 95, "并发连接成功率: {}/100", success_count);
}
```

### 7.3 内存压力测试

```rust
// tests/memory_tests.rs
#[tokio::test]
async fn test_memory_under_load() {
    let initial_memory = get_memory_usage();

    // 创建大量会话
    let mut sessions = Vec::new();
    for i in 0..1000 {
        let session = create_session(&format!("session-{}", i)).await;
        sessions.push(session);
    }

    let peak_memory = get_memory_usage();

    // 释放会话
    drop(sessions);

    // 强制 GC (如果适用)
    #[cfg(feature = "jemalloc")]
    jemalloc_ctl::epoch::advance().unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    let final_memory = get_memory_usage();

    // 验证内存释放
    let leaked = final_memory - initial_memory;
    assert!(leaked < 10_000_000, "内存泄漏: {} bytes", leaked); // < 10MB
}
```

---

## 8. 安全测试

### 8.1 密码学测试

```rust
// tests/security_tests.rs
#[test]
fn test_encryption_not_deterministic() {
    let data = b"test data";
    let key = derive_key("password", &[1u8; 16]);

    let encrypted1 = encrypt(data, &key).unwrap();
    let encrypted2 = encrypt(data, &key).unwrap();

    // 相同数据加密结果应该不同 (使用随机 nonce)
    assert_ne!(encrypted1, encrypted2);
}

#[test]
fn test_side_channel_resistance() {
    let key1 = derive_key("password1", &[1u8; 16]);
    let key2 = derive_key("password2", &[2u8; 16]);
    let data = b"test data";

    let encrypted = encrypt(data, &key1).unwrap();

    // 使用常量时间比较
    let start = Instant::now();
    let result1 = constant_time_decrypt(&encrypted, &key1);
    let duration1 = start.elapsed();

    let start = Instant::now();
    let result2 = constant_time_decrypt(&encrypted, &key2);
    let duration2 = start.elapsed();

    // 时间差应该小于阈值
    let diff = if duration1 > duration2 {
        duration1 - duration2
    } else {
        duration2 - duration1
    };

    assert!(diff < Duration::from_micros(100));
}
```

### 8.2 输入验证测试

```rust
#[test]
fn test_sql_injection_prevention() {
    let malicious_name = "'; DROP TABLE servers; --";
    let server = Server::new(malicious_name, "host", 22);

    // 应该正常存储，不会执行 SQL
    let db = setup_test_db();
    let id = db.add_server(&server).unwrap();

    let retrieved = db.get_server(&id).unwrap();
    assert_eq!(retrieved.name, malicious_name);

    // 验证表仍然存在
    let count = db.count_servers().unwrap();
    assert!(count >= 1);
}

#[test]
fn test_command_injection_prevention() {
    let malicious_cmd = "; rm -rf /; echo ";

    // 命令应该被正确转义
    let sanitized = sanitize_command(malicious_cmd);
    assert!(!sanitized.contains(';'));
}
```

### 8.3 模糊测试

```rust
// 使用 cargo-fuzz
#[macro_use]
extern crate libfuzzer_sys;

fuzz_target!(|data: &[u8]| {
    // 测试配置解析
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = SshConfig::from_str(s);
    }

    // 测试密钥解析
    let _ = parse_private_key(data);
});
```

---

## 9. 测试工具

### 9.1 常用 Cargo 工具

```bash
# 代码覆盖率
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --out Lcov

# 突变测试
cargo install cargo-mutants
cargo mutants

# 性能测试
cargo install cargo-criterion

# 模糊测试
cargo install cargo-fuzz

# 测试并行化
cargo install cargo-nextest
cargo nextest run
```

### 9.2 测试 fixtures

```rust
// tests/fixtures/mod.rs
pub struct TestServerBuilder {
    config: SshConfig,
}

impl TestServerBuilder {
    pub fn new() -> Self {
        Self {
            config: SshConfig {
                host: "localhost".to_string(),
                port: 2222,
                username: "test".to_string(),
                auth: AuthMethod::Password("test".to_string()),
            },
        }
    }

    pub fn with_host(mut self, host: &str) -> Self {
        self.config.host = host.to_string();
        self
    }

    pub fn with_key_auth(mut self, key_path: &Path) -> Self {
        self.config.auth = AuthMethod::Key(key_path.to_path_buf());
        self
    }

    pub fn build(self) -> SshConfig {
        self.config
    }
}

// 使用
let config = TestServerBuilder::new()
    .with_host("192.168.1.1")
    .build();
```

### 9.3 测试配置

```toml
# Cargo.toml (dev-dependencies)
[dev-dependencies]
tokio-test = "0.4"
mockall = "0.12"
tempfile = "3"
proptest = "1"
criterion = { version = "0.5", features = ["async_tokio"] }
testcontainers = "0.15"
wiremock = "0.6"

# 集成测试配置
[[test]]
name = "integration"
path = "tests/integration/main.rs"
```

---

## 10. CI/CD 集成

### 10.1 GitHub Actions 工作流

```yaml
# .github/workflows/test.yml
name: Test Suite

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          components: rustfmt, clippy

      - name: Run unit tests
        run: cargo test --lib --all-features

      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml

      - uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml

  integration-tests:
    runs-on: ubuntu-latest
    services:
      sshd:
        image: linuxserver/openssh-server
        ports:
          - 2222:2222
    steps:
      - uses: actions/checkout@v4

      - name: Wait for SSHD
        run: |
          until nc -z localhost 2222; do
            sleep 1
          done

      - name: Run integration tests
        run: cargo test --test integration_tests
        env:
          TEST_SSH_HOST: localhost
          TEST_SSH_PORT: 2222

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-4-dev libadwaita-1-dev

      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Install Playwright
        run: |
          cd platforms/desktop
          pnpm install
          pnpm exec playwright install

      - name: Run E2E tests
        run: |
          cd platforms/desktop
          pnpm test:e2e

      - uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: playwright-report
          path: platforms/desktop/playwright-report/
```

### 10.2 预提交钩子

```bash
#!/bin/sh
# .git/hooks/pre-commit

echo "Running pre-commit checks..."

# 格式化检查
cargo fmt -- --check || exit 1

# Clippy 检查
cargo clippy -- -D warnings || exit 1

# 单元测试
cargo test --lib --quiet || exit 1

echo "Pre-commit checks passed!"
```

---

## 11. 测试最佳实践

### 11.1 测试命名规范

```rust
// ✅ 清晰描述测试目的
#[test]
fn test_encrypt_decrypt_roundtrip_succeeds() {}

#[test]
fn test_decrypt_with_wrong_key_fails_with_decryption_error() {}

#[test]
fn test_empty_password_returns_validation_error() {}

// ❌ 避免模糊的命名
#[test]
fn test1() {}
#[test]
fn it_works() {}
```

### 11.2 AAA 模式 (Arrange-Act-Assert)

```rust
#[test]
fn test_server_group_assignment() {
    // Arrange
    let server = create_test_server("Test Server");
    let group = create_test_group("Production");
    let manager = ServerManager::new();

    // Act
    manager.add_server(&server).unwrap();
    manager.assign_to_group(&server.id, &group.id).unwrap();

    // Assert
    let assigned_group = manager.get_server_group(&server.id).unwrap();
    assert_eq!(assigned_group.id, group.id);
}
```

---

## 12. 故障排除

### 12.1 常见测试问题

#### GTK4/Linux依赖缺失

```bash
# 错误信息
error: failed to run custom build command for `gobject-sys v0.19.8`
Could not run `pkg-config --libs --cflags gobject-2.0`

# 解决方案 (Windows)
# GTK4测试在Windows上无法运行，请使用Linux环境或跳过GTK4测试
cargo test -p easyssh-core --lib --tests  # 跳过GTK4

# 解决方案 (Linux)
sudo apt-get install -y libgtk-4-dev libadwaita-1-dev pkg-config
```

#### Rust版本不兼容

```bash
# 错误信息
error: rustc 1.89.0 is not supported by the following packages:
  aws-config@1.8.15 requires rustc 1.91.1

# 解决方案
# Pro版本需要更新的Rust版本，使用Lite/Standard版本测试
cargo test -p easyssh-core --lib --tests --no-default-features --features "lite,standard"

# 或升级Rust版本
rustup update stable
```

#### Example编译失败

```bash
# 错误信息
error[E0432]: unresolved import `crossterm`
error[E0599]: no method named `execute` found for struct `Stdout`

# 原因
# examples依赖crossterm等终端库，需要完整运行环境
# examples目录下的文件不是测试的一部分

# 解决方案 (推荐)
# 使用 --lib --tests 标志跳过examples编译
cargo test -p easyssh-core --lib
cargo test -p easyssh-core --tests
cargo test -p easyssh-core --lib --tests  # 同时运行两种测试

# 如果需要编译examples，确保安装完整依赖
cargo build -p easyssh-core --examples
```

#### 数据库测试失败

```bash
# 错误信息
database tests failed: unable to open database file

# 解决方案
# 确保测试目录有写入权限
# 使用临时目录创建测试数据库
export TEST_DB_DIR=/tmp/easyssh-test
cargo test database
```

#### 并行测试冲突

```bash
# 错误信息
test failed: database is locked

# 解决方案
# 减少并行测试线程数
cargo test -- --test-threads=1

# 或使用nextest（更好的并行控制）
cargo nextest run
```

### 12.2 测试调试技巧

```bash
# 运行单个测试并显示输出
cargo test test_crypto_state_new_is_locked -- --nocapture

# 显示测试执行详细信息
cargo test -- --verbose

# 仅运行失败的测试
cargo test -- --failed

# 忽略的测试（包含需要Docker的测试）
cargo test -- --include-ignored

# 特定模块的测试
cargo test -p easyssh-core crypto::tests
cargo test -p easyssh-core services::group_service::tests
```

### 12.3 测试环境设置

| 环境 | 必需依赖 | 推荐工具 |
|------|----------|----------|
| **Lite测试** | 无特殊依赖 | cargo-test |
| **Standard测试** | SQLite | cargo-nextest |
| **Pro测试** | Rust 1.91+, Docker | cargo-nextest, testcontainers |
| **GTK4测试** | GTK4, pkg-config | Linux环境 |
| **Windows UI测试** | Windows SDK | Windows环境 |
| **E2E测试** | Node.js, Playwright | pnpm |

### 12.4 忽略的测试说明

当前有10个被忽略的测试，主要原因：

| 类别 | 原因 | 运行方式 |
|------|------|----------|
| Docker容器测试 | 需要Docker环境 | `cargo test -- --include-ignored` |
| SSH连接测试 | 需要SSH服务器 | 配置TEST_SSH_HOST/PORT环境变量 |
| 网络测试 | 需要网络连接 | 在CI环境中运行 |
| 性能基准测试 | 长时间运行 | `cargo bench`单独运行 |

---

## 13. 相关文档

- [设置指南](./SETUP.md) - 环境配置
- [调试指南](./DEBUGGING.md) - 故障排查
- [性能分析指南](./PROFILING.md) - 性能优化
- [故障排除指南](./TROUBLESHOOTING.md) - 常见问题
- [核心测试框架](../../crates/easyssh-core/tests/README.md) - EasySSH Core测试框架

---

*最后更新: 2026-04-03*
