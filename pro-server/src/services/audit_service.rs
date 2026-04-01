use crate::models::*;
use anyhow::Result;
use chrono::Utc;
use sqlx::AnyPool;
use uuid::Uuid;

pub struct AuditService {
    db: AnyPool,
}

impl AuditService {
    pub fn new(db: AnyPool) -> Self {
        Self { db }
    }

    pub async fn query_logs(
        &self,
        team_id: Option<&str>,
        user_id: Option<&str>,
        action: Option<&str>,
        resource_type: Option<&str>,
        from_date: Option<chrono::DateTime<chrono::Utc>>,
        to_date: Option<chrono::DateTime<chrono::Utc>>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AuditLog>, i64)> {
        let mut query = String::from("SELECT * FROM audit_logs WHERE 1=1");
        let mut count_query = String::from("SELECT COUNT(*) FROM audit_logs WHERE 1=1");

        if team_id.is_some() {
            query.push_str(" AND team_id = ?");
            count_query.push_str(" AND team_id = ?");
        }
        if user_id.is_some() {
            query.push_str(" AND user_id = ?");
            count_query.push_str(" AND user_id = ?");
        }
        if action.is_some() {
            query.push_str(" AND action = ?");
            count_query.push_str(" AND action = ?");
        }
        if resource_type.is_some() {
            query.push_str(" AND resource_type = ?");
            count_query.push_str(" AND resource_type = ?");
        }
        if from_date.is_some() {
            query.push_str(" AND timestamp >= ?");
            count_query.push_str(" AND timestamp >= ?");
        }
        if to_date.is_some() {
            query.push_str(" AND timestamp <= ?");
            count_query.push_str(" AND timestamp <= ?");
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut query_builder = sqlx::query_as::<_, AuditLog>(&query);

        if let Some(tid) = team_id {
            query_builder = query_builder.bind(tid);
        }
        if let Some(uid) = user_id {
            query_builder = query_builder.bind(uid);
        }
        if let Some(act) = action {
            query_builder = query_builder.bind(act);
        }
        if let Some(rt) = resource_type {
            query_builder = query_builder.bind(rt);
        }
        if let Some(from) = from_date {
            query_builder = query_builder.bind(from);
        }
        if let Some(to) = to_date {
            query_builder = query_builder.bind(to);
        }
        query_builder = query_builder.bind(limit).bind(offset);

        let logs = query_builder.fetch_all(&self.db).await?;

        // Get total count
        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query);
        if let Some(tid) = team_id {
            count_builder = count_builder.bind(tid);
        }
        if let Some(uid) = user_id {
            count_builder = count_builder.bind(uid);
        }
        if let Some(act) = action {
            count_builder = count_builder.bind(act);
        }
        if let Some(rt) = resource_type {
            count_builder = count_builder.bind(rt);
        }
        if let Some(from) = from_date {
            count_builder = count_builder.bind(from);
        }
        if let Some(to) = to_date {
            count_builder = count_builder.bind(to);
        }

        let total = count_builder.fetch_one(&self.db).await?;

        Ok((logs, total))
    }

    pub async fn get_stats(
        &self,
        team_id: Option<&str>,
        from_date: Option<chrono::DateTime<chrono::Utc>>,
        to_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<serde_json::Value> {
        let mut query = String::from("SELECT action, COUNT(*) as count FROM audit_logs WHERE 1=1");

        if team_id.is_some() {
            query.push_str(" AND team_id = ?");
        }
        if from_date.is_some() {
            query.push_str(" AND timestamp >= ?");
        }
        if to_date.is_some() {
            query.push_str(" AND timestamp <= ?");
        }

        query.push_str(" GROUP BY action");

        let mut query_builder = sqlx::query(&query);

        if let Some(tid) = team_id {
            query_builder = query_builder.bind(tid);
        }
        if let Some(from) = from_date {
            query_builder = query_builder.bind(from);
        }
        if let Some(to) = to_date {
            query_builder = query_builder.bind(to);
        }

        let rows = query_builder.fetch_all(&self.db).await?;

        let mut stats = serde_json::Map::new();
        for row in rows {
            let action: String = row.try_get("action")?;
            let count: i64 = row.try_get("count")?;
            stats.insert(action, serde_json::Value::Number(count.into()));
        }

        Ok(serde_json::Value::Object(stats))
    }

    pub async fn log_event(
        &self,
        user_id: Option<&str>,
        team_id: Option<&str>,
        action: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        details: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        success: bool,
        error_message: Option<&str>,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO audit_logs (id, timestamp, user_id, team_id, action, resource_type, resource_id, details, ip_address, user_agent, success, error_message) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(now)
        .bind(user_id)
        .bind(team_id)
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(details)
        .bind(ip_address)
        .bind(user_agent)
        .bind(success)
        .bind(error_message)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}
