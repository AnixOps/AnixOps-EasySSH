//! 审计查询引擎
//! 提供高级查询、聚合和分析功能

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

use super::{
    models::{
        AuditFilter, IpActivitySummary, PaginatedAuditResult, QueryOptions, SortField, SortOrder,
        TimeAggregatedResult, UserActivitySummary,
    },
    storage::AuditStorage,
    AuditCategory, AuditEventType, AuditRecord, AuditResult, Severity,
};

/// 审计查询引擎
pub struct AuditQuery {
    storage: std::sync::Arc<dyn AuditStorage>,
}

impl AuditQuery {
    /// 创建新的查询引擎
    pub fn new(storage: std::sync::Arc<dyn AuditStorage>) -> Self {
        Self { storage }
    }

    /// 执行查询
    pub async fn query(&self, options: QueryOptions) -> AuditResult<PaginatedAuditResult> {
        let total = self.storage.count(&options.filter).await?;

        let records = self.storage.query(&options.filter).await?;

        let page = options.filter.offset.map(|o| o / options.filter.limit.unwrap_or(50) + 1).unwrap_or(1);
        let page_size = options.filter.limit.unwrap_or(50);

        Ok(PaginatedAuditResult {
            records,
            total,
            page,
            page_size,
            has_more: (page * page_size) < total,
        })
    }

    /// 根据ID获取记录
    pub async fn get_by_id(&self, id: &str) -> AuditResult<Option<AuditRecord>> {
        self.storage.get_by_id(id).await
    }

    /// 获取用户活动时间线
    pub async fn get_user_timeline(
        &self,
        user_id: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
    ) -> AuditResult<Vec<AuditRecord>> {
        let filter = AuditFilter {
            user_ids: Some(vec![user_id.to_string()]),
            start_time: Some(start_time),
            end_time: Some(end_time),
            limit: Some(limit),
            ..Default::default()
        };

        self.storage.query(&filter).await
    }

    /// 获取资源操作历史
    pub async fn get_resource_history(
        &self,
        resource_type: &str,
        resource_id: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> AuditResult<Vec<AuditRecord>> {
        let filter = AuditFilter {
            resource_types: Some(vec![resource_type.to_string()]),
            resource_id: Some(resource_id.to_string()),
            start_time: Some(start_time),
            end_time: Some(end_time),
            ..Default::default()
        };

        self.storage.query(&filter).await
    }

    /// 获取会话历史
    pub async fn get_session_history(
        &self,
        session_id: &str,
    ) -> AuditResult<Vec<AuditRecord>> {
        let filter = AuditFilter {
            session_id: Some(session_id.to_string()),
            ..Default::default()
        };

        self.storage.query(&filter).await
    }

    /// 获取IP活动摘要
    pub async fn get_ip_summary(
        &self,
        ip_address: &str,
        days: i64,
    ) -> AuditResult<IpActivitySummary> {
        let start_time = Utc::now() - Duration::days(days);

        let filter = AuditFilter {
            ip_addresses: Some(vec![ip_address.to_string()]),
            start_time: Some(start_time),
            end_time: Some(Utc::now()),
            ..Default::default()
        };

        let records = self.storage.query(&filter).await?;

        let mut unique_users = std::collections::HashSet::new();
        let mut unique_teams = std::collections::HashSet::new();
        let mut failed_attempts = 0;
        let mut first_seen: Option<DateTime<Utc>> = None;
        let mut last_seen: Option<DateTime<Utc>> = None;

        for record in &records {
            unique_users.insert(record.user_id.clone());
            if let Some(ref team_id) = record.team_id {
                unique_teams.insert(team_id.clone());
            }

            // 统计失败尝试
            if matches!(record.result, super::ActionResult::Failure | super::ActionResult::Denied) {
                failed_attempts += 1;
            }

            // 追踪时间范围
            if first_seen.is_none() || record.timestamp < first_seen.unwrap() {
                first_seen = Some(record.timestamp);
            }
            if last_seen.is_none() || record.timestamp > last_seen.unwrap() {
                last_seen = Some(record.timestamp);
            }
        }

        // 计算风险分数
        let risk_score = self.calculate_ip_risk_score(&records, failed_attempts);

        Ok(IpActivitySummary {
            ip_address: ip_address.to_string(),
            total_requests: records.len() as i64,
            unique_users: unique_users.into_iter().collect(),
            unique_teams: unique_teams.into_iter().collect(),
            failed_attempts,
            first_seen: first_seen.unwrap_or(start_time),
            last_seen: last_seen.unwrap_or(Utc::now()),
            country: None,
            city: None,
            risk_score,
            is_blocked: false,
        })
    }

    /// 获取用户活动摘要
    pub async fn get_user_summary(
        &self,
        user_id: &str,
        days: i64,
    ) -> AuditResult<UserActivitySummary> {
        let start_time = Utc::now() - Duration::days(days);

        let filter = AuditFilter {
            user_ids: Some(vec![user_id.to_string()]),
            start_time: Some(start_time),
            end_time: Some(Utc::now()),
            ..Default::default()
        };

        let records = self.storage.query(&filter).await?;

        let mut unique_sessions = std::collections::HashSet::new();
        let mut unique_ips = std::collections::HashSet::new();
        let mut hourly_activity: HashMap<u8, i64> = HashMap::new();
        let mut first_activity: Option<DateTime<Utc>> = None;
        let mut last_activity: Option<DateTime<Utc>> = None;
        let user_name = records.first().map(|r| r.user_name.clone()).unwrap_or_default();

        for record in &records {
            if let Some(ref session_id) = record.session_id {
                unique_sessions.insert(session_id.clone());
            }
            unique_ips.insert(record.ip_address.clone());

            let hour = record.timestamp.hour() as u8;
            *hourly_activity.entry(hour).or_insert(0) += 1;

            if first_activity.is_none() || record.timestamp < first_activity.unwrap() {
                first_activity = Some(record.timestamp);
            }
            if last_activity.is_none() || record.timestamp > last_activity.unwrap() {
                last_activity = Some(record.timestamp);
            }
        }

        // 找出最活跃的小时
        let most_active_hour = hourly_activity
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(hour, _)| *hour)
            .unwrap_or(0);

        // 计算风险分数
        let risk_score = self.calculate_user_risk_score(&records, unique_ips.len());

        Ok(UserActivitySummary {
            user_id: user_id.to_string(),
            user_name,
            total_actions: records.len() as i64,
            unique_sessions: unique_sessions.len() as i64,
            unique_ips: unique_ips.len() as i64,
            first_activity: first_activity.unwrap_or(start_time),
            last_activity: last_activity.unwrap_or(Utc::now()),
            most_active_hour,
            risk_score,
        })
    }

    /// 时间聚合查询
    pub async fn aggregate_by_time(
        &self,
        filter: AuditFilter,
        bucket_size_minutes: i64,
    ) -> AuditResult<Vec<TimeAggregatedResult>> {
        let records = self.storage.query(&filter).await?;

        let mut buckets: HashMap<DateTime<Utc>, Vec<&AuditRecord>> = HashMap::new();

        // 按时间桶分组
        for record in &records {
            let bucket_start = record.timestamp
                - Duration::minutes(
                    (record.timestamp.timestamp() / 60) % bucket_size_minutes,
                );
            buckets.entry(bucket_start).or_default().push(record);
        }

        let mut results: Vec<TimeAggregatedResult> = buckets
            .iter()
            .map(|(start, records)| {
                let mut events_by_type: HashMap<String, i64> = HashMap::new();
                let mut unique_users = std::collections::HashSet::new();

                for record in records {
                    let event_type = format!("{:?}", record.event_type);
                    *events_by_type.entry(event_type).or_insert(0) += 1;
                    unique_users.insert(record.user_id.clone());
                }

                TimeAggregatedResult {
                    bucket_start: *start,
                    bucket_end: *start + Duration::minutes(bucket_size_minutes),
                    count: records.len() as i64,
                    events_by_type,
                    unique_users: unique_users.len() as i64,
                }
            })
            .collect();

        results.sort_by(|a, b| a.bucket_start.cmp(&b.bucket_start));
        Ok(results)
    }

    /// 获取热门事件类型
    pub async fn get_top_event_types(
        &self,
        filter: &AuditFilter,
        limit: usize,
    ) -> AuditResult<Vec<(AuditEventType, i64)>> {
        let records = self.storage.query(filter).await?;

        let mut counts: HashMap<AuditEventType, i64> = HashMap::new();
        for record in &records {
            *counts.entry(record.event_type).or_insert(0) += 1;
        }

        let mut result: Vec<(AuditEventType, i64)> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result.truncate(limit);

        Ok(result)
    }

    /// 获取活跃用户
    pub async fn get_most_active_users(
        &self,
        filter: &AuditFilter,
        limit: usize,
    ) -> AuditResult<Vec<(String, String, i64)>> {
        let records = self.storage.query(filter).await?;

        let mut counts: HashMap<String, (String, i64)> = HashMap::new();
        for record in &records {
            let entry = counts
                .entry(record.user_id.clone())
                .or_insert((record.user_name.clone(), 0));
            entry.1 += 1;
        }

        let mut result: Vec<(String, String, i64)> = counts
            .into_iter()
            .map(|(id, (name, count))| (id, name, count))
            .collect();
        result.sort_by(|a, b| b.2.cmp(&a.2));
        result.truncate(limit);

        Ok(result)
    }

    /// 检测异常行为
    pub async fn detect_anomalies(
        &self,
        user_id: Option<&str>,
        days: i64,
    ) -> AuditResult<Vec<super::Anomaly>> {
        let start_time = Utc::now() - Duration::days(days);

        let filter = AuditFilter {
            user_ids: user_id.map(|id| vec![id.to_string()]),
            start_time: Some(start_time),
            end_time: Some(Utc::now()),
            ..Default::default()
        };

        let records = self.storage.query(&filter).await?;
        let mut anomalies = Vec::new();

        // 检测登录失败异常
        self.detect_login_failures(&records, &mut anomalies);

        // 检测地理位置异常
        self.detect_geo_anomalies(&records, &mut anomalies);

        // 检测时间异常
        self.detect_time_anomalies(&records, &mut anomalies);

        // 检测频率异常
        self.detect_frequency_anomalies(&records, &mut anomalies);

        Ok(anomalies)
    }

    /// 生成审计摘要
    pub async fn generate_summary(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> AuditResult<super::AuditSummary> {
        let filter = AuditFilter {
            start_time: Some(start_time),
            end_time: Some(end_time),
            ..Default::default()
        };

        let records = self.storage.query(&filter).await?;

        let total = records.len() as i64;
        let mut by_category: HashMap<String, i64> = HashMap::new();
        let mut by_severity: HashMap<String, i64> = HashMap::new();
        let mut failed_logins = 0i64;
        let mut unique_users = std::collections::HashSet::new();
        let mut unique_ips = std::collections::HashSet::new();

        for record in &records {
            let category = format!("{:?}", record.category);
            *by_category.entry(category).or_insert(0) += 1;

            let severity = format!("{:?}", record.severity);
            *by_severity.entry(severity).or_insert(0) += 1;

            if record.event_type == AuditEventType::LoginFailure {
                failed_logins += 1;
            }

            unique_users.insert(record.user_id.clone());
            unique_ips.insert(record.ip_address.clone());
        }

        Ok(super::AuditSummary {
            total_records: total,
            records_by_category: by_category,
            records_by_severity: by_severity,
            failed_logins,
            unique_users: unique_users.len() as i64,
            unique_ips: unique_ips.len() as i64,
            time_range_start: start_time,
            time_range_end: end_time,
        })
    }

    // 私有辅助方法

    fn calculate_ip_risk_score(&self, records: &[AuditRecord], failed_attempts: i64) -> f64 {
        let base_score = 0.0;

        // 登录失败增加风险
        let failure_score = (failed_attempts as f64 / records.len() as f64) * 50.0;

        // 可疑事件增加风险
        let suspicious_count = records
            .iter()
            .filter(|r| {
                matches!(
                    r.event_type,
                    AuditEventType::SuspiciousActivity | AuditEventType::BruteForceAttempt
                )
            })
            .count() as f64;
        let suspicious_score = (suspicious_count / records.len() as f64) * 100.0;

        // 权限拒绝增加风险
        let denied_count = records
            .iter()
            .filter(|r| matches!(r.result, super::ActionResult::Denied))
            .count() as f64;
        let denied_score = (denied_count / records.len() as f64) * 30.0;

        let total_score = base_score + failure_score + suspicious_score + denied_score;
        total_score.min(100.0)
    }

    fn calculate_user_risk_score(&self, records: &[AuditRecord], unique_ip_count: usize) -> f64 {
        let base_score = 0.0;

        // 多IP登录增加风险
        let ip_score = if unique_ip_count > 3 {
            (unique_ip_count as f64 - 3.0) * 10.0
        } else {
            0.0
        };

        // 登录失败增加风险
        let failed_count = records
            .iter()
            .filter(|r| r.event_type == AuditEventType::LoginFailure)
            .count() as f64;
        let failure_score = (failed_count / records.len().max(1) as f64) * 40.0;

        // 可疑活动增加风险
        let suspicious_count = records
            .iter()
            .filter(|r| {
                matches!(
                    r.event_type,
                    AuditEventType::SuspiciousActivity
                        | AuditEventType::BruteForceAttempt
                        | AuditEventType::PermissionDenied
                )
            })
            .count() as f64;
        let suspicious_score = (suspicious_count / records.len().max(1) as f64) * 100.0;

        let total_score = base_score + ip_score + failure_score + suspicious_score;
        total_score.min(100.0)
    }

    fn detect_login_failures(&self, records: &[AuditRecord], anomalies: &mut Vec<super::Anomaly>) {
        let failed_logins: Vec<&AuditRecord> = records
            .iter()
            .filter(|r| r.event_type == AuditEventType::LoginFailure)
            .collect();

        if failed_logins.len() >= 5 {
            anomalies.push(super::Anomaly {
                anomaly_type: "multiple_login_failures".to_string(),
                severity: Severity::High,
                description: format!("检测到 {} 次登录失败", failed_logins.len()),
                affected_user: Some(failed_logins[0].user_id.clone()),
                affected_ip: Some(failed_logins[0].ip_address.clone()),
                detected_at: Utc::now(),
                related_events: failed_logins.iter().map(|r| r.id.clone()).collect(),
            });
        }
    }

    fn detect_geo_anomalies(&self, _records: &[AuditRecord], _anomalies: &mut Vec<super::Anomaly>) {
        // 地理位置异常检测 (需要IP地理位置库)
        // 检测短时间内从多个国家登录
    }

    fn detect_time_anomalies(&self, records: &[AuditRecord], anomalies: &mut Vec<super::Anomaly>) {
        // 检测非工作时间活动
        let after_hours: Vec<&AuditRecord> = records
            .iter()
            .filter(|r| {
                let hour = r.timestamp.hour();
                hour < 6 || hour > 22
            })
            .collect();

        if after_hours.len() > 10 {
            anomalies.push(super::Anomaly {
                anomaly_type: "after_hours_activity".to_string(),
                severity: Severity::Medium,
                description: format!("检测到 {} 次非工作时间活动", after_hours.len()),
                affected_user: after_hours.first().map(|r| r.user_id.clone()),
                affected_ip: after_hours.first().map(|r| r.ip_address.clone()),
                detected_at: Utc::now(),
                related_events: after_hours.iter().map(|r| r.id.clone()).collect(),
            });
        }
    }

    fn detect_frequency_anomalies(
        &self,
        records: &[AuditRecord],
        anomalies: &mut Vec<super::Anomaly>,
    ) {
        // 检测活动频率异常
        if records.len() > 1000 {
            let user_id = records[0].user_id.clone();
            let ip_address = records[0].ip_address.clone();

            anomalies.push(super::Anomaly {
                anomaly_type: "high_frequency_activity".to_string(),
                severity: Severity::Medium,
                description: format!("检测到高频活动: {} 次事件", records.len()),
                affected_user: Some(user_id),
                affected_ip: Some(ip_address),
                detected_at: Utc::now(),
                related_events: records.iter().take(10).map(|r| r.id.clone()).collect(),
            });
        }
    }
}

/// 查询构建器
pub struct QueryBuilder {
    filter: AuditFilter,
    sort_field: SortField,
    sort_order: SortOrder,
    include_details: bool,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            filter: AuditFilter::default(),
            sort_field: SortField::Timestamp,
            sort_order: SortOrder::Desc,
            include_details: true,
        }
    }

    pub fn with_time_range(
        mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Self {
        self.filter.start_time = Some(start);
        self.filter.end_time = Some(end);
        self
    }

    pub fn with_event_types(mut self, types: Vec<AuditEventType>) -> Self {
        self.filter.event_types = Some(types);
        self
    }

    pub fn with_categories(mut self, categories: Vec<AuditCategory>) -> Self {
        self.filter.categories = Some(categories);
        self
    }

    pub fn with_severities(mut self, severities: Vec<Severity>) -> Self {
        self.filter.severities = Some(severities);
        self
    }

    pub fn with_users(mut self, user_ids: Vec<String>) -> Self {
        self.filter.user_ids = Some(user_ids);
        self
    }

    pub fn with_team(mut self, team_id: impl Into<String>) -> Self {
        self.filter.team_id = Some(team_id.into());
        self
    }

    pub fn with_resource(
        mut self,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        self.filter.resource_types = Some(vec![resource_type.into()]);
        self.filter.resource_id = Some(resource_id.into());
        self
    }

    pub fn with_ip_addresses(mut self, ips: Vec<String>) -> Self {
        self.filter.ip_addresses = Some(ips);
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.filter.session_id = Some(session_id.into());
        self
    }

    pub fn with_pagination(mut self, page: i64, page_size: i64) -> Self {
        self.filter.limit = Some(page_size);
        self.filter.offset = Some((page - 1) * page_size);
        self
    }

    pub fn with_sort(mut self, field: SortField, order: SortOrder) -> Self {
        self.sort_field = field;
        self.sort_order = order;
        self
    }

    pub fn include_details(mut self, include: bool) -> Self {
        self.include_details = include;
        self
    }

    pub fn build(self) -> QueryOptions {
        QueryOptions {
            filter: self.filter,
            sort_field: self.sort_field,
            sort_order: self.sort_order,
            include_details: self.include_details,
            include_location: true,
        }
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 查询结果包装器
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub records: Vec<AuditRecord>,
    pub total_count: i64,
    pub execution_time_ms: u64,
    pub query_id: String,
}
