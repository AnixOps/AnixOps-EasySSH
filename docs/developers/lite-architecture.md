# EasySSH Lite 架构文档
# EasySSH Lite Architecture

> **English Version**: [Jump to English Section](#architecture-overview)

---

## 架构概览 / Architecture Overview

EasySSH Lite 采用纯原生 UI 架构，为每个平台提供最优性能和用户体验。

```
┌─────────────────────────────────────────────────────────────────────┐
│                        EasySSH Lite v0.3.0                           │
│                        整体架构图                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   ┌──────────────┐     ┌──────────────┐     ┌──────────────┐      │
│   │  Windows UI  │     │   Linux UI    │     │   macOS UI   │      │
│   │  (egui)      │     │  (GTK4)       │     │  (SwiftUI)   │      │
│   │              │     │               │     │              │      │
│   │ ┌──────────┐ │     │ ┌──────────┐  │     │ ┌──────────┐ │      │
│   │ │egui      │ │     │ │GTK4      │  │     │ │SwiftUI   │ │      │
│   │ │widgets   │ │     │ │widgets   │  │     │ │views     │ │      │
│   │ └──────────┘ │     │ └──────────┘  │     │ └──────────┘ │      │
│   └──────┬───────┘     └──────┬───────┘     └──────┬───────┘      │
│          │                    │                    │               │
│          └────────────────────┼────────────────────┘               │
│                               │                                    │
│          ┌────────────────────┴────────────────────┐               │
│          │           Core Library (Rust)          │               │
│          │                                          │               │
│          │  ┌──────────┐  ┌──────────┐  ┌────────┐ │               │
│          │  │Crypto    │  │SSH       │  │Config  │ │               │
│          │  │(AES/     │  │(ssh2/    │  │(SQLite │ │               │
│          │  │Argon2)   │  │russh)    │  │/JSON)  │ │               │
│          │  └──────────┘  └──────────┘  └────────┘ │               │
│          │                                          │               │
│          │  ┌──────────┐  ┌──────────┐  ┌────────┐ │               │
│          │  │Keychain  │  │Search    │  │Import/ │ │               │
│          │  │(keyring) │  │(fuzzy)   │  │Export  │ │               │
│          │  └──────────┘  └──────────┘  └────────┘ │               │
│          │                                          │               │
│          └──────────────────────────────────────────┘               │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 一、模块结构 / Module Structure

### 1.1 Monorepo 布局

```
easyssh/
├── Cargo.toml                 # Workspace 根配置
│
├── crates/
│   ├── core/                  # 核心库 (平台无关)
│   │   ├── src/
│   │   │   ├── crypto/        # 加密模块
│   │   │   ├── config/        # 配置管理
│   │   │   ├── ssh/           # SSH 处理
│   │   │   ├── search/        # 搜索功能
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   │
│   ├── lite-egui/             # Windows egui 版本
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── ui/
│   │   │   │   ├── server_list.rs
│   │   │   │   ├── group_tree.rs
│   │   │   │   ├── add_server.rs
│   │   │   │   └── mod.rs
│   │   │   └── app.rs
│   │   └── Cargo.toml
│   │
│   ├── lite-gtk/              # Linux GTK4 版本
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── ui/
│   │   │   │   ├── window.rs
│   │   │   │   ├── server_list.rs
│   │   │   │   └── mod.rs
│   │   │   └── app.rs
│   │   └── Cargo.toml
│   │
│   └── lite-swift/            # macOS SwiftUI 版本
│       ├── Sources/
│       │   ├── EasySSH/
│       │   │   ├── App.swift
│       │   │   ├── Views/
│       │   │   └── Models/
│       │   └── RustBridge/
│       └── Package.swift
│
└── docs/                      # 文档
```

### 1.2 核心模块依赖图

```
                    ┌──────────────┐
                    │  lite-egui   │
                    │  lite-gtk    │
                    │  lite-swift  │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  core-lib    │
                    │  (公共接口)   │
                    └──────┬───────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼─────┐      ┌────▼─────┐      ┌────▼─────┐
   │ crypto   │      │   ssh    │      │  config  │
   │ module   │      │  module  │      │  module  │
   └──────────┘      └──────────┘      └──────────┘
        │                  │                  │
   ┌────▼─────┐      ┌────▼─────┐      ┌────▼─────┐
   │ ring     │      │ ssh2     │      │ sqlite   │
   │ argon2   │      │ russh    │      │ serde    │
   │ aes-gcm  │      │          │      │          │
   └──────────┘      └──────────┘      └──────────┘
```

---

## 二、核心模块详解 / Core Modules

### 2.1 加密模块 (crypto)

```rust
// crates/core/src/crypto/mod.rs

//! 加密模块 - 提供军用级数据保护
//! Crypto Module - Military-grade data protection

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};

/// 主密码哈希参数 (OWASP 推荐)
/// Master password hashing parameters
pub struct KdfParams {
    pub memory_cost: u32,      // 64 MB
    pub time_cost: u32,        // 3 iterations
    pub parallelism: u32,      // 4 threads
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            memory_cost: 65536,
            time_cost: 3,
            parallelism: 4,
        }
    }
}

/// 加密管理器
/// Encryption manager
pub struct CryptoManager {
    master_key: Option<Key<Aes256Gcm>>,
    kdf_params: KdfParams,
}

impl CryptoManager {
    /// 使用主密码初始化
    /// Initialize with master password
    pub fn init(&mut self, password: &str, salt: &[u8]) -> Result<()> {
        // Argon2id 密钥派生
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(
                self.kdf_params.memory_cost,
                self.kdf_params.time_cost,
                self.kdf_params.parallelism,
                Some(32),
            )?,
        );

        let mut master_key = [0u8; 32];
        argon2.hash_password_into(
            password.as_bytes(),
            salt,
            &mut master_key,
        )?;

        self.master_key = Some(Key::from_slice(&master_key).clone());
        Ok(())
    }

    /// 加密数据
    /// Encrypt data
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData> {
        let cipher = Aes256Gcm::new(self.master_key.as_ref().unwrap());
        let nonce = generate_secure_random(12);
        let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce), plaintext)?;

        Ok(EncryptedData { nonce, ciphertext })
    }

    /// 解密数据
    /// Decrypt data
    pub fn decrypt(&self, data: &EncryptedData) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(self.master_key.as_ref().unwrap());
        cipher.decrypt(
            Nonce::from_slice(&data.nonce),
            data.ciphertext.as_ref(),
        )
        .map_err(|e| e.into())
    }
}
```

### 2.2 配置管理模块 (config)

```rust
// crates/core/src/config/mod.rs

//! 配置管理模块 - 服务器配置 CRUD
//! Config module - Server configuration CRUD

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::path::PathBuf;

/// 服务器配置结构
/// Server configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
    pub group_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub connection_options: ConnectionOptions,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_connected: Option<chrono::DateTime<chrono::Utc>>,
}

/// 认证方式枚举
/// Authentication method enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    #[serde(rename = "password")]
    Password {
        /// 密码存储在系统 keychain 中
        /// Password stored in system keychain
        keychain_entry: String,
    },
    #[serde(rename = "key")]
    SshKey {
        private_key_path: PathBuf,
        /// 密钥口令存储在 keychain
        /// Passphrase stored in keychain
        passphrase_entry: Option<String>,
        public_key_path: Option<PathBuf>,
    },
    #[serde(rename = "agent")]
    Agent {
        /// 使用 SSH Agent 中的密钥
        /// Use key from SSH agent
        key_fingerprint: Option<String>,
    },
}

/// 连接选项
/// Connection options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionOptions {
    pub timeout_seconds: u64,
    pub keepalive_interval: u64,
    pub retry_attempts: u32,
    pub compression: bool,
    pub strict_host_key_checking: bool,
}

/// 配置管理器
/// Configuration manager
pub struct ConfigManager {
    db: SqliteConnection,
    crypto: CryptoManager,
    cache: ConfigCache,
}

impl ConfigManager {
    /// 创建服务器配置
    /// Create server configuration
    pub fn create_server(&mut self, config: ServerConfig) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let encrypted = self.crypto.encrypt(
            &serde_json::to_vec(&config)?
        )?;

        self.db.execute(
            "INSERT INTO servers (id, encrypted_data, created_at) VALUES (?1, ?2, ?3)",
            params![id.to_string(), encrypted.to_bytes(), chrono::Utc::now()],
        )?;

        self.cache.invalidate();
        Ok(id)
    }

    /// 获取所有服务器
    /// Get all servers
    pub fn get_servers(&self, filter: Option<ServerFilter>) -> Result<Vec<ServerConfig>> {
        let query = build_query(filter);
        let rows = self.db.prepare(&query)?.query_map(params![], |row| {
            let encrypted_data: Vec<u8> = row.get(1)?;
            let decrypted = self.crypto.decrypt(&EncryptedData::from_bytes(&encrypted_data))?;
            Ok(serde_json::from_slice(&decrypted)?)
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
    }
}
```

### 2.3 钥匙串集成模块 (keychain)

```rust
// crates/core/src/keychain/mod.rs

//! 钥匙串集成模块 - 安全凭证存储
//! Keychain integration - Secure credential storage

use keyring::Entry;
use secrecy::{ExposeSecret, SecretString};

/// 钥匙串服务名称
const SERVICE_NAME: &str = "com.anixops.easyssh-lite";

/// 安全凭证存储
/// Secure credential storage
pub struct KeychainStorage;

impl KeychainStorage {
    /// 存储密码
    /// Store password
    pub fn store_password(key: &str, password: &SecretString) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, key)?;
        entry.set_password(password.expose_secret())?;
        Ok(())
    }

    /// 获取密码
    /// Get password
    pub fn get_password(key: &str) -> Result<Option<SecretString>> {
        let entry = Entry::new(SERVICE_NAME, key)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(SecretString::new(password))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 删除密码
    /// Delete password
    pub fn delete_password(key: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, key)?;
        entry.delete_password()?;
        Ok(())
    }
}
```

### 2.4 搜索模块 (search)

```rust
// crates/core/src/search/mod.rs

//! 搜索模块 - 模糊搜索和过滤
//! Search module - Fuzzy search and filtering

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

/// 搜索引擎
/// Search engine
pub struct SearchEngine {
    matcher: SkimMatcherV2,
    index: SearchIndex,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            index: SearchIndex::new(),
        }
    }

    /// 模糊搜索服务器
    /// Fuzzy search servers
    pub fn search(&self, query: &str, servers: &[ServerConfig]) -> Vec<SearchResult> {
        servers
            .iter()
            .filter_map(|server| {
                // 搜索名称
                let name_score = self.matcher.fuzzy_match(&server.name, query);
                // 搜索主机
                let host_score = self.matcher.fuzzy_match(&server.host, query);
                // 搜索标签
                let tag_score = server.tags.iter()
                    .filter_map(|tag| self.matcher.fuzzy_match(tag, query))
                    .max();

                let best_score = name_score
                    .max(host_score)
                    .max(tag_score.unwrap_or(0));

                if best_score > 0 {
                    Some(SearchResult {
                        server: server.clone(),
                        score: best_score,
                        matched_fields: self.get_matched_fields(server, query),
                    })
                } else {
                    None
                }
            })
            .sorted_by(|a, b| b.score.cmp(&a.score))
            .collect()
    }

    /// 高级过滤
    /// Advanced filtering
    pub fn filter(&self, servers: &[ServerConfig], criteria: FilterCriteria) -> Vec<ServerConfig> {
        servers
            .iter()
            .filter(|s| {
                // 分组过滤
                let group_match = criteria.group_id
                    .map(|id| s.group_id == Some(id))
                    .unwrap_or(true);

                // 标签过滤
                let tag_match = criteria.tags
                    .as_ref()
                    .map(|tags| tags.iter().all(|t| s.tags.contains(t)))
                    .unwrap_or(true);

                // 认证方式过滤
                let auth_match = criteria.auth_type
                    .map(|auth| std::mem::discriminant(&s.auth_method) == std::mem::discriminant(&auth))
                    .unwrap_or(true);

                group_match && tag_match && auth_match
            })
            .cloned()
            .collect()
    }
}
```

---

## 三、平台适配层 / Platform Adaptation Layer

### 3.1 Windows (egui)

```rust
// crates/lite-egui/src/app.rs

//! Windows egui 应用主入口
//! Windows egui application entry

use eframe::egui;
use easyssh_core::{ConfigManager, CryptoManager};

pub struct EasySshLiteApp {
    config: ConfigManager,
    crypto: CryptoManager,
    ui_state: UiState,
}

impl eframe::App for EasySshLiteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 顶部菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.render_menu(ui);
        });

        // 左侧服务器列表
        egui::SidePanel::left("server_list")
            .default_width(250.0)
            .show(ctx, |ui| {
                self.render_server_list(ui);
            });

        // 中央详情区域
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_detail_view(ui);
        });
    }
}

/// 唤起 Windows 终端
/// Launch Windows terminal
fn launch_windows_terminal(server: &ServerConfig) -> Result<()> {
    let ssh_cmd = build_ssh_command(server);

    // 优先尝试 Windows Terminal
    if let Ok(wt) = which::which("wt") {
        std::process::Command::new(wt)
            .arg("new-tab")
            .arg("--title")
            .arg(&server.name)
            .arg("ssh")
            .args(ssh_cmd.split_whitespace())
            .spawn()?;
    } else {
        // 回退到 PowerShell
        std::process::Command::new("powershell")
            .arg("-Command")
            .arg(format!("ssh {}", ssh_cmd))
            .spawn()?;
    }

    Ok(())
}
```

### 3.2 Linux (GTK4)

```rust
// crates/lite-gtk/src/main.rs

//! Linux GTK4 应用主入口
//! Linux GTK4 application entry

use gtk4::prelude::*;
use adw::prelude::*;

fn main() {
    let app = adw::Application::builder()
        .application_id("com.anixops.EasySSHLite")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("EasySSH Lite")
        .default_width(900)
        .default_height(600)
        .build();

    // 创建主布局
    let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    paned.set_position(250);

    // 左侧: 服务器列表
    let sidebar = build_server_list_sidebar();
    paned.set_start_child(Some(&sidebar));

    // 右侧: 详情视图
    let content = build_detail_view();
    paned.set_end_child(Some(&content));

    window.set_content(Some(&paned));
    window.present();
}

/// 唤起 Linux 终端
/// Launch Linux terminal
fn launch_linux_terminal(server: &ServerConfig) -> Result<()> {
    let ssh_cmd = build_ssh_command(server);

    // 检测可用的终端
    let terminals = vec![
        ("gnome-terminal", vec!["--", "ssh"]),
        ("konsole", vec!["-e", "ssh"]),
        ("alacritty", vec!["-e", "ssh"]),
        ("xterm", vec!["-e", "ssh"]),
    ];

    for (term, args) in terminals {
        if which::which(term).is_ok() {
            let mut cmd = std::process::Command::new(term);
            cmd.args(&args);
            cmd.arg(&format!("{}@{}", server.username, server.host));
            cmd.spawn()?;
            return Ok(());
        }
    }

    Err(Error::NoTerminalFound)
}
```

### 3.3 macOS (SwiftUI + Rust Bridge)

```swift
// crates/lite-swift/Sources/EasySSH/App.swift

import SwiftUI
import Foundation

@main
struct EasySSHLiteApp: App {
    @StateObject private var appState = AppState()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
        }
        .commands {
            CommandMenu("服务器") {
                Button("新建服务器") {
                    appState.showAddServer = true
                }
                .keyboardShortcut("n", modifiers: .command)

                Button("连接") {
                    appState.connectSelected()
                }
                .keyboardShortcut(.return, modifiers: .command)
            }
        }
    }
}

// Rust FFI Bridge
// Rust FFI 桥接
class RustBridge {
    static let shared = RustBridge()

    // 调用 Rust 核心库
    func loadServers() -> [Server] {
        let cString = rust_load_servers()
        let jsonString = String(cString: cString!)
        rust_free_string(cString)

        let data = jsonString.data(using: .utf8)!
        return try! JSONDecoder().decode([Server].self, from: data)
    }

    func connect(to server: Server) {
        var terminal = TerminalPreferences.shared.preferred

        switch terminal {
        case .iterm2:
            launchIterm2(server: server)
        case .terminal:
            launchTerminal(server: server)
        case .alacritty:
            launchAlacritty(server: server)
        }
    }

    private func launchIterm2(server: Server) {
        let script = """
        tell application "iTerm"
            set newWindow to (create window with default profile)
            tell current session of newWindow
n                write text "ssh -p \(server.port) \(server.username)@\(server.host)"
            end tell
        end tell
        """

        var error: NSDictionary?
        NSAppleScript(source: script)?.executeAndReturnError(&error)
    }
}
```

---

## 四、数据流设计 / Data Flow Design

### 4.1 配置加载流程

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   启动应用    │────→│  检查数据库   │────→│  请求主密码  │
└──────────────┘     └──────────────┘     └──────┬───────┘
                                                  │
                                                  ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   解密配置    │←────│  Argon2 派生 │←────│  输入主密码   │
│   加载内存    │     │  解密密钥    │     │              │
└──────┬───────┘     └──────────────┘     └──────────────┘
       │
       ▼
┌──────────────┐
│  构建 UI 列表 │
│  缓存到内存   │
└──────────────┘
```

### 4.2 连接流程

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  双击服务器   │────→│  检查 Agent  │────→│  加载密钥    │
└──────────────┘     │  密钥状态     │     │  (如需要)    │
                     └──────┬───────┘     └──────┬───────┘
                            │                      │
                            ▼                      ▼
                     ┌──────────────┐     ┌──────────────┐
                     │  构建 SSH    │────→│  唤起终端    │
                     │  命令字符串   │     │  执行连接    │
                     └──────────────┘     └──────────────┘
```

---

## Architecture Overview (English)

### Module Structure
- **core**: Platform-independent library (encryption, config, SSH, search)
- **lite-egui**: Windows native UI using egui
- **lite-gtk**: Linux native UI using GTK4
- **lite-swift**: macOS native UI using SwiftUI with Rust FFI

### Core Components

1. **Crypto Module**: Argon2id + AES-256-GCM encryption
2. **Config Module**: SQLite storage with encrypted JSON blobs
3. **Keychain Module**: Cross-platform secure credential storage
4. **Search Module**: Fuzzy matching with Skim algorithm
5. **SSH Module**: ssh2/russh for SSH operations

### Security Architecture
- Master password → Argon2id → AES-256-GCM key
- All credentials stored in OS keychain
- Memory protection with SecureString
- Config file encrypted at rest

### Platform Integration
- **Windows**: egui + Windows Terminal/PowerShell
- **Linux**: GTK4 + GNOME Terminal/Konsole/Alacritty
- **macOS**: SwiftUI + iTerm2/Terminal.app/Alacritty

---

## 技术栈 / Tech Stack

| 组件 | 技术 | 版本 |
|------|------|------|
| 核心语言 | Rust | 1.75+ |
| Windows UI | egui | 0.24+ |
| Linux UI | GTK4 + libadwaita | 4.0+ |
| macOS UI | SwiftUI | macOS 12+ |
| 加密 | argon2 + aes-gcm | latest |
| 数据库 | rusqlite | 0.30+ |
| SSH | ssh2 / openssh | - |
| 钥匙串 | keyring | 2.0+ |

---

**文档版本**: v0.3.0
**最后更新**: 2026-04-02
