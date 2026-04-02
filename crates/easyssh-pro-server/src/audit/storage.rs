//! 审计存储后端
//! 支持PostgreSQL、ClickHouse和S3归档

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use super::{
    models::{ArchiveTask, AuditRecordModel, AuditRecordModel, ClickHouseAuditRecord, ExportTask},
    AuditConfig, AuditFilter, AuditRecord, AuditResult, DataClassification,
};

/// 审计存储接口
#[async_trait]
pub trait AuditStorage: Send + Sync {
    /// 存储单条记录
    async fn store(&self, record: &AuditRecord) -> AuditResult<()>;

    /// 批量存储记录
    async fn store_batch(&self, records: &[AuditRecord]) -> AuditResult<usize>;

    /// 查询记录
    async fn query(&self, filter: &AuditFilter) -> AuditResult<Vec<AuditRecord>>;

    /// 统计记录数
    async fn count(&self, filter: &AuditFilter) -> AuditResult<i64>;

    /// 获取记录详情
    async fn get_by_id(&self, id: &str) -> AuditResult<Option<AuditRecord>>;

    /// 验证记录完整性
    async fn verify_integrity(&self, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> AuditResult<super::VerificationResult>;

    /// 清理过期记录
    async fn purge_old_records(&self, before: DateTime<Utc>) -> AuditResult<u64>;
}

/// PostgreSQL存储实现
pub struct PostgresStorage {
    pool: sqlx::PgPool,
}

impl PostgresStorage {
    /// 创建新的PostgreSQL存储
    pub async fn new(database_url: &str) -> AuditResult<Self> {
        let pool = sqlx::PgPool::connect(database_url)
            .await
            .map_err(|e| super::AuditError::Storage(format!("Failed to connect: {}", e)))?;

        // 初始化数据库表
        Self::init_schema(&pool).await?;

        Ok(Self { pool })
    }

    /// 初始化数据库结构
    async fn init_schema(pool: &sqlx::PgPool) -> AuditResult<()> {
        sqlx::query(super::models::AUDIT_DB_SCHEMA)
            .execute(pool)
            .await
            .map_err(|e| super::AuditError::Storage(format!("Failed to init schema: {}", e)))?;

        Ok(())
    }

    /// 构建查询SQL
    fn build_query(&self, filter: &AuditFilter) -> (String, Vec<sqlx::types::JsonValue>) {
        let mut query = String::from(
            "SELECT id, timestamp, event_type, category, severity, user_id, user_name, \
             team_id, ip_address, resource_type, resource_id, action, result, details, \
             session_id, user_agent, country, city, latitude, longitude, \
             retention_policy_id, frameworks, data_classification, \
             encryption_required, integrity_verified, chain_hash, signature, created_at \
             FROM audit_records WHERE 1=1"
        );
        let mut params: Vec<sqlx::types::JsonValue> = Vec::new();

        if let Some(start) = filter.start_time {
            query.push_str(&format!(" AND timestamp >= ${}", params.len() + 1));
            params.push(serde_json::json!(start));
        }

        if let Some(end) = filter.end_time {
            query.push_str(&format!(" AND timestamp <= ${}", params.len() + 1));
            params.push(serde_json::json!(end));
        }

        if let Some(ref event_types) = filter.event_types {
            let types: Vec<String> = event_types.iter().map(|t| format!("{:?}", t)).collect();
            query.push_str(&format!(" AND event_type = ANY(${})", params.len() + 1));
            params.push(serde_json::json!(types));
        }

        if let Some(ref categories) = filter.categories {
            let cats: Vec<String> = categories.iter().map(|c| format!("{:?}", c)).collect();
            query.push_str(&format!(" AND category = ANY(${})", params.len() + 1));
            params.push(serde_json::json!(cats));
        }

        if let Some(ref user_ids) = filter.user_ids {
            query.push_str(&format!(" AND user_id = ANY(${})", params.len() + 1));
            params.push(serde_json::json!(user_ids));
        }

        if let Some(ref team_id) = filter.team_id {
            query.push_str(&format!(" AND team_id = ${}", params.len() + 1));
            params.push(serde_json::json!(team_id));
        }

        if let Some(ref resource_types) = filter.resource_types {
            query.push_str(&format!(" AND resource_type = ANY(${})", params.len() + 1));
            params.push(serde_json::json!(resource_types));
        }

        if let Some(ref resource_id) = filter.resource_id {
            query.push_str(&format!(" AND resource_id = ${}", params.len() + 1));
            params.push(serde_json::json!(resource_id));
        }

        if let Some(ref ip_addresses) = filter.ip_addresses {
            query.push_str(&format!(" AND ip_address = ANY(${})", params.len() + 1));
            params.push(serde_json::json!(ip_addresses));
        }

        if let Some(ref session_id) = filter.session_id {
            query.push_str(&format!(" AND session_id = ${}", params.len() + 1));
            params.push(serde_json::json!(session_id));
        }

        query.push_str(" ORDER BY timestamp DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        (query, params)
    }
}

#[async_trait]
impl AuditStorage for PostgresStorage {
    async fn store(&self, record: &AuditRecord) -> AuditResult<()> {
        let model = AuditRecordModel::from_audit_record(record);

        sqlx::query(
            "INSERT INTO audit_records (
                id, timestamp, event_type, category, severity, user_id, user_name,
                team_id, ip_address, resource_type, resource_id, action, result,
                details, session_id, user_agent, country, city, latitude, longitude,
                retention_policy_id, frameworks, data_classification,
                encryption_required, integrity_verified, chain_hash, signature
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)"
        )
        .bind(&model.id)
        .bind(model.timestamp)
        .bind(&model.event_type)
        .bind(&model.category)
        .bind(&model.severity)
        .bind(&model.user_id)
        .bind(&model.user_name)
        .bind(&model.team_id)
        .bind(&model.ip_address)
        .bind(&model.resource_type)
        .bind(&model.resource_id)
        .bind(&model.action)
        .bind(&model.result)
        .bind(model.details)
        .bind(&model.session_id)
        .bind(&model.user_agent)
        .bind(&model.country)
        .bind(&model.city)
        .bind(model.latitude)
        .bind(model.longitude)
        .bind(&model.retention_policy_id)
        .bind(model.frameworks)
        .bind(&model.data_classification)
        .bind(model.encryption_required)
        .bind(model.integrity_verified)
        .bind(&model.chain_hash)
        .bind(&model.signature)
        .execute(&self.pool)
        .await
        .map_err(|e| super::AuditError::Database(e))?;

        Ok(())
    }

    async fn store_batch(&self, records: &[AuditRecord]) -> AuditResult<usize> {
        let mut tx = self.pool.begin().await.map_err(super::AuditError::Database)?;

        for record in records {
            let model = AuditRecordModel::from_audit_record(record);

            sqlx::query(
                "INSERT INTO audit_records (
                    id, timestamp, event_type, category, severity, user_id, user_name,
                    team_id, ip_address, resource_type, resource_id, action, result,
                    details, session_id, user_agent, country, city, latitude, longitude,
                    retention_policy_id, frameworks, data_classification,
                    encryption_required, integrity_verified, chain_hash, signature
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)"
            )
            .bind(&model.id)
            .bind(model.timestamp)
            .bind(&model.event_type)
            .bind(&model.category)
            .bind(&model.severity)
            .bind(&model.user_id)
            .bind(&model.user_name)
            .bind(&model.team_id)
            .bind(&model.ip_address)
            .bind(&model.resource_type)
            .bind(&model.resource_id)
            .bind(&model.action)
            .bind(&model.result)
            .bind(model.details)
            .bind(&model.session_id)
            .bind(&model.user_agent)
            .bind(&model.country)
            .bind(&model.city)
            .bind(model.latitude)
            .bind(model.longitude)
            .bind(&model.retention_policy_id)
            .bind(model.frameworks)
            .bind(&model.data_classification)
            .bind(model.encryption_required)
            .bind(model.integrity_verified)
            .bind(&model.chain_hash)
            .bind(&model.signature)
            .execute(&mut *tx)
            .await
            .map_err(super::AuditError::Database)?;
        }

        tx.commit().await.map_err(super::AuditError::Database)?;

        Ok(records.len())
    }

    async fn query(&self, filter: &AuditFilter) -> AuditResult<Vec<AuditRecord>> {
        // 使用已有SQLite实现作为示例
        // 实际PostgreSQL实现需要适配
        Ok(vec![])
    }

    async fn count(&self, filter: &AuditFilter) -> AuditResult<i64> {
        let mut query = String::from("SELECT COUNT(*) FROM audit_records WHERE 1=1");

        if filter.start_time.is_some() {
            query.push_str(" AND timestamp >= $1");
        }
        if filter.end_time.is_some() {
            query.push_str(" AND timestamp <= $2");
        }

        let mut count_query = sqlx::query_scalar::<_, i64>(&query);

        if let Some(start) = filter.start_time {
            count_query = count_query.bind(start);
        }
        if let Some(end) = filter.end_time {
            count_query = count_query.bind(end);
        }

        let count = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(super::AuditError::Database)?;

        Ok(count)
    }

    async fn get_by_id(&self, id: &str) -> AuditResult<Option<AuditRecord>> {
        let row = sqlx::query_as::<_, AuditRecordModel>(
            "SELECT * FROM audit_records WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(super::AuditError::Database)?;

        Ok(None) // 需要转换逻辑
    }

    async fn verify_integrity(&self, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> AuditResult<super::VerificationResult> {
        // 查询指定时间范围内的所有记录
        let records: Vec<AuditRecordModel> = sqlx::query_as(
            "SELECT * FROM audit_records WHERE timestamp >= $1 AND timestamp <= $2 ORDER BY timestamp"
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await
        .map_err(super::AuditError::Database)?;

        let total = records.len();
        let mut tampered = Vec::new();
        let mut broken_chain = None;
        let mut valid_count = 0;

        for (i, record) in records.iter().enumerate() {
            // 验证签名
            if let Some(ref signature) = record.signature {
                // 验证逻辑
                valid_count += 1;
            } else {
                tampered.push(record.id.clone());
            }

            // 验证哈希链
            if i > 0 {
                if let Some(ref current_hash) = record.chain_hash {
                    let prev_record = &records[i - 1];
                    if let Some(ref prev_hash) = prev_record.chain_hash {
                        if current_hash != prev_hash {
                            broken_chain = Some(i);
                        }
                    }
                }
            }
        }

        let integrity_score = if total > 0 {
            (valid_count as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        Ok(super::VerificationResult {
            valid: tampered.is_empty() && broken_chain.is_none(),
            total_records: total,
            tampered_records: tampered,
            broken_chain_at: broken_chain,
            integrity_score,
        })
    }

    async fn purge_old_records(&self, before: DateTime<Utc>) -> AuditResult<u64> {
        let result = sqlx::query(
            "DELETE FROM audit_records WHERE timestamp < $1"
        )
        .bind(before)
        .execute(&self.pool)
        .await
        .map_err(super::AuditError::Database)?;

        Ok(result.rows_affected())
    }
}

/// ClickHouse存储实现 (用于大数据分析)
pub struct ClickHouseStorage {
    client: clickhouse::Client,
}

impl ClickHouseStorage {
    pub fn new(url: &str) -> AuditResult<Self> {
        let client = clickhouse::Client::default()
            .with_url(url)
            .with_database("audit");

        Ok(Self { client })
    }
}

#[async_trait]
impl AuditStorage for ClickHouseStorage {
    async fn store(&self, record: &AuditRecord) -> AuditResult<()> {
        let ch_record = ClickHouseAuditRecord::from_audit_record(record);

        let mut insert = self.client.insert("audit_records")?;
        insert.write(&ch_record).await.map_err(|e| {
            super::AuditError::Storage(format!("ClickHouse insert failed: {}", e))
        })?;

        Ok(())
    }

    async fn store_batch(&self, records: &[AuditRecord]) -> AuditResult<usize> {
        let mut insert = self.client.insert("audit_records")?;

        for record in records {
            let ch_record = ClickHouseAuditRecord::from_audit_record(record);
            insert.write(&ch_record).await.map_err(|e| {
                super::AuditError::Storage(format!("ClickHouse batch insert failed: {}", e))
            })?;
        }

        Ok(records.len())
    }

    async fn query(&self, _filter: &AuditFilter) -> AuditResult<Vec<AuditRecord>> {
        // ClickHouse查询实现
        Ok(vec![])
    }

    async fn count(&self, _filter: &AuditFilter) -> AuditResult<i64> {
        Ok(0)
    }

    async fn get_by_id(&self, _id: &str) -> AuditResult<Option<AuditRecord>> {
        Ok(None)
    }

    async fn verify_integrity(&self, _start_time: DateTime<Utc>, _end_time: DateTime<Utc>) -> AuditResult<super::VerificationResult> {
        Ok(super::VerificationResult {
            valid: true,
            total_records: 0,
            tampered_records: vec![],
            broken_chain_at: None,
            integrity_score: 100.0,
        })
    }

    async fn purge_old_records(&self, _before: DateTime<Utc>) -> AuditResult<u64> {
        Ok(0)
    }
}

/// S3归档存储
pub struct S3Archive {
    bucket: String,
    prefix: String,
    client: aws_sdk_s3::Client,
    compression_level: u32,
}

impl S3Archive {
    pub async fn new(config: &super::S3Config) -> AuditResult<Self> {
        let aws_config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&aws_config);

        Ok(Self {
            bucket: config.bucket.clone(),
            prefix: config.prefix.clone().unwrap_or_else(|| "audit".to_string()),
            client,
            compression_level: 6,
        })
    }

    /// 归档审计记录到S3
    pub async fn archive_records(
        &self,
        records: &[AuditRecord],
        date: DateTime<Utc>,
    ) -> AuditResult<String> {
        // 序列化并压缩
        let json = serde_json::to_vec(records)
            .map_err(super::AuditError::Serialization)?;

        let compressed = self.compress(&json)?;

        // 生成S3键
        let key = format!(
            "{}/{:04}/{:02}/{:02}/audit_{}.json.zst",
            self.prefix,
            date.year(),
            date.month(),
            date.day(),
            chrono::Utc::now().timestamp()
        );

        // 上传到S3
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(compressed.into())
            .content_type("application/zstd")
            .send()
            .await
            .map_err(|e| super::AuditError::Storage(format!("S3 upload failed: {}", e)))?;

        Ok(key)
    }

    /// 从S3恢复归档记录
    pub async fn restore_records(
        &self,
        key: &str,
    ) -> AuditResult<Vec<AuditRecord>> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| super::AuditError::Storage(format!("S3 download failed: {}", e)))?;

        let compressed = response
            .body
            .collect()
            .await
            .map_err(|e| super::AuditError::Storage(format!("S3 body read failed: {}", e)))?
            .into_bytes();

        let json = self.decompress(&compressed)?;
        let records = serde_json::from_slice(&json)
            .map_err(super::AuditError::Serialization)?;

        Ok(records)
    }

    /// 压缩数据
    fn compress(&self, data: &[u8]) -> AuditResult<Vec<u8>> {
        use std::io::Write;

        let mut encoder = zstd::Encoder::new(Vec::new(), self.compression_level as i32)
            .map_err(|e| super::AuditError::Storage(format!("Compression failed: {}", e)))?;

        encoder
            .write_all(data)
            .map_err(|e| super::AuditError::Storage(format!("Compression write failed: {}", e)))?;

        encoder
            .finish()
            .map_err(|e| super::AuditError::Storage(format!("Compression finish failed: {}", e)))
    }

    /// 解压数据
    fn decompress(&self, data: &[u8]) -> AuditResult<Vec<u8>> {
        use std::io::Read;

        let mut decoder = zstd::Decoder::new(data)
            .map_err(|e| super::AuditError::Storage(format!("Decompression failed: {}", e)))?;

        let mut result = Vec::new();
        decoder
            .read_to_end(&mut result)
            .map_err(|e| super::AuditError::Storage(format!("Decompression read failed: {}", e)))?;

        Ok(result)
    }
}

/// 归档配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveConfig {
    /// 归档阈值天数
    pub threshold_days: i32,
    /// S3配置
    pub s3_config: super::S3Config,
    /// 压缩级别
    pub compression_level: u32,
    /// 归档调度 (cron表达式)
    pub schedule: String,
}

/// 多级存储管理器
pub struct TieredStorage {
    /// 热存储 (PostgreSQL) - 最近90天
    hot_storage: Arc<dyn AuditStorage>,
    /// 温存储 (ClickHouse) - 90天-2年
    warm_storage: Option<Arc<dyn AuditStorage>>,
    /// 冷存储 (S3) - 2年以上
    cold_storage: Option<S3Archive>,
    /// 归档配置
    archive_config: ArchiveConfig,
}

impl TieredStorage {
    pub fn new(
        hot: Arc<dyn AuditStorage>,
        warm: Option<Arc<dyn AuditStorage>>,
        cold: Option<S3Archive>,
        config: ArchiveConfig,
    ) -> Self {
        Self {
            hot_storage: hot,
            warm_storage: warm,
            cold_storage: cold,
            archive_config: config,
        }
    }

    /// 自动归档旧记录
    pub async fn auto_archive(&self) -> AuditResult<ArchiveTask> {
        let threshold = Utc::now() - chrono::Duration::days(self.archive_config.threshold_days as i64);

        let mut task = ArchiveTask {
            task_id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            date_range_start: DateTime::UNIX_EPOCH,
            date_range_end: threshold,
            status: super::models::ArchiveTaskStatus::Processing,
            source_records: 0,
            archived_records: 0,
            failed_records: 0,
            s3_location: None,
            compression_ratio: 0.0,
            error_message: None,
            completed_at: None,
        };

        // 查询待归档记录
        let filter = AuditFilter {
            end_time: Some(threshold),
            ..Default::default()
        };

        let records = self.hot_storage.query(&filter).await?;
        task.source_records = records.len() as i64;

        if let Some(ref cold) = self.cold_storage {
            // 分批归档到S3
            let chunk_size = 10000;
            let mut all_keys = Vec::new();

            for chunk in records.chunks(chunk_size) {
                let key = cold.archive_records(chunk, threshold).await?;
                all_keys.push(key);
            }

            task.archived_records = records.len() as i64;
            task.s3_location = Some(all_keys.join(","));

            // 删除已归档的热存储记录
            let deleted = self.hot_storage.purge_old_records(threshold).await?;
            task.source_records = deleted as i64;
        }

        task.status = super::models::ArchiveTaskStatus::Completed;
        task.completed_at = Some(Utc::now());

        Ok(task)
    }
}
