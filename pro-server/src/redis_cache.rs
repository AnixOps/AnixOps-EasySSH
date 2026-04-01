use anyhow::Result;
use redis::{aio::ConnectionManager, AsyncCommands};
use std::time::Duration;

pub struct RedisCache {
    connection: ConnectionManager,
}

impl RedisCache {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let connection = ConnectionManager::new(client).await?;
        Ok(Self { connection })
    }

    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.connection.clone();
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.connection.clone();
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    pub async fn set(&self, key: &str, value: &str, ttl: Duration) -> Result<()> {
        let mut conn = self.connection.clone();
        conn.set_ex(key, value, ttl.as_secs() as u64).await?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.connection.clone();
        conn.del(key).await?;
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.connection.clone();
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    // Rate limiting helpers
    pub async fn rate_limit_check(&self, key: &str, limit: u32, window_secs: u64) -> Result<bool> {
        let mut conn = self.connection.clone();
        let current_count: Option<u32> = conn.get(key).await?;

        match current_count {
            Some(count) if count >= limit => Ok(false),
            Some(count) => {
                conn.incr(key, 1).await?;
                Ok(true)
            }
            None => {
                conn.set_ex(key, 1, window_secs).await?;
                Ok(true)
            }
        }
    }

    // Session management
    pub async fn store_session(&self, session_id: &str, user_id: &str, ttl: Duration) -> Result<()> {
        self.set(&format!("session:{}", session_id), user_id, ttl).await
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<String>> {
        self.get(&format!("session:{}", session_id)).await
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        self.delete(&format!("session:{}", session_id)).await
    }

    // API Key management
    pub async fn store_api_key(&self, api_key_hash: &str, user_id: &str, ttl: Option<Duration>) -> Result<()> {
        let key = format!("apikey:{}", api_key_hash);
        match ttl {
            Some(duration) => self.set(&key, user_id, duration).await,
            None => {
                let mut conn = self.connection.clone();
                conn.set(&key, user_id).await?;
                Ok(())
            }
        }
    }

    pub async fn get_api_key_user(&self, api_key_hash: &str) -> Result<Option<String>> {
        self.get(&format!("apikey:{}", api_key_hash)).await
    }

    pub async fn revoke_api_key(&self, api_key_hash: &str) -> Result<()> {
        self.delete(&format!("apikey:{}", api_key_hash)).await
    }

    // WebSocket connection management
    pub async fn store_ws_connection(&self, user_id: &str, connection_id: &str, ttl: Duration) -> Result<()> {
        let key = format!("ws:user:{}", user_id);
        let mut conn = self.connection.clone();
        conn.sadd(&key, connection_id).await?;
        conn.expire(&key, ttl.as_secs() as i64).await?;
        Ok(())
    }

    pub async fn remove_ws_connection(&self, user_id: &str, connection_id: &str) -> Result<()> {
        let key = format!("ws:user:{}", user_id);
        let mut conn = self.connection.clone();
        conn.srem(key, connection_id).await?;
        Ok(())
    }

    pub async fn get_user_ws_connections(&self, user_id: &str) -> Result<Vec<String>> {
        let key = format!("ws:user:{}", user_id);
        let mut conn = self.connection.clone();
        let connections: Vec<String> = conn.smembers(key).await?;
        Ok(connections)
    }

    // Real-time collaboration: lock management
    pub async fn acquire_lock(&self, resource_id: &str, user_id: &str, ttl: Duration) -> Result<bool> {
        let key = format!("lock:{}", resource_id);
        let mut conn = self.connection.clone();

        let acquired: bool = conn.set_nx(&key, user_id).await?;
        if acquired {
            conn.expire(&key, ttl.as_secs() as i64).await?;
        }
        Ok(acquired)
    }

    pub async fn release_lock(&self, resource_id: &str, user_id: &str) -> Result<()> {
        let key = format!("lock:{}", resource_id);
        let mut conn = self.connection.clone();

        let current: Option<String> = conn.get(&key).await?;
        if current == Some(user_id.to_string()) {
            conn.del(&key).await?;
        }
        Ok(())
    }
}
