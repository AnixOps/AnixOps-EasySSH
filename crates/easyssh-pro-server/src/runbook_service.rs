//! DevOps事件响应中心 - 运行手册管理服务
//!
//! 提供运行手册的创建、执行、管理和自动化功能

use crate::db::Database;
use crate::incident_models::*;
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct RunbookService {
    db: Arc<Database>,
}

impl RunbookService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    // ============= 运行手册CRUD =============

    /// 创建运行手册
    pub async fn create_runbook(
        &self,
        req: CreateRunbookRequest,
        user_id: &str,
    ) -> Result<Runbook> {
        let runbook_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let incident_types = req.incident_types.map(|types| {
            let type_strings: Vec<String> = types.iter().map(|t| t.as_str().to_string()).collect();
            serde_json::json!(type_strings)
        });

        let severity_levels = req.severity_levels.map(|levels| {
            let level_strings: Vec<String> =
                levels.iter().map(|s| s.as_str().to_string()).collect();
            serde_json::json!(level_strings)
        });

        let steps = req.steps.map(|s| serde_json::json!(s));
        let tags = req.tags.map(|t| serde_json::json!(t));

        sqlx::query(
            r#"
            INSERT INTO runbooks (
                id, title, description, incident_types, severity_levels, team_id, is_global,
                content, steps, automation_script, estimated_duration_minutes,
                success_rate, usage_count, created_by, created_at, updated_at, is_active, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&runbook_id)
        .bind(&req.title)
        .bind(&req.description)
        .bind(incident_types)
        .bind(severity_levels)
        .bind(&req.team_id)
        .bind(req.is_global)
        .bind(&req.content)
        .bind(steps)
        .bind(&req.automation_script)
        .bind(req.estimated_duration_minutes)
        .bind(1.0f64) // 初始成功率100%
        .bind(0i32) // 初始使用次数0
        .bind(user_id)
        .bind(now)
        .bind(now)
        .bind(true)
        .bind(tags)
        .execute(self.db.pool())
        .await?;

        info!("Created runbook {}: {}", runbook_id, req.title);

        self.get_runbook_by_id(&runbook_id).await
    }

    /// 获取运行手册详情
    pub async fn get_runbook_by_id(&self, runbook_id: &str) -> Result<Runbook> {
        let runbook = sqlx::query_as::<_, Runbook>("SELECT * FROM runbooks WHERE id = ?")
            .bind(runbook_id)
            .fetch_optional(self.db.pool())
            .await?;

        runbook.ok_or_else(|| anyhow!("Runbook not found: {}", runbook_id))
    }

    /// 更新运行手册
    pub async fn update_runbook(
        &self,
        runbook_id: &str,
        title: Option<&str>,
        description: Option<&str>,
        content: Option<&str>,
        steps: Option<Vec<RunbookStep>>,
        is_active: Option<bool>,
    ) -> Result<Runbook> {
        let now = Utc::now();

        let current = self.get_runbook_by_id(runbook_id).await?;

        sqlx::query(
            r#"
            UPDATE runbooks SET
                title = ?,
                description = ?,
                content = ?,
                steps = ?,
                updated_at = ?,
                is_active = ?
            WHERE id = ?
        "#,
        )
        .bind(title.unwrap_or(&current.title))
        .bind(description.unwrap_or(&current.description))
        .bind(content.unwrap_or(&current.content))
        .bind(steps.map(|s| serde_json::json!(s)))
        .bind(now)
        .bind(is_active.unwrap_or(current.is_active))
        .bind(runbook_id)
        .execute(self.db.pool())
        .await?;

        info!("Updated runbook {}", runbook_id);

        self.get_runbook_by_id(runbook_id).await
    }

    /// 查询运行手册列表
    pub async fn list_runbooks(
        &self,
        team_id: &str,
        incident_type: Option<&IncidentType>,
        is_global: Option<bool>,
    ) -> Result<Vec<Runbook>> {
        let mut query = r#"
            SELECT * FROM runbooks
            WHERE (team_id = ? OR is_global = TRUE)
            AND is_active = TRUE
        "#
        .to_string();

        // let mut params: Vec<Box<dyn sqlx::Type<sqlx::Any> + Send + Sync>> = vec![];

        if incident_type.is_some() {
            query.push_str(" AND incident_types LIKE ?");
        }

        if let Some(global) = is_global {
            query.push_str(" AND is_global = ?");
        }

        query.push_str(" ORDER BY usage_count DESC, created_at DESC");

        let runbooks = sqlx::query_as::<_, Runbook>(&query)
            .bind(team_id)
            .fetch_all(self.db.pool())
            .await?;

        Ok(runbooks)
    }

    /// 搜索运行手册
    pub async fn search_runbooks(&self, team_id: &str, query: &str) -> Result<Vec<Runbook>> {
        let search_pattern = format!("%{}%", query);

        let runbooks = sqlx::query_as::<_, Runbook>(
            r#"
            SELECT * FROM runbooks
            WHERE (team_id = ? OR is_global = TRUE)
            AND is_active = TRUE
            AND (title LIKE ? OR description LIKE ? OR content LIKE ? OR tags LIKE ?)
            ORDER BY usage_count DESC
        "#,
        )
        .bind(team_id)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(self.db.pool())
        .await?;

        Ok(runbooks)
    }

    /// 删除运行手册
    pub async fn delete_runbook(&self, runbook_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM runbooks WHERE id = ?")
            .bind(runbook_id)
            .execute(self.db.pool())
            .await?;

        info!("Deleted runbook {}", runbook_id);
        Ok(())
    }

    // ============= 运行手册执行 =============

    /// 执行运行手册
    pub async fn execute_runbook(
        &self,
        runbook_id: &str,
        incident_id: &str,
        executed_by: &str,
    ) -> Result<RunbookExecution> {
        let runbook = self.get_runbook_by_id(runbook_id).await?;

        let execution_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        // 解析步骤
        let steps: Vec<RunbookStep> = if let Some(steps_json) = &runbook.steps {
            serde_json::from_value(steps_json.clone()).unwrap_or_default()
        } else {
            vec![]
        };

        let total_steps = steps.len() as i32;

        sqlx::query(
            r#"
            INSERT INTO runbook_executions (
                id, runbook_id, incident_id, executed_by, status, started_at,
                current_step, total_steps, results, output_log
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&execution_id)
        .bind(runbook_id)
        .bind(incident_id)
        .bind(executed_by)
        .bind("running")
        .bind(now)
        .bind(0i32)
        .bind(total_steps)
        .bind(serde_json::json!([]))
        .bind("")
        .execute(self.db.pool())
        .await?;

        // 更新使用次数
        sqlx::query("UPDATE runbooks SET usage_count = usage_count + 1 WHERE id = ?")
            .bind(runbook_id)
            .execute(self.db.pool())
            .await?;

        info!(
            "Started runbook execution {} for incident {}",
            execution_id, incident_id
        );

        // 如果有自动化脚本，启动异步执行
        if runbook.automation_script.is_some() {
            // 在实际实现中，这里会启动一个后台任务来执行脚本
            info!(
                "Runbook {} has automation script, will execute steps",
                runbook_id
            );
        }

        self.get_execution_by_id(&execution_id).await
    }

    /// 执行单个步骤
    pub async fn execute_step(
        &self,
        execution_id: &str,
        step_number: i32,
        result: &str,
        output: Option<&str>,
    ) -> Result<RunbookExecution> {
        let execution = self.get_execution_by_id(execution_id).await?;

        // 更新步骤结果
        let mut results: Vec<serde_json::Value> = if let Some(r) = &execution.results {
            serde_json::from_value(r.clone()).unwrap_or_default()
        } else {
            vec![]
        };

        results.push(serde_json::json!({
            "step": step_number,
            "result": result,
            "output": output,
            "executed_at": Utc::now().to_rfc3339(),
        }));

        let new_step = step_number + 1;
        let status = if new_step >= execution.total_steps {
            "completed"
        } else {
            "running"
        };

        sqlx::query(
            r#"
            UPDATE runbook_executions
            SET current_step = ?, results = ?, status = ?, output_log = ?
            WHERE id = ?
        "#,
        )
        .bind(new_step)
        .bind(serde_json::json!(results))
        .bind(status)
        .bind(output.unwrap_or(""))
        .bind(execution_id)
        .execute(self.db.pool())
        .await?;

        debug!(
            "Executed step {} of runbook execution {}",
            step_number, execution_id
        );

        self.get_execution_by_id(execution_id).await
    }

    /// 完成运行手册执行
    pub async fn complete_execution(
        &self,
        execution_id: &str,
        success: bool,
    ) -> Result<RunbookExecution> {
        let now = Utc::now();

        let status = if success { "completed" } else { "failed" };

        sqlx::query(
            r#"
            UPDATE runbook_executions
            SET status = ?, completed_at = ?
            WHERE id = ?
        "#,
        )
        .bind(status)
        .bind(now)
        .bind(execution_id)
        .execute(self.db.pool())
        .await?;

        // 更新运行手册成功率统计
        let execution = self.get_execution_by_id(execution_id).await?;
        self.update_runbook_success_rate(&execution.runbook_id)
            .await?;

        info!(
            "Completed runbook execution {} with status {}",
            execution_id, status
        );

        self.get_execution_by_id(execution_id).await
    }

    /// 更新运行手册成功率
    async fn update_runbook_success_rate(&self, runbook_id: &str) -> Result<()> {
        let stats: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as success
            FROM runbook_executions
            WHERE runbook_id = ?
        "#,
        )
        .bind(runbook_id)
        .fetch_one(self.db.pool())
        .await?;

        let total = stats.0 as f64;
        let success = stats.1 as f64;

        if total > 0.0 {
            let success_rate = success / total;

            sqlx::query("UPDATE runbooks SET success_rate = ? WHERE id = ?")
                .bind(success_rate)
                .bind(runbook_id)
                .execute(self.db.pool())
                .await?;
        }

        Ok(())
    }

    /// 获取执行详情
    pub async fn get_execution_by_id(&self, execution_id: &str) -> Result<RunbookExecution> {
        let execution =
            sqlx::query_as::<_, RunbookExecution>("SELECT * FROM runbook_executions WHERE id = ?")
                .bind(execution_id)
                .fetch_optional(self.db.pool())
                .await?;

        execution.ok_or_else(|| anyhow!("Runbook execution not found: {}", execution_id))
    }

    /// 获取运行手册的执行历史
    pub async fn get_runbook_executions(&self, runbook_id: &str) -> Result<Vec<RunbookExecution>> {
        let executions = sqlx::query_as::<_, RunbookExecution>(
            r#"
            SELECT * FROM runbook_executions
            WHERE runbook_id = ?
            ORDER BY started_at DESC
        "#,
        )
        .bind(runbook_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(executions)
    }

    /// 获取事件的运行手册执行记录
    pub async fn get_incident_executions(
        &self,
        incident_id: &str,
    ) -> Result<Vec<RunbookExecution>> {
        let executions = sqlx::query_as::<_, RunbookExecution>(
            r#"
            SELECT * FROM runbook_executions
            WHERE incident_id = ?
            ORDER BY started_at DESC
        "#,
        )
        .bind(incident_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(executions)
    }

    // ============= 智能推荐 =============

    /// 为事件推荐运行手册
    pub async fn suggest_runbooks_for_incident(&self, incident: &Incident) -> Result<Vec<Runbook>> {
        // 1. 基于事件类型匹配
        let by_type = self
            .list_runbooks(&incident.team_id, Some(&incident.incident_type), None)
            .await?;

        // 2. 基于严重程度匹配
        let by_severity: Vec<Runbook> = sqlx::query_as::<_, Runbook>(
            r#"
            SELECT * FROM runbooks
            WHERE (team_id = ? OR is_global = TRUE)
            AND is_active = TRUE
            AND severity_levels LIKE ?
            ORDER BY success_rate DESC, usage_count DESC
        "#,
        )
        .bind(&incident.team_id)
        .bind(format!("%{}%", incident.severity.as_str()))
        .fetch_all(self.db.pool())
        .await?;

        // 3. 合并结果并去重
        let mut all_runbooks = by_type;
        for rb in by_severity {
            if !all_runbooks.iter().any(|r| r.id == rb.id) {
                all_runbooks.push(rb);
            }
        }

        // 4. 按成功率排序
        all_runbooks.sort_by(|a, b| {
            let a_rate = a.success_rate.unwrap_or(0.0);
            let b_rate = b.success_rate.unwrap_or(0.0);
            b_rate
                .partial_cmp(&a_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 返回前10个
        Ok(all_runbooks.into_iter().take(10).collect())
    }

    /// 获取热门运行手册
    pub async fn get_popular_runbooks(&self, team_id: &str, limit: i64) -> Result<Vec<Runbook>> {
        let runbooks = sqlx::query_as::<_, Runbook>(
            r#"
            SELECT * FROM runbooks
            WHERE (team_id = ? OR is_global = TRUE)
            AND is_active = TRUE
            ORDER BY usage_count DESC, success_rate DESC
            LIMIT ?
        "#,
        )
        .bind(team_id)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;

        Ok(runbooks)
    }

    /// 初始化默认运行手册
    pub async fn seed_default_runbooks(&self, team_id: &str, admin_user_id: &str) -> Result<()> {
        let default_runbooks = vec![
            (
                "服务器宕机处理手册",
                "处理服务器无法访问或宕机的标准流程",
                IncidentType::ServerDown,
                IncidentSeverity::Critical,
                r#"## 服务器宕机处理流程

### 1. 初步检查
- [ ] 检查服务器网络连通性 (ping)
- [ ] 尝试通过IPMI/iLO/iDRAC远程管理卡访问
- [ ] 检查机房状态指示灯

### 2. 诊断步骤
```bash
# 如果能远程访问，收集日志
journalctl -xe --since "1 hour ago"
dmesg | tail -100
cat /var/log/messages | tail -100
```

### 3. 恢复措施
- 如果是软件问题：尝试重启服务
- 如果是硬件问题：联系硬件供应商
- 如果需要重启：安排维护窗口

### 4. 验证恢复
- [ ] 确认服务已恢复
- [ ] 检查监控指标
- [ ] 通知相关团队"#,
            ),
            (
                "CPU使用率过高处理手册",
                "处理CPU使用率异常的标准流程",
                IncidentType::HighCpu,
                IncidentSeverity::High,
                r#"## CPU使用率过高处理流程

### 1. 识别高CPU进程
```bash
# 查看CPU使用率最高的进程
top -bn1 | head -20
ps aux --sort=-%cpu | head -20
```

### 2. 分析进程
- 识别是正常负载还是异常进程
- 检查是否为恶意进程
- 查看进程日志

### 3. 缓解措施
```bash
# 如果是失控进程，可以限制CPU使用
cpulimit -p <pid> -l 50

# 或者调整进程优先级
renice +10 <pid>
```

### 4. 根因分析
- 检查最近的代码部署
- 查看数据库查询性能
- 检查外部API调用"#,
            ),
            (
                "磁盘空间不足处理手册",
                "处理磁盘空间不足的标准流程",
                IncidentType::DiskFull,
                IncidentSeverity::High,
                r#"## 磁盘空间不足处理流程

### 1. 快速诊断
```bash
# 查看磁盘使用情况
df -h

# 查看目录占用
du -sh /* 2>/dev/null | sort -hr | head -20
```

### 2. 紧急清理
```bash
# 清理日志文件
find /var/log -name "*.log" -mtime +7 -delete

# 清理临时文件
rm -rf /tmp/*
rm -rf /var/tmp/*

# 清理包缓存
# CentOS/RHEL
yum clean all
# Ubuntu/Debian
apt-get clean
```

### 3. 长期措施
- 设置日志轮转
- 监控磁盘使用率
- 考虑扩容"#,
            ),
            (
                "服务不可用处理手册",
                "处理应用程序服务不可用的标准流程",
                IncidentType::ServiceUnavailable,
                IncidentSeverity::High,
                r#"## 服务不可用处理流程

### 1. 快速检查
```bash
# 检查服务状态
systemctl status <service-name>

# 检查端口监听
netstat -tlnp | grep <port>
ss -tlnp | grep <port>
```

### 2. 查看日志
```bash
# 应用日志
tail -f /var/log/<app>/*.log

# 系统日志
journalctl -u <service-name> -f
```

### 3. 恢复服务
```bash
# 重启服务
systemctl restart <service-name>

# 如果是容器
docker restart <container-id>
```

### 4. 健康检查
```bash
# HTTP健康检查
curl -f http://localhost:8080/health

# TCP检查
telnet localhost <port>
```"#,
            ),
        ];

        for (title, desc, incident_type, severity, content) in default_runbooks {
            let req = CreateRunbookRequest {
                title: title.to_string(),
                description: desc.to_string(),
                incident_types: Some(vec![incident_type]),
                severity_levels: Some(vec![severity]),
                team_id: team_id.to_string(),
                is_global: true,
                content: content.to_string(),
                steps: None,
                automation_script: None,
                estimated_duration_minutes: Some(30),
                tags: Some(vec!["default".to_string(), "auto-created".to_string()]),
            };

            match self.create_runbook(req, admin_user_id).await {
                Ok(rb) => info!("Created default runbook: {}", rb.title),
                Err(e) => warn!("Failed to create default runbook '{}': {}", title, e),
            }
        }

        Ok(())
    }
}

// ============= 扩展trait实现 =============

impl RunbookExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunbookExecutionStatus::Pending => "pending",
            RunbookExecutionStatus::Running => "running",
            RunbookExecutionStatus::Paused => "paused",
            RunbookExecutionStatus::Completed => "completed",
            RunbookExecutionStatus::Failed => "failed",
            RunbookExecutionStatus::Cancelled => "cancelled",
        }
    }
}
