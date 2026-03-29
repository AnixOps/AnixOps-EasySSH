use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use rand::{rngs::OsRng, RngCore};
use std::sync::Mutex;

use crate::error::LiteError;

/// 加密状态
pub struct CryptoState {
    cipher: Option<Aes256Gcm>,
    salt: Option<[u8; 32]>,
}

impl CryptoState {
    pub fn new() -> Self {
        Self {
            cipher: None,
            salt: None,
        }
    }

    /// 初始化加密状态（首次设置主密码）
    pub fn initialize(&mut self, master_password: &str) -> Result<(), LiteError> {
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);

        let key = self.derive_key_internal(master_password, &salt)?;
        let cipher =
            Aes256Gcm::new_from_slice(&key).map_err(|e| LiteError::Crypto(e.to_string()))?;

        self.cipher = Some(cipher);
        self.salt = Some(salt);
        Ok(())
    }

    /// 使用已有盐值解锁（验证主密码）
    pub fn unlock(&mut self, master_password: &str) -> Result<bool, LiteError> {
        let salt = self.salt.ok_or(LiteError::InvalidMasterPassword)?;

        let key = self.derive_key_internal(master_password, &salt)?;
        let cipher =
            Aes256Gcm::new_from_slice(&key).map_err(|e| LiteError::Crypto(e.to_string()))?;

        self.cipher = Some(cipher);
        Ok(true)
    }

    fn derive_key_internal(
        &self,
        master_password: &str,
        salt: &[u8; 32],
    ) -> Result<[u8; 32], LiteError> {
        let salt_str =
            SaltString::encode_b64(salt).map_err(|e| LiteError::Crypto(e.to_string()))?;

        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(master_password.as_bytes(), &salt_str)
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        let output = hash.hash.ok_or(LiteError::InvalidMasterPassword)?;
        let key_bytes = output.as_bytes();

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes[..32]);
        Ok(key)
    }

    /// 加密数据
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, LiteError> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or(LiteError::InvalidMasterPassword)?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    /// 解密数据
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, LiteError> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or(LiteError::InvalidMasterPassword)?;

        if data.len() < 12 {
            return Err(LiteError::Crypto("数据太短".to_string()));
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| LiteError::Crypto(e.to_string()))
    }

    /// 获取盐值
    pub fn get_salt(&self) -> Option<Vec<u8>> {
        self.salt.map(|s| s.to_vec())
    }

    /// 设置盐值
    pub fn set_salt(&mut self, salt: [u8; 32]) {
        self.salt = Some(salt);
    }

    /// 锁定（清除密钥）
    pub fn lock(&mut self) {
        self.cipher = None;
    }

    /// 检查是否已解锁
    pub fn is_unlocked(&self) -> bool {
        self.cipher.is_some()
    }
}

impl Default for CryptoState {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局加密状态
pub static CRYPTO_STATE: std::sync::LazyLock<Mutex<CryptoState>> =
    std::sync::LazyLock::new(|| Mutex::new(CryptoState::new()));
