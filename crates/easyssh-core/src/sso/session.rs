//! SSO会话管理模块
//!
//! 提供加密令牌存储、会话验证和生命周期管理

use crate::error::LiteError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// SSO会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoSession {
    /// 会话ID
    pub id: String,
    /// 用户ID
    pub user_id: String,
    /// 提供商ID
    pub provider_id: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 最后使用时间
    pub last_used_at: DateTime<Utc>,
    /// 加密存储的SSO令牌 (安全性增强)
    #[serde(skip_serializing, skip_deserializing)]
    pub encrypted_sso_token: Option<EncryptedSsoToken>,
    /// ID令牌 (JWT格式, 加密存储)
    #[serde(skip_serializing, skip_deserializing)]
    pub encrypted_id_token: Option<EncryptedSsoToken>,
    /// 访问令牌 (加密存储)
    #[serde(skip_serializing, skip_deserializing)]
    pub encrypted_access_token: Option<EncryptedSsoToken>,
    /// 刷新令牌 (加密存储)
    #[serde(skip_serializing, skip_deserializing)]
    pub encrypted_refresh_token: Option<EncryptedSsoToken>,
    /// IP地址 (用于会话绑定)
    pub ip_address: Option<String>,
    /// 用户代理
    pub user_agent: Option<String>,
    /// 会话状态
    pub status: SessionStatus,
    /// 扩展属性
    pub metadata: HashMap<String, String>,
}

/// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// 活跃
    Active,
    /// 已过期
    Expired,
    /// 已撤销
    Revoked,
    /// 已暂停 (临时禁用)
    Suspended,
}

/// 加密存储的SSO令牌
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSsoToken {
    /// 密文
    pub ciphertext: String,
    /// Nonce (用于GCM)
    pub nonce: String,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
}

impl Zeroize for EncryptedSsoToken {
    fn zeroize(&mut self) {
        self.ciphertext.zeroize();
        self.nonce.zeroize();
    }
}

impl Drop for EncryptedSsoToken {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl SsoSession {
    /// 创建新会话 (使用加密令牌)
    pub fn new(user_id: &str, provider_id: &str, duration_hours: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            provider_id: provider_id.to_string(),
            created_at: now,
            expires_at: now + Duration::hours(duration_hours),
            last_used_at: now,
            encrypted_sso_token: None,
            encrypted_id_token: None,
            encrypted_access_token: None,
            encrypted_refresh_token: None,
            ip_address: None,
            user_agent: None,
            status: SessionStatus::Active,
            metadata: HashMap::new(),
        }
    }

    /// 使用原始令牌创建新会话
    pub fn new_with_tokens(
        user_id: &str,
        provider_id: &str,
        sso_token: &str,
        id_token: Option<&str>,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        duration_hours: i64,
    ) -> Result<Self, LiteError> {
        use crate::crypto::CRYPTO_STATE;
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

        let mut session = Self::new(user_id, provider_id, duration_hours);

        let crypto = CRYPTO_STATE
            .read()
            .map_err(|e| LiteError::Crypto(format!("Failed to access crypto state: {}", e)))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::Crypto("Crypto state not unlocked".to_string()));
        }

        // 加密SSO令牌
        let encrypted = crypto
            .encrypt(sso_token.as_bytes())
            .map_err(|e| LiteError::Crypto(format!("Failed to encrypt SSO token: {}", e)))?;
        session.encrypted_sso_token = Some(EncryptedSsoToken {
            ciphertext: BASE64.encode(&encrypted[12..]),
            nonce: BASE64.encode(&encrypted[..12]),
            expires_at: session.expires_at,
        });

        // 加密ID令牌
        if let Some(token) = id_token {
            let encrypted = crypto
                .encrypt(token.as_bytes())
                .map_err(|e| LiteError::Crypto(format!("Failed to encrypt ID token: {}", e)))?;
            session.encrypted_id_token = Some(EncryptedSsoToken {
                ciphertext: BASE64.encode(&encrypted[12..]),
                nonce: BASE64.encode(&encrypted[..12]),
                expires_at: session.expires_at,
            });
        }

        // 加密访问令牌
        if let Some(token) = access_token {
            let encrypted = crypto
                .encrypt(token.as_bytes())
                .map_err(|e| LiteError::Crypto(format!("Failed to encrypt access token: {}", e)))?;
            session.encrypted_access_token = Some(EncryptedSsoToken {
                ciphertext: BASE64.encode(&encrypted[12..]),
                nonce: BASE64.encode(&encrypted[..12]),
                expires_at: session.expires_at,
            });
        }

        // 加密刷新令牌
        if let Some(token) = refresh_token {
            let encrypted = crypto.encrypt(token.as_bytes()).map_err(|e| {
                LiteError::Crypto(format!("Failed to encrypt refresh token: {}", e))
            })?;
            session.encrypted_refresh_token = Some(EncryptedSsoToken {
                ciphertext: BASE64.encode(&encrypted[12..]),
                nonce: BASE64.encode(&encrypted[..12]),
                expires_at: session.expires_at,
            });
        }

        Ok(session)
    }

    /// 解密获取SSO令牌
    pub fn get_sso_token(&self) -> Result<Option<String>, LiteError> {
        self.decrypt_token(&self.encrypted_sso_token)
    }

    /// 解密获取ID令牌
    pub fn get_id_token(&self) -> Result<Option<String>, LiteError> {
        self.decrypt_token(&self.encrypted_id_token)
    }

    /// 解密获取访问令牌
    pub fn get_access_token(&self) -> Result<Option<String>, LiteError> {
        self.decrypt_token(&self.encrypted_access_token)
    }

    /// 解密获取刷新令牌
    pub fn get_refresh_token(&self) -> Result<Option<String>, LiteError> {
        self.decrypt_token(&self.encrypted_refresh_token)
    }

    fn decrypt_token(
        &self,
        encrypted: &Option<EncryptedSsoToken>,
    ) -> Result<Option<String>, LiteError> {
        use crate::crypto::CRYPTO_STATE;
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

        let token = match encrypted {
            Some(t) => t,
            None => return Ok(None),
        };

        let crypto = CRYPTO_STATE
            .read()
            .map_err(|e| LiteError::Crypto(format!("Failed to access crypto state: {}", e)))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        // 重建加密blob
        let mut encrypted_blob = Vec::new();
        encrypted_blob.extend_from_slice(
            &BASE64
                .decode(&token.nonce)
                .map_err(|_| LiteError::Crypto("Invalid token nonce".to_string()))?,
        );
        encrypted_blob.extend_from_slice(
            &BASE64
                .decode(&token.ciphertext)
                .map_err(|_| LiteError::Crypto("Invalid token ciphertext".to_string()))?,
        );

        let decrypted = crypto
            .decrypt(&encrypted_blob)
            .map_err(|_| LiteError::InvalidMasterPassword)?;

        String::from_utf8(decrypted)
            .map_err(|_| LiteError::Crypto("Invalid UTF-8 in token".to_string()))
            .map(Some)
    }

    /// 检查会话是否过期
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at || self.status == SessionStatus::Expired
    }

    /// 检查会话是否活跃
    pub fn is_active(&self) -> bool {
        !self.is_expired() && self.status == SessionStatus::Active
    }

    /// 刷新最后使用时间
    pub fn touch(&mut self) {
        self.last_used_at = Utc::now();
    }

    /// 延长会话
    pub fn extend(&mut self, duration_hours: i64) {
        self.expires_at = Utc::now() + Duration::hours(duration_hours);
    }

    /// 撤销会话
    pub fn revoke(&mut self) {
        self.status = SessionStatus::Revoked;
        // 清零所有令牌
        if let Some(ref mut token) = self.encrypted_sso_token {
            token.zeroize();
        }
        if let Some(ref mut token) = self.encrypted_id_token {
            token.zeroize();
        }
        if let Some(ref mut token) = self.encrypted_access_token {
            token.zeroize();
        }
        if let Some(ref mut token) = self.encrypted_refresh_token {
            token.zeroize();
        }
        self.encrypted_sso_token = None;
        self.encrypted_id_token = None;
        self.encrypted_access_token = None;
        self.encrypted_refresh_token = None;
    }

    /// 暂停会话
    pub fn suspend(&mut self) {
        self.status = SessionStatus::Suspended;
    }

    /// 恢复会话
    pub fn resume(&mut self) {
        if self.status == SessionStatus::Suspended && !self.is_expired() {
            self.status = SessionStatus::Active;
        }
    }

    /// 设置IP地址
    pub fn set_ip_address(&mut self, ip: &str) {
        self.ip_address = Some(ip.to_string());
    }

    /// 设置用户代理
    pub fn set_user_agent(&mut self, ua: &str) {
        self.user_agent = Some(ua.to_string());
    }

    /// 添加元数据
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// 获取剩余时间 (秒)
    pub fn remaining_seconds(&self) -> i64 {
        let now = Utc::now();
        if self.expires_at > now {
            (self.expires_at - now).num_seconds()
        } else {
            0
        }
    }

    /// 检查IP是否匹配 (用于会话绑定)
    pub fn check_ip_binding(&self, ip: &str, allow_subnet: bool) -> bool {
        match &self.ip_address {
            Some(session_ip) => {
                if allow_subnet {
                    // 简化实现：检查前3个八位字节 (粗略的子网匹配)
                    let session_parts: Vec<&str> = session_ip.split('.').collect();
                    let check_parts: Vec<&str> = ip.split('.').collect();
                    session_parts[..3.min(session_parts.len())]
                        == check_parts[..3.min(check_parts.len())]
                } else {
                    session_ip == ip
                }
            }
            None => true, // 如果没有存储IP，允许任何IP
        }
    }
}

/// 会话管理器
pub struct SsoSessionManager {
    /// 活跃会话
    sessions: HashMap<String, SsoSession>,
    /// 用户索引 (user_id -> session_ids)
    user_index: HashMap<String, Vec<String>>,
    /// 提供商索引 (provider_id -> session_ids)
    provider_index: HashMap<String, Vec<String>>,
    /// 最大会话数 (每用户)
    max_sessions_per_user: usize,
    /// 默认会话时长 (小时)
    default_session_duration_hours: i64,
    /// 启用IP绑定
    enable_ip_binding: bool,
}

impl SsoSessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            user_index: HashMap::new(),
            provider_index: HashMap::new(),
            max_sessions_per_user: 5,
            default_session_duration_hours: 8,
            enable_ip_binding: false,
        }
    }

    /// 配置会话管理器
    pub fn with_config(
        mut self,
        max_sessions: usize,
        duration_hours: i64,
        ip_binding: bool,
    ) -> Self {
        self.max_sessions_per_user = max_sessions;
        self.default_session_duration_hours = duration_hours;
        self.enable_ip_binding = ip_binding;
        self
    }

    /// 创建新会话
    pub fn create_session(
        &mut self,
        user_id: &str,
        provider_id: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> SsoSession {
        // 检查并限制用户会话数
        self.enforce_session_limit(user_id);

        let mut session = SsoSession::new(user_id, provider_id, self.default_session_duration_hours);

        if let Some(ip) = ip_address {
            session.set_ip_address(ip);
        }

        if let Some(ua) = user_agent {
            session.set_user_agent(ua);
        }

        let session_id = session.id.clone();

        // 添加到索引
        self.user_index
            .entry(user_id.to_string())
            .or_default()
            .push(session_id.clone());

        self.provider_index
            .entry(provider_id.to_string())
            .or_default()
            .push(session_id.clone());

        // 存储会话
        self.sessions.insert(session_id, session.clone());

        session
    }

    /// 使用令牌创建会话
    pub fn create_session_with_tokens(
        &mut self,
        user_id: &str,
        provider_id: &str,
        sso_token: &str,
        id_token: Option<&str>,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<SsoSession, LiteError> {
        self.enforce_session_limit(user_id);

        let mut session = SsoSession::new_with_tokens(
            user_id,
            provider_id,
            sso_token,
            id_token,
            access_token,
            refresh_token,
            self.default_session_duration_hours,
        )?;

        if let Some(ip) = ip_address {
            session.set_ip_address(ip);
        }

        if let Some(ua) = user_agent {
            session.set_user_agent(ua);
        }

        let session_id = session.id.clone();

        self.user_index
            .entry(user_id.to_string())
            .or_default()
            .push(session_id.clone());

        self.provider_index
            .entry(provider_id.to_string())
            .or_default()
            .push(session_id.clone());

        self.sessions.insert(session_id, session.clone());

        Ok(session)
    }

    /// 获取会话
    pub fn get_session(&self, session_id: &str) -> Option<&SsoSession> {
        self.sessions.get(session_id).filter(|s| s.is_active())
    }

    /// 获取会话(可变)
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut SsoSession> {
        self.sessions.get_mut(session_id).filter(|s| s.is_active())
    }

    /// 验证并刷新会话
    pub fn validate_and_touch(&mut self, session_id: &str, ip: Option<&str>) -> Option<&SsoSession> {
        let session = self.sessions.get_mut(session_id)?;

        // 检查状态
        if !session.is_active() {
            return None;
        }

        // 检查IP绑定
        if self.enable_ip_binding {
            if let Some(check_ip) = ip {
                if !session.check_ip_binding(check_ip, true) {
                    // IP不匹配，撤销会话
                    session.revoke();
                    return None;
                }
            }
        }

        // 刷新最后使用时间
        session.touch();

        // 返回不可变引用
        self.sessions.get(session_id)
    }

    /// 终止会话
    pub fn terminate_session(&mut self, session_id: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.revoke();

            // 从索引中移除
            if let Some(sessions) = self.user_index.get_mut(&session.user_id) {
                sessions.retain(|id| id != session_id);
            }

            if let Some(sessions) = self.provider_index.get_mut(&session.provider_id) {
                sessions.retain(|id| id != session_id);
            }

            true
        } else {
            false
        }
    }

    /// 终止用户的所有会话
    pub fn terminate_user_sessions(&mut self, user_id: &str) -> usize {
        let session_ids: Vec<String> = self
            .user_index
            .get(user_id)
            .cloned()
            .unwrap_or_default();

        let mut count = 0;
        for session_id in &session_ids {
            if self.terminate_session(session_id) {
                count += 1;
            }
        }

        // 清理用户索引
        self.user_index.remove(user_id);

        count
    }

    /// 终止提供商的所有会话
    pub fn terminate_provider_sessions(&mut self, provider_id: &str) -> usize {
        let session_ids: Vec<String> = self
            .provider_index
            .get(provider_id)
            .cloned()
            .unwrap_or_default();

        let mut count = 0;
        for session_id in &session_ids {
            if self.terminate_session(session_id) {
                count += 1;
            }
        }

        // 清理提供商索引
        self.provider_index.remove(provider_id);

        count
    }

    /// 列出用户的活跃会话
    pub fn list_user_sessions(&self, user_id: &str) -> Vec<&SsoSession> {
        self.user_index
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.sessions.get(id))
                    .filter(|s| s.is_active())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 清理过期会话
    pub fn cleanup_expired_sessions(&mut self) -> usize {
        let expired: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.is_expired() || s.status == SessionStatus::Revoked)
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired.len();
        for id in expired {
            self.terminate_session(&id);
        }

        count
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> SessionStatistics {
        let total = self.sessions.len();
        let active = self
            .sessions
            .values()
            .filter(|s| s.is_active())
            .count();
        let expired = self
            .sessions
            .values()
            .filter(|s| s.is_expired())
            .count();
        let revoked = self
            .sessions
            .values()
            .filter(|s| s.status == SessionStatus::Revoked)
            .count();

        SessionStatistics {
            total,
            active,
            expired,
            revoked,
            unique_users: self.user_index.len(),
        }
    }

    /// 强制执行会话数限制
    fn enforce_session_limit(&mut self, user_id: &str) {
        if let Some(session_ids) = self.user_index.get(user_id) {
            if session_ids.len() >= self.max_sessions_per_user {
                // 终止最旧的会话
                let oldest_id = session_ids[0].clone();
                self.terminate_session(&oldest_id);
            }
        }
    }
}

impl Default for SsoSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 会话统计信息
#[derive(Debug, Clone)]
pub struct SessionStatistics {
    pub total: usize,
    pub active: usize,
    pub expired: usize,
    pub revoked: usize,
    pub unique_users: usize,
}

/// 会话事件 (用于审计)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub event_id: String,
    pub session_id: String,
    pub user_id: String,
    pub event_type: SessionEventType,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub details: HashMap<String, String>,
}

/// 会话事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventType {
    Created,
    Refreshed,
    Extended,
    Suspended,
    Resumed,
    Revoked,
    Expired,
    AccessDenied,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session() -> SsoSession {
        SsoSession::new("user123", "provider456", 8)
    }

    #[test]
    fn test_sso_session_creation() {
        let session = create_test_session();

        assert!(!session.is_expired());
        assert_eq!(session.user_id, "user123");
        assert_eq!(session.provider_id, "provider456");
        assert!(session.is_active());
    }

    #[test]
    fn test_session_expiration() {
        let mut session = create_test_session();

        // 测试过期
        session.expires_at = Utc::now() - Duration::hours(1);
        assert!(session.is_expired());
        assert!(!session.is_active());
    }

    #[test]
    fn test_session_revocation() {
        let mut session = create_test_session();

        // 初始化加密令牌
        session.encrypted_sso_token = Some(EncryptedSsoToken {
            ciphertext: "test".to_string(),
            nonce: "test".to_string(),
            expires_at: Utc::now(),
        });

        session.revoke();

        assert_eq!(session.status, SessionStatus::Revoked);
        assert!(!session.is_active());
        assert!(session.encrypted_sso_token.is_none());
    }

    #[test]
    fn test_session_touch() {
        let mut session = create_test_session();
        let old_last_used = session.last_used_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();

        assert!(session.last_used_at > old_last_used);
    }

    #[test]
    fn test_session_extend() {
        let mut session = create_test_session();
        let old_expires = session.expires_at;

        session.extend(24);

        assert!(session.expires_at > old_expires);
        assert!(session.remaining_seconds() > 0);
    }

    #[test]
    fn test_session_manager_creation() {
        let manager = SsoSessionManager::new();
        let stats = manager.get_statistics();

        assert_eq!(stats.total, 0);
        assert_eq!(stats.active, 0);
    }

    #[test]
    fn test_session_manager_create() {
        let mut manager = SsoSessionManager::new();

        let session = manager.create_session("user1", "provider1", Some("192.168.1.1"), Some("Mozilla/5.0"));

        assert!(!session.is_expired());
        assert_eq!(session.ip_address, Some("192.168.1.1".to_string()));

        let stats = manager.get_statistics();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.unique_users, 1);
    }

    #[test]
    fn test_session_termination() {
        let mut manager = SsoSessionManager::new();

        let session = manager.create_session("user1", "provider1", None, None);
        let session_id = session.id.clone();

        assert!(manager.terminate_session(&session_id));
        assert!(!manager.terminate_session(&session_id)); // 已终止

        let stats = manager.get_statistics();
        assert_eq!(stats.revoked, 1);
    }

    #[test]
    fn test_user_session_termination() {
        let mut manager = SsoSessionManager::new();

        manager.create_session("user1", "provider1", None, None);
        manager.create_session("user1", "provider1", None, None);
        manager.create_session("user2", "provider1", None, None);

        let terminated = manager.terminate_user_sessions("user1");
        assert_eq!(terminated, 2);

        let user1_sessions = manager.list_user_sessions("user1");
        assert_eq!(user1_sessions.len(), 0);

        let user2_sessions = manager.list_user_sessions("user2");
        assert_eq!(user2_sessions.len(), 1);
    }

    #[test]
    fn test_session_limit() {
        let mut manager = SsoSessionManager::new().with_config(2, 8, false);

        let session1 = manager.create_session("user1", "provider1", None, None);
        let session2 = manager.create_session("user1", "provider1", None, None);
        let session3 = manager.create_session("user1", "provider1", None, None);

        let user1_sessions = manager.list_user_sessions("user1");
        assert_eq!(user1_sessions.len(), 2);

        // 第一个会话应已被终止
        assert!(!manager.get_session(&session1.id).is_some() || manager.get_session(&session1.id).unwrap().status == SessionStatus::Revoked);
    }

    #[test]
    fn test_ip_binding() {
        let mut manager = SsoSessionManager::new().with_config(5, 8, true);

        let session = manager.create_session("user1", "provider1", Some("192.168.1.1"), None);
        let session_id = session.id.clone();

        // 相同IP应通过
        let validated = manager.validate_and_touch(&session_id, Some("192.168.1.1"));
        assert!(validated.is_some());

        // 不同IP应失败并撤销会话
        let validated = manager.validate_and_touch(&session_id, Some("192.168.2.2"));
        assert!(validated.is_none());

        // 会话应被撤销
        let session = manager.sessions.get(&session_id).unwrap();
        assert_eq!(session.status, SessionStatus::Revoked);
    }

    #[test]
    fn test_cleanup_expired() {
        let mut manager = SsoSessionManager::new();

        let mut session1 = manager.create_session("user1", "provider1", None, None);
        session1.expires_at = Utc::now() - Duration::hours(1); // 已过期
        let id1 = session1.id.clone();
        manager.sessions.insert(id1.clone(), session1);

        manager.create_session("user2", "provider1", None, None);

        let cleaned = manager.cleanup_expired_sessions();
        assert_eq!(cleaned, 1);

        let stats = manager.get_statistics();
        assert_eq!(stats.total, 2); // 会话仍存在但被撤销
        assert_eq!(stats.revoked, 1);
    }

    #[test]
    fn test_session_suspend_resume() {
        let mut session = create_test_session();

        session.suspend();
        assert_eq!(session.status, SessionStatus::Suspended);
        assert!(!session.is_active());

        session.resume();
        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.is_active());
    }

    #[test]
    fn test_session_metadata() {
        let mut session = create_test_session();

        session.add_metadata("login_method", "sso");
        session.add_metadata("provider", "okta");

        assert_eq!(session.metadata.get("login_method"), Some(&"sso".to_string()));
        assert_eq!(session.metadata.get("provider"), Some(&"okta".to_string()));
    }
}
