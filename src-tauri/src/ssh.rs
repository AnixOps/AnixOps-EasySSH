use crate::error::LiteError;
use ssh2::Session;
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// SSH会话信息
#[derive(Clone)]
pub struct SshSessionInfo {
    pub id: String,
    pub server_id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
}

/// 连接池项 - 复用连接
struct MuxPoolItem {
    session: Arc<TokioMutex<Session>>,
    info: SshSessionInfo,
    last_used: std::time::Instant,
    ref_count: usize,
}

/// SSH会话管理器 - 支持连接复用(MUX)
pub struct SshSessionManager {
    // 用户会话 -> SSH连接
    sessions: HashMap<String, Arc<TokioMutex<Session>>>,
    // 服务器索引 -> 连接池 (实现MUX)
    mux_pools: HashMap<String, MuxPoolItem>,
    // MUX连接最大复用次数
    max_mux_uses: usize,
    // MUX连接最大空闲时间(秒)
    mux_idle_timeout: u64,
}

impl SshSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            mux_pools: HashMap::new(),
            max_mux_uses: 100,      // 每个连接最多复用100次
            mux_idle_timeout: 300,  // 5分钟空闲后关闭
        }
    }

    /// 生成MUX key
    fn mux_key(host: &str, port: u16, username: &str) -> String {
        format!("{}@{}:{}", username, host, port)
    }

    /// 清理过期MUX连接 (懒清理)
    fn cleanup_expired_mux(&mut self) {
        let now = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(self.mux_idle_timeout);

        // 只移除过期的条目，不在这里断开连接
        // 连接会在Arc被drop时自动断开
        self.mux_pools.retain(|_, pool| {
            if pool.ref_count == 0 && now.duration_since(pool.last_used) > timeout {
                log::info!("SSH MUX: 清理过期连接池 (idle {}s)", pool.last_used.elapsed().as_secs());
                false
            } else {
                true
            }
        });
    }

    /// 连接到SSH服务器
    pub async fn connect(
        &mut self,
        session_id: &str,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
    ) -> Result<(), LiteError> {
        self.connect_with_mux(session_id, host, port, username, password, false).await
    }

    /// 连接到SSH服务器 (支持MUX)
    pub async fn connect_with_mux(
        &mut self,
        session_id: &str,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
        use_mux: bool,
    ) -> Result<(), LiteError> {
        // 清理过期连接
        self.cleanup_expired_mux();

        let mux_key = Self::mux_key(host, port, username);

        // 尝试复用MUX连接
        if use_mux {
            if let Some(pool_item) = self.mux_pools.get_mut(&mux_key) {
                if pool_item.ref_count < self.max_mux_uses {
                    // 复用现有连接
                    pool_item.ref_count += 1;
                    pool_item.last_used = std::time::Instant::now();

                    self.sessions.insert(
                        session_id.to_string(),
                        pool_item.session.clone(),
                    );
                    log::info!("SSH MUX: 复用连接 {} (ref={})", mux_key, pool_item.ref_count);
                    return Ok(());
                }
            }
        }

        // 创建新连接
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| LiteError::Ssh(e.to_string()))?;

        let mut session = Session::new().map_err(|e| LiteError::Ssh(e.to_string()))?;

        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| LiteError::Ssh(e.to_string()))?;

        // 认证
        match password {
            Some(pwd) => {
                session
                    .userauth_password(username, pwd)
                    .map_err(|e| LiteError::Ssh(e.to_string()))?;
            }
            None => {
                session
                    .userauth_agent(username)
                    .map_err(|e| LiteError::Ssh(e.to_string()))?;
            }
        }

        if !session.authenticated() {
            return Err(LiteError::Ssh("认证失败".to_string()));
        }

        let session = Arc::new(TokioMutex::new(session));

        // 如果启用MUX，添加到连接池
        if use_mux {
            self.mux_pools.insert(
                mux_key.clone(),
                MuxPoolItem {
                    session: session.clone(),
                    info: SshSessionInfo {
                        id: session_id.to_string(),
                        server_id: String::new(),
                        host: host.to_string(),
                        port,
                        username: username.to_string(),
                    },
                    last_used: std::time::Instant::now(),
                    ref_count: 1,
                },
            );
            log::info!("SSH MUX: 创建新连接池 {}", mux_key);
        }

        self.sessions
            .insert(session_id.to_string(), session);

        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self, session_id: &str) -> Result<(), LiteError> {
        // 检查是否是MUX会话
        let mut mux_key_to_decrement: Option<String> = None;

        for (key, pool_item) in &mut self.mux_pools {
            if pool_item.info.id == session_id {
                mux_key_to_decrement = Some(key.clone());
                break;
            }
        }

        // 减少MUX引用计数
        if let Some(key) = mux_key_to_decrement {
            if let Some(pool_item) = self.mux_pools.get_mut(&key) {
                pool_item.ref_count = pool_item.ref_count.saturating_sub(1);
                log::info!("SSH MUX: 释放连接 {} (ref={})", key, pool_item.ref_count);
            }
        }

        // 从会话表移除
        if let Some(session) = self.sessions.remove(session_id) {
            let session = session.lock().await;
            session
                .disconnect(None, "Normal shutdown", None)
                .map_err(|e| LiteError::Ssh(e.to_string()))?;
        }
        Ok(())
    }

    /// 执行命令
    pub async fn execute(&self, session_id: &str, command: &str) -> Result<String, LiteError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or(LiteError::Ssh("会话不存在".to_string()))?;

        let session = session.lock().await;
        let mut channel = session
            .channel_session()
            .map_err(|e| LiteError::Ssh(e.to_string()))?;

        channel
            .exec(command)
            .map_err(|e| LiteError::Ssh(e.to_string()))?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| LiteError::Ssh(e.to_string()))?;

        channel
            .wait_close()
            .map_err(|e| LiteError::Ssh(e.to_string()))?;

        Ok(output)
    }

    /// 获取所有活跃会话
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    /// 检查会话是否存在
    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }

    /// 获取MUX统计信息
    pub fn get_mux_stats(&self) -> MuxStats {
        let pools: Vec<MuxPoolInfo> = self.mux_pools
            .iter()
            .map(|(key, pool)| MuxPoolInfo {
                key: key.clone(),
                ref_count: pool.ref_count,
                idle_seconds: pool.last_used.elapsed().as_secs(),
            })
            .collect();

        MuxStats {
            total_pools: pools.len(),
            pools,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MuxStats {
    pub total_pools: usize,
    pub pools: Vec<MuxPoolInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MuxPoolInfo {
    pub key: String,
    pub ref_count: usize,
    pub idle_seconds: u64,
}

impl Default for SshSessionManager {
    fn default() -> Self {
        Self::new()
    }
}
