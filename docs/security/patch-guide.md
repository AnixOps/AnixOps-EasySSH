# EasySSH 安全加固快速修复指南

**版本**: v0.3.0
**日期**: 2026-04-01
**优先级**: P0（立即修复）

---

## 立即行动项（24小时内）

### 1. 更新依赖修复CVE

```bash
# 更新Cargo.toml中的依赖版本
cargo update -p sqlx --precise 0.8.2
cargo update -p rustls --precise 0.23.5
cargo update -p url --precise 2.5.4
cargo update -p ring --precise 0.17.14

# 验证修复
cargo audit
```

### 2. 应用安全补丁

#### 补丁1: 修复全局加密状态锁（crypto.rs）

```rust
// core/src/crypto.rs
// 修改第139行

// 旧代码
pub static CRYPTO_STATE: std::sync::LazyLock<Mutex<CryptoState>> =
    std::sync::LazyLock::new(|| Mutex::new(CryptoState::new()));

// 新代码（异步安全）
pub static CRYPTO_STATE: std::sync::LazyLock<tokio::sync::RwLock<CryptoState>> =
    std::sync::LazyLock::new(|| tokio::sync::RwLock::new(CryptoState::new()));
```

#### 补丁2: 增强Argon2参数（crypto.rs）

```rust
// core/src/crypto.rs
// 在derive_key_internal函数中

fn derive_key_internal(
    &self,
    master_password: &str,
    salt: &[u8; 32],
) -> Result<[u8; 32], LiteError> {
    let salt_str =
        SaltString::encode_b64(salt).map_err(|e| LiteError::Crypto(e.to_string()))?;

    // 使用更强的参数
    let params = argon2::Params::new(
        64 * 1024,  // 64MB内存
        3,          // 3次迭代
        4,          // 4个并行线程
        Some(32)    // 32字节输出
    ).map_err(|e| LiteError::Crypto(e.to_string()))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let hash = argon2
        .hash_password(master_password.as_bytes(), &salt_str)
        .map_err(|e| LiteError::Crypto(e.to_string()))?;

    let output = hash.hash.ok_or(LiteError::InvalidMasterPassword)?;
    let key_bytes = output.as_bytes();

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes[..32]);
    Ok(key)
}
```

#### 补丁3: 增强bcrypt成本（auth.rs）

```rust
// pro-server/src/auth.rs
// 修改第97行

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    // 成本从默认10提高到12
    let hashed = hash(password, 12)?;
    Ok(hashed)
}
```

### 3. 配置安全加固

#### Cargo.toml添加安全依赖

```toml
# core/Cargo.toml
[dependencies]
# 添加内存清零支持
zeroize = { version = "1.8", features = ["derive"] }

# 更新到安全版本
sqlx = "0.8.2"
ring = "0.17.14"
```

```toml
# pro-server/Cargo.toml
[dependencies]
# SAML安全处理
samael = { version = "0.21", optional = true }

# 更新到安全版本
rustls = "0.23.5"
```

#### 添加安全编译配置

```toml
# Cargo.toml（根）
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
overflow-checks = true  # 添加溢出检查
```

---

## 短期修复（1周内）

### SSO安全加固

#### 添加SAML签名验证

```rust
// core/src/sso.rs
// 在process_saml_response中添加

use samael::service_provider::ServiceProvider;

pub fn process_saml_response(
    &mut self,
    response: &SamlAuthResponse,
) -> Result<(SsoUserInfo, SsoSession), LiteError> {
    let provider = self.providers.get(&response.provider_id)
        .ok_or_else(|| LiteError::Sso("Provider not found".to_string()))?;

    let SsoProviderConfig::Saml(config) = &provider.config else {
        return Err(LiteError::Sso("Invalid provider type".to_string()));
    };

    // 1. Base64解码SAML响应
    let saml_xml = base64::decode(&response.saml_response)?;

    // 2. 使用samael解析和验证
    let service_provider = ServiceProvider::new()
        .entity_id(&config.sp_entity_id)
        .acs_url(&config.sp_acs_url);

    let assertion = service_provider.parse_response(&saml_xml)
        .map_err(|e| LiteError::Sso(format!("SAML validation failed: {}", e)))?;

    // 3. 验证签名
    if config.require_signed_assertions {
        assertion.verify_signature(&config.idp_certificate)
            .map_err(|e| LiteError::Sso("Invalid SAML signature".to_string()))?;
    }

    // 4. 验证条件
    assertion.validate_conditions(
        &config.sp_entity_id,
        Utc::now(),
        chrono::Duration::minutes(5)
    )?;

    // 5. 提取用户信息
    let user_info = extract_user_info_from_assertion(&assertion, &config.attribute_mapping)?;

    let session = SsoSession::new(&user_info.user_id, &response.provider_id, 8);
    Ok((user_info, session))
}
```

#### 加密SSO令牌存储

```rust
// core/src/sso.rs
// 添加加密存储支持

use crate::crypto::CRYPTO_STATE;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSsoSession {
    pub id: String,
    pub user_id: String,
    pub provider_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub encrypted_sso_token: String,  // Base64编码的加密数据
    pub encrypted_id_token: Option<String>,
    pub nonce: String,  // Base64编码
}

impl EncryptedSsoSession {
    pub async fn encrypt(session: SsoSession) -> Result<Self, LiteError> {
        let crypto = CRYPTO_STATE.lock()
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        let token_encrypted = crypto.encrypt(session.sso_token.as_bytes())?;
        let id_token_encrypted = session.id_token
            .map(|t| crypto.encrypt(t.as_bytes()))
            .transpose()?;

        Ok(Self {
            id: session.id,
            user_id: session.user_id,
            provider_id: session.provider_id,
            created_at: session.created_at,
            expires_at: session.expires_at,
            encrypted_sso_token: base64::encode(&token_encrypted),
            encrypted_id_token: id_token_encrypted.map(base64::encode),
            nonce: base64::encode(&token_encrypted[..12]),
        })
    }

    pub async fn decrypt(&self) -> Result<SsoSession, LiteError> {
        let crypto = CRYPTO_STATE.lock()
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        let token_bytes = base64::decode(&self.encrypted_sso_token)?;
        let token = String::from_utf8(crypto.decrypt(&token_bytes)?)?;

        let id_token = match &self.encrypted_id_token {
            Some(enc) => {
                let bytes = base64::decode(enc)?;
                Some(String::from_utf8(crypto.decrypt(&bytes)?)?)
            }
            None => None,
        };

        Ok(SsoSession {
            id: self.id.clone(),
            user_id: self.user_id.clone(),
            provider_id: self.provider_id.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
            last_used_at: self.created_at,
            sso_token: token,
            id_token,
        })
    }
}
```

---

## 验证清单

### 修复后验证

- [ ] `cargo audit`显示0高危漏洞
- [ ] 单元测试通过
- [ ] 集成测试通过
- [ ] 手动测试密码解锁
- [ ] 手动测试SSO登录
- [ ] 审计日志完整性验证

### 安全测试

```bash
# 运行安全相关测试
cargo test --package easyssh-core crypto
cargo test --package easyssh-core audit
cargo test --package easyssh-core sso

# 审计依赖
cargo audit

# 检查许可证
cargo deny check licenses
```

---

## 紧急联系

如果发现安全问题：
1. 立即创建私有Issue
2. 发送邮件至 security@easyssh.io
3. 不要公开披露直到修复

**安全响应时间**: P0漏洞24小时内响应
