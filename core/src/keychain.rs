use keyring::Entry;

use crate::error::LiteError;

/// 服务名称用于Keychain
const SERVICE_NAME: &str = "com.easyssh.lite";

/// 在Keychain中存储密码
pub fn store_password(server_id: &str, password: &str) -> Result<(), LiteError> {
    let entry =
        Entry::new(SERVICE_NAME, server_id).map_err(|e| LiteError::Keychain(e.to_string()))?;

    entry
        .set_password(password)
        .map_err(|e| LiteError::Keychain(e.to_string()))?;

    Ok(())
}

/// 从Keychain获取密码
pub fn get_password(server_id: &str) -> Result<Option<String>, LiteError> {
    let entry =
        Entry::new(SERVICE_NAME, server_id).map_err(|e| LiteError::Keychain(e.to_string()))?;

    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(LiteError::Keychain(e.to_string())),
    }
}

/// 从Keychain删除密码
pub fn delete_password(server_id: &str) -> Result<(), LiteError> {
    let entry =
        Entry::new(SERVICE_NAME, server_id).map_err(|e| LiteError::Keychain(e.to_string()))?;

    entry
        .delete_credential()
        .map_err(|e| LiteError::Keychain(e.to_string()))?;

    Ok(())
}

/// 存储主密码到Keychain（用于快速解锁）
pub fn store_master_password_hash(hash: &str) -> Result<(), LiteError> {
    let entry = Entry::new(SERVICE_NAME, "master_password")
        .map_err(|e| LiteError::Keychain(e.to_string()))?;

    entry
        .set_password(hash)
        .map_err(|e| LiteError::Keychain(e.to_string()))?;

    Ok(())
}

/// 获取主密码哈希
pub fn get_master_password_hash() -> Result<Option<String>, LiteError> {
    let entry = Entry::new(SERVICE_NAME, "master_password")
        .map_err(|e| LiteError::Keychain(e.to_string()))?;

    match entry.get_password() {
        Ok(hash) => Ok(Some(hash)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(LiteError::Keychain(e.to_string())),
    }
}
