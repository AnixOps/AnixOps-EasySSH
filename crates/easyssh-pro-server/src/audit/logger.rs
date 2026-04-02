//! 审计日志记录器
//! 提供高性能、可靠的审计日志记录功能

use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use super::{
    storage::{AuditStorage, PostgresStorage},
    ActionResult, AuditConfig, AuditEventType, AuditRecord, AuditResult, ComplianceInfo,
    DataClassification,
};

/// 审计日志记录器
pub struct AuditLogger {
    /// 主存储后端 (PostgreSQL)
    storage: Arc<dyn AuditStorage>,
    /// 配置
    config: AuditConfig,
    /// 签名密钥 (用于防篡改)
    signing_key: Option<Vec<u8>>,
    /// 最后记录的哈希 (用于链式哈希)
    last_hash: Arc<RwLock<Option<String>>>,
    /// 异步写入通道
    write_tx: mpsc::Sender<AuditRecord>,
    /// 实时告警发送器
    alert_tx: Option<mpsc::Sender<AuditRecord>>,
}

impl AuditLogger {
    /// 创建新的审计日志记录器
    pub async fn new(config: AuditConfig) -> AuditResult<Self> {
        // 初始化PostgreSQL存储
        let storage: Arc<dyn AuditStorage> =
            Arc::new(PostgresStorage::new(&config.postgres_url).await?);

        // 解析签名密钥
        let signing_key = config.signing_key.as_ref().map(|k| k.as_bytes().to_vec());

        // 创建异步写入通道
        let (write_tx, mut write_rx) = mpsc::channel::<AuditRecord>(10000);

        // 创建告警通道 (如果启用了告警)
        let alert_tx = if config.alerting.enabled {
            let (tx, mut rx) = mpsc::channel::<AuditRecord>(1000);

            // 启动告警处理任务
            tokio::spawn(async move {
                while let Some(record) = rx.recv().await {
                    if record.event_type.requires_alert() {
                        // 触发告警 (通过AlertEngine)
                        debug!("Alert triggered for event: {:?}", record.event_type);
                    }
                }
            });

            Some(tx)
        } else {
            None
        };

        // 启动后台写入任务
        let storage_clone = storage.clone();
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(100);
            let mut batch_timer = tokio::time::interval(tokio::time::Duration::from_secs(1));

            loop {
                tokio::select! {
                    Some(record) = write_rx.recv() => {
                        batch.push(record);

                        if batch.len() >= 100 {
                            if let Err(e) = storage_clone.store_batch(&batch).await {
                                error!("Failed to store audit batch: {}", e);
                            }
                            batch.clear();
                        }
                    }
                    _ = batch_timer.tick() => {
                        if !batch.is_empty() {
                            if let Err(e) = storage_clone.store_batch(&batch).await {
                                error!("Failed to store audit batch: {}", e);
                            }
                            batch.clear();
                        }
                    }
                }
            }
        });

        Ok(Self {
            storage,
            config,
            signing_key,
            last_hash: Arc::new(RwLock::new(None)),
            write_tx,
            alert_tx,
        })
    }

    /// 记录审计事件 (简化接口)
    pub async fn log(
        &self,
        event_type: AuditEventType,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
    ) -> AuditResult<()> {
        let record = AuditRecord::new(event_type, user_id, user_name, ip_address);
        self.log_record(record).await
    }

    /// 记录审计事件 (带详情)
    pub async fn log_with_details(
        &self,
        event_type: AuditEventType,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
        details: serde_json::Value,
    ) -> AuditResult<()> {
        let record = AuditRecord::new(event_type, user_id, user_name, ip_address)
            .with_details(details);
        self.log_record(record).await
    }

    /// 记录完整的审计记录
    pub async fn log_record(&self, record: AuditRecord) -> AuditResult<()> {
        let mut record = record;

        // 添加合规信息
        record.compliance = self.build_compliance_info(&record);

        // 密封记录 (添加哈希链和签名)
        if self.signing_key.is_some() {
            let last_hash = self.last_hash.read().await.clone();
            record = record.seal(last_hash.as_deref(), self.signing_key.as_deref());

            // 更新最后哈希
            let mut guard = self.last_hash.write().await;
            *guard = Some(record.compute_hash());
        }

        // 发送到告警通道 (如果是需要告警的事件)
        if record.event_type.requires_alert() {
            if let Some(ref tx) = self.alert_tx {
                let _ = tx.send(record.clone()).await;
            }
        }

        // 发送到写入通道
        self.write_tx
            .send(record)
            .await
            .map_err(|e| super::AuditError::Storage(format!("Failed to queue record: {}", e)))?;

        Ok(())
    }

    /// 批量记录审计事件
    pub async fn log_batch(&self, records: Vec<AuditRecord>) -> AuditResult<usize> {
        let count = records.len();

        for record in records {
            self.log_record(record).await?;
        }

        Ok(count)
    }

    /// 记录认证事件
    pub async fn log_auth(
        &self,
        event_type: AuditEventType,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
        success: bool,
        details: Option<serde_json::Value>,
    ) -> AuditResult<()> {
        let result = if success {
            ActionResult::Success
        } else {
            ActionResult::Failure
        };

        let mut record =
            AuditRecord::new(event_type, user_id, user_name, ip_address).with_result(result);

        if let Some(details) = details {
            record = record.with_details(details);
        }

        self.log_record(record).await
    }

    /// 记录服务器事件
    pub async fn log_server(
        &self,
        event_type: AuditEventType,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
        server_id: impl Into<String>,
        server_name: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> AuditResult<()> {
        let mut record = AuditRecord::new(event_type, user_id, user_name, ip_address)
            .with_resource("server", server_id);

        let mut details_map = details.unwrap_or_else(|| serde_json::json!({}));

        if let serde_json::Value::Object(ref mut map) = details_map {
            map.insert("server_name".to_string(), serde_json::json!(server_name.into()));
        }

        record = record.with_details(details_map);

        self.log_record(record).await
    }

    /// 记录团队事件
    pub async fn log_team(
        &self,
        event_type: AuditEventType,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
        team_id: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> AuditResult<()> {
        let record = AuditRecord::new(event_type, user_id, user_name, ip_address)
            .with_team_id(team_id)
            .with_resource("team", "")
            .with_details(details.unwrap_or_else(|| serde_json::json!({})));

        self.log_record(record).await
    }

    /// 记录安全事件
    pub async fn log_security(
        &self,
        event_type: AuditEventType,
        user_id: Option<impl Into<String>>,
        user_name: Option<impl Into<String>>,
        ip_address: impl Into<String>,
        details: serde_json::Value,
    ) -> AuditResult<()> {
        let user_id = user_id
            .map(|id| id.into())
            .unwrap_or_else(|| "anonymous".to_string());
        let user_name = user_name
            .map(|name| name.into())
            .unwrap_or_else(|| "Anonymous".to_string());

        let record = AuditRecord::new(event_type, user_id, user_name, ip_address)
            .with_result(ActionResult::Denied)
            .with_details(details);

        self.log_record(record).await
    }

    /// 记录会话事件
    pub async fn log_session(
        &self,
        event_type: AuditEventType,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
        session_id: impl Into<String>,
        server_id: Option<impl Into<String>>,
        details: Option<serde_json::Value>,
    ) -> AuditResult<()> {
        let mut record = AuditRecord::new(event_type, user_id, user_name, ip_address)
            .with_session_id(session_id);

        if let Some(server_id) = server_id {
            record = record.with_resource("server", server_id);
        }

        if let Some(details) = details {
            record = record.with_details(details);
        }

        self.log_record(record).await
    }

    /// 记录配置变更事件
    pub async fn log_config_change(
        &self,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
        config_key: impl Into<String>,
        old_value: Option<serde_json::Value>,
        new_value: Option<serde_json::Value>,
    ) -> AuditResult<()> {
        let details = serde_json::json!({
            "config_key": config_key.into(),
            "old_value": old_value,
            "new_value": new_value,
        });

        let record = AuditRecord::new(
            AuditEventType::ConfigChanged,
            user_id,
            user_name,
            ip_address,
        )
        .with_resource("config", "")
        .with_details(details);

        self.log_record(record).await
    }

    /// 记录命令执行
    pub async fn log_command(
        &self,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
        session_id: impl Into<String>,
        server_id: impl Into<String>,
        command: impl Into<String>,
        exit_code: Option<i32>,
    ) -> AuditResult<()> {
        let details = serde_json::json!({
            "command": command.into(),
            "exit_code": exit_code,
        });

        let result = match exit_code {
            Some(0) => ActionResult::Success,
            Some(_) => ActionResult::Failure,
            None => ActionResult::Error,
        };

        let record = AuditRecord::new(
            AuditEventType::CommandExecuted,
            user_id,
            user_name,
            ip_address,
        )
        .with_session_id(session_id)
        .with_resource("server", server_id)
        .with_result(result)
        .with_details(details);

        self.log_record(record).await
    }

    /// 记录文件传输
    pub async fn log_file_transfer(
        &self,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
        session_id: impl Into<String>,
        server_id: impl Into<String>,
        direction: TransferDirection,
        file_path: impl Into<String>,
        file_size: u64,
        success: bool,
    ) -> AuditResult<()> {
        let details = serde_json::json!({
            "direction": match direction {
                TransferDirection::Upload => "upload",
                TransferDirection::Download => "download",
            },
            "file_path": file_path.into(),
            "file_size": file_size,
        });

        let result = if success {
            ActionResult::Success
        } else {
            ActionResult::Failure
        };

        let record = AuditRecord::new(
            AuditEventType::FileTransferred,
            user_id,
            user_name,
            ip_address,
        )
        .with_session_id(session_id)
        .with_resource("server", server_id)
        .with_result(result)
        .with_details(details);

        self.log_record(record).await
    }

    /// 记录可疑活动
    pub async fn log_suspicious_activity(
        &self,
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
        activity_type: impl Into<String>,
        description: impl Into<String>,
        risk_score: f64,
        details: Option<serde_json::Value>,
    ) -> AuditResult<()> {
        let mut merged_details = details.unwrap_or_else(|| serde_json::json!({}));

        if let serde_json::Value::Object(ref mut map) = merged_details {
            map.insert("activity_type".to_string(), serde_json::json!(activity_type.into()));
            map.insert("description".to_string(), serde_json::json!(description.into()));
            map.insert("risk_score".to_string(), serde_json::json!(risk_score));
            map.insert("auto_blocked".to_string(), serde_json::json!(risk_score > 0.8));
        }

        let record = AuditRecord::new(
            AuditEventType::SuspiciousActivity,
            user_id,
            user_name,
            ip_address,
        )
        .with_result(ActionResult::Denied)
        .with_details(merged_details);

        self.log_record(record).await
    }

    /// 获取存储接口
    pub fn storage(&self) -> Arc<dyn AuditStorage> {
        self.storage.clone()
    }

    /// 关闭记录器
    pub async fn shutdown(&self) -> AuditResult<()> {
        info!("Shutting down audit logger...");

        // 等待队列中的记录写入完成
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        Ok(())
    }

    /// 构建合规信息
    fn build_compliance_info(&self, record: &AuditRecord) -> ComplianceInfo {
        let frameworks = self.config.compliance_frameworks.clone();

        let data_classification = match record.event_type {
            AuditEventType::SecretViewed => DataClassification::Restricted,
            AuditEventType::ServerConnected | AuditEventType::CommandExecuted => {
                DataClassification::Confidential
            }
            _ => DataClassification::Internal,
        };

        ComplianceInfo {
            retention_policy_id: None,
            frameworks,
            data_classification,
            encryption_required: matches!(
                record.event_type,
                AuditEventType::SecretViewed | AuditEventType::KeyUploaded
            ),
            integrity_verified: self.signing_key.is_some(),
        }
    }
}

/// 文件传输方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,
    Download,
}

/// 审计上下文 (用于在请求处理中传递审计信息)
#[derive(Debug, Clone)]
pub struct AuditContext {
    pub user_id: String,
    pub user_name: String,
    pub team_id: Option<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub session_id: Option<String>,
}

impl AuditContext {
    pub fn new(
        user_id: impl Into<String>,
        user_name: impl Into<String>,
        ip_address: impl Into<String>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            user_name: user_name.into(),
            team_id: None,
            ip_address: ip_address.into(),
            user_agent: None,
            session_id: None,
        }
    }

    pub fn with_team_id(mut self, team_id: impl Into<String>) -> Self {
        self.team_id = Some(team_id.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }
}

/// 审计中间件助手
pub struct AuditMiddleware;

impl AuditMiddleware {
    /// 从HTTP请求中提取审计上下文
    pub fn from_request<T>(req: &axum::http::Request<T>) -> Option<AuditContext> {
        use axum::http::header;

        // 从请求头中提取信息
        let user_id = req
            .headers()
            .get("X-User-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "anonymous".to_string());

        let user_name = req
            .headers()
            .get("X-User-Name")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Anonymous".to_string());

        let ip_address = req
            .headers()
            .get("X-Forwarded-For")
            .or_else(|| req.headers().get("X-Real-Ip"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let user_agent = req
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let session_id = req
            .headers()
            .get("X-Session-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Some(AuditContext {
            user_id,
            user_name,
            team_id: None,
            ip_address,
            user_agent,
            session_id,
        })
    }
}
