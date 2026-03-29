# 代码质量标准

> 商用级代码健壮性、可维护性、可测试性规范

---

## 1. Rust代码标准

### 1.1 错误处理

```rust
// ❌ 避免: 使用unwrap()和expect()
fn bad_example() -> Result<String, Error> {
    let value = some_function()?;
    Ok(value.unwrap())  // 可能panic
}

// ✅ 推荐: 使用?和自定义错误类型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("SSH connection failed: {0}")]
    SshConnection(#[from] ssh2::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

fn good_example() -> Result<String, AppError> {
    let value = some_function()?;
    Ok(value)
}

// ✅ 使用anyhow用于库代码，thiserror用于应用代码
```

### 1.2 异步编程

```rust
// ❌ 避免: 阻塞在async上下文中
async fn bad_async() {
    let data = std::fs::read("file.txt").unwrap(); // 阻塞!
}

// ✅ 推荐: 使用tokio异步IO
async fn good_async() -> Result<Vec<u8>, std::io::Error> {
    tokio::fs::read("file.txt").await
}

// ✅ 所有Tauri command必须是async但不能跨await持有锁
#[tauri::command]
async fn correct_command(state: State<'_, AppState>) -> Result<(), String> {
    // ✅ 在单个async fn中获取锁后立即操作
    let result = {
        let guard = state.ssh_manager.lock().await;
        guard.some_operation()?
    };
    Ok(result)
}
```

### 1.3 所有权和生命周期

```rust
// ❌ 避免: 不必要的clone
fn bad_ownership(data: String) -> String {
    let cloned = data.clone(); // 不必要
    cloned
}

// ✅ 推荐: 借用或移动
fn good_ownership(data: &str) -> String {
    data.to_uppercase()
}

// ✅ 使用Arc用于共享所有权
struct SharedState {
    data: Arc<Mutex<Vec<String>>>,
}
```

### 1.4 依赖管理

```toml
# ❌ 避免: 依赖过多的大型crate
tokio = { version = "1", features = ["full"] }  # 拉取整个tokio

# ✅ 推荐: 只启用需要的feature
tokio = { version = "1", features = ["rt", "macros", "sync"] }
```

---

## 2. TypeScript代码标准

### 2.1 类型安全

```typescript
// ❌ 避免: any类型
function badFunction(data: any) {
    return data.value; // 运行时错误风险
}

// ✅ 推荐: 完整类型定义
interface ServerConfig {
    id: string;
    host: string;
    port: number;
    auth: AuthMethod;
}

function goodFunction(data: ServerConfig): string {
    return data.host;
}

// ✅ 使用类型守卫
function isServerConfig(data: unknown): data is ServerConfig {
    return (
        typeof data === 'object' &&
        data !== null &&
        'host' in data &&
        'port' in data
    );
}
```

### 2.2 React组件规范

```typescript
// ❌ 避免: 内联函数导致重渲染
function BadComponent({ onClick }: { onClick: () => void }) {
    return <button onClick={() => onClick()}>Click</button>;
}

// ✅ 推荐: useCallback + 组件拆分
const Button = React.memo(({ onClick, children }: ButtonProps) => {
    return <button onClick={onClick}>{children}</button>;
});

function GoodComponent() {
    const handleClick = useCallback(() => {
        // 处理逻辑
    }, []);

    return <Button onClick={handleClick}>Click</Button>;
}

// ✅ Props接口必须显式定义
interface ServerItemProps {
    server: Server;
    isSelected: boolean;
    onConnect: (id: string) => void;
    onEdit: (id: string) => void;
}
```

### 2.3 状态管理

```typescript
// ✅ Zustand store规范
interface ServerStore {
    servers: Server[];
    addServer: (server: Omit<Server, 'id'>) => void;
    updateServer: (id: string, updates: Partial<Server>) => void;
    deleteServer: (id: string) => void;
}

export const useServerStore = create<ServerStore>((set) => ({
    servers: [],

    addServer: (server) =>
        set((state) => ({
            servers: [...state.servers, { ...server, id: crypto.randomUUID() }],
        })),

    updateServer: (id, updates) =>
        set((state) => ({
            servers: state.servers.map((s) =>
                s.id === id ? { ...s, ...updates } : s
            ),
        })),

    deleteServer: (id) =>
        set((state) => ({
            servers: state.servers.filter((s) => s.id !== id),
        })),
}));
```

---

## 3. 测试标准

### 3.1 Rust测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ✅ 单元测试
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let data = b"sensitive data";
        let key = derive_key("password", &[1, 2, 3, 4]);

        let encrypted = encrypt(data, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();

        assert_eq!(data.as_slice(), decrypted.as_slice());
    }

    // ✅ 测试错误情况
    #[test]
    fn test_invalid_key_fails() {
        let encrypted = encrypt(b"data", &key1).unwrap();
        let result = decrypt(&encrypted, &key2); // 不同的key

        assert!(result.is_err());
    }

    // ✅ Property-based测试
    #[test]
    fn test_encryption_deterministic() {
        // 相同的输入+key应该产生相同的输出
        // 或者验证每次生成不同的nonce
    }
}
```

### 3.2 TypeScript测试

```typescript
// ✅ 使用Vitest
describe('ServerStore', () => {
    it('should add a server', () => {
        const store = createServerStore();

        store.getState().addServer({
            name: 'Test',
            host: 'localhost',
            port: 22,
            // ...
        });

        expect(store.getState().servers).toHaveLength(1);
    });

    it('should update server color', () => {
        const store = createServerStore();
        store.getState().addServer(testServer);

        store.getState().updateServer(testServer.id, { color: '#FF0000' });

        expect(store.getState().servers[0].color).toBe('#FF0000');
    });
});

// ✅ Playwright E2E测试
test('user can connect to server', async ({ page }) => {
    await page.goto('/');

    // 添加服务器
    await page.click('[data-testid="add-server"]');
    await page.fill('#host', '192.168.1.10');
    await page.fill('#username', 'admin');

    // 连接
    await page.click('text=Connect');

    // 验证终端打开
    await expect(page.locator('[data-testid="terminal"]')).toBeVisible();
});
```

---

## 4. 代码审查标准

### 4.1 PR Checklist

```
□ 代码符合本指南的命名规范
□ 所有公共API都有文档注释
□ 新代码有对应的测试
□ 测试覆盖率没有下降
□ 没有TODO注释遗留
□ 没有console.log/debugger遗留
□ Rust: cargo clippy通过
□ TypeScript: tsc --noEmit通过
□ 构建成功
□ E2E测试通过 (如果涉及UI)
```

### 4.2 Review要点

1. **错误处理**: 错误是否被正确传播？
2. **性能**: 是否有明显的性能问题？
3. **安全性**: 是否有安全漏洞？
4. **可维护性**: 未来容易修改吗？
5. **测试**: 是否有必要的测试覆盖？

---

## 5. 性能标准

### 5.1 Rust性能

```rust
// ✅ 使用#[inline]避免小函数调用开销
#[inline]
fn small_operation(&self) -> u32 {
    self.value
}

// ✅ 使用String vs &str的正确场景
fn process_owned(s: String) { /* 需要所有权 */ }
fn process_borrowed(s: &str) { /* 仅需要引用 */ }

// ✅ 避免在热路径中分配
fn hot_path() {
    // 预分配buffer
    let mut buffer = Vec::with_capacity(1024);

    // 复用buffer
    buffer.clear();
    // ... 使用buffer
}
```

### 5.2 前端性能

```typescript
// ✅ React性能优化
const MemoizedComponent = React.memo(
    UnmemoizedComponent,
    (prevProps, nextProps) => {
        // 精细比较
        return prevProps.server.id === nextProps.server.id;
    }
);

// ✅ 虚拟列表长列表
const VirtualList = ({ items }) => (
    <FixedSizeList
        height={400}
        itemCount={items.length}
        itemSize={50}
    >
        {({ index, style }) => (
            <ListItem style={style}>{items[index]}</ListItem>
        )}
    </FixedSizeList>
);

// ✅ 懒加载
const HeavyComponent = lazy(() => import('./HeavyComponent'));
```

---

## 6. 安全标准

### 6.1 Rust安全

```rust
// ✅ 不在日志中打印敏感信息
fn connect_ssh(config: &SshConfig) {
    tracing::info!("Connecting to {}", config.host);
    // ❌ tracing::debug!("Password: {}", config.password); // 禁止!
}

// ✅ 使用安全的随机数
use rand::rngs::OsRng;
let random_bytes: [u8; 32] = rand::Fill::fill(&mut OsRng);

// ✅ 常量时间比较 (密码等)
use subtle::ConstantTimeEq;
if a.ct_eq(&b).unwrap_u8() == 1 { /* 密码正确 */ }
```

### 6.2 前端安全

```typescript
// ✅ 不在localStorage中存储敏感信息
localStorage.setItem('theme', 'dark'); // OK
// localStorage.setItem('token', sensitiveToken); // 禁止!

// ✅ 使用httpOnly Cookie存储Token

// ✅ 验证所有用户输入
const validatedHost = z.string()
    .ip({ version: 'v4', message: 'Invalid IP' })
    .parse(userInput);
```
