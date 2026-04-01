# EasySSH 测试指南

> 全面的测试策略、工具和实践方法

---

## 目录

1. [测试概述](#1-测试概述)
2. [单元测试](#2-单元测试)
3. [集成测试](#3-集成测试)
4. [端到端测试](#4-端到端测试)
5. [性能测试](#5-性能测试)
6. [安全测试](#6-安全测试)
7. [测试工具](#7-测试工具)
8. [CI/CD 集成](#8-cicd-集成)

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

## 2. 单元测试

### 2.1 Rust 单元测试

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

### 2.2 异步测试

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

### 2.3 Mock 测试

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

### 2.4 TypeScript 单元测试

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
    store.getState().addServer(testServer);
    const id = store.getState().servers[0].id;

    store.getState().updateServer(id, { color: '#FF0000' });

    expect(store.getState().servers[0].color).toBe('#FF0000');
  });

  it('should handle invalid updates', () => {
    store.getState().addServer(testServer);

    expect(() => {
      store.getState().updateServer('invalid-id', { name: 'New Name' });
    }).not.toThrow();

    expect(store.getState().servers[0].name).toBe(testServer.name);
  });
});
```

---

## 3. 集成测试

### 3.1 SSH 集成测试

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

### 3.2 数据库集成测试

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

### 3.3 API 集成测试

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

## 4. 端到端测试

### 4.1 Playwright E2E 测试

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

### 4.2 视觉回归测试

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

### 4.3 可访问性测试

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

## 5. 性能测试

### 5.1 负载测试

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

### 5.2 并发测试

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

### 5.3 内存压力测试

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

## 6. 安全测试

### 6.1 密码学测试

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

### 6.2 输入验证测试

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

### 6.3 模糊测试

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

## 7. 测试工具

### 7.1 常用 Cargo 工具

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

### 7.2 测试 fixtures

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

### 7.3 测试配置

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

## 8. CI/CD 集成

### 8.1 GitHub Actions 工作流

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

### 8.2 预提交钩子

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

## 9. 测试最佳实践

### 9.1 测试命名规范

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

### 9.2 AAA 模式 (Arrange-Act-Assert)

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

## 10. 相关文档

- [设置指南](./SETUP.md) - 环境配置
- [调试指南](./DEBUGGING.md) - 故障排查
- [性能分析指南](./PROFILING.md) - 性能优化
- [故障排除指南](./TROUBLESHOOTING.md) - 常见问题

---

*最后更新: 2026-04-01*
