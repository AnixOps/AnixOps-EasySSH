//! EasySSH Standard 版本监控模块使用示例
//!
//! 这个示例展示了如何使用监控模块的核心功能。

use easyssh_core::monitoring::{
    AlertCondition, AlertRule, AlertSeverity, AuthMethod, ChartData, MetricStats, MetricType,
    MonitoringConfig, MonitoringManager, MonitoringSession, ServerConnectionConfig,
    ServerHealthStatus, SystemMetrics, TimeRange,
};
use std::sync::Arc;

/// 示例 1: 创建并配置监控会话
/// 适用于 Standard 版本的单服务器监控
async fn example_monitoring_session() -> Result<(), Box<dyn std::error::Error>> {
    // 创建监控会话
    let mut session = MonitoringSession::new("server-001".to_string());

    // 配置连接信息
    session
        .configure_connection(
            "192.168.1.100".to_string(),
            22,
            "admin".to_string(),
            AuthMethod::PrivateKey {
                key_path: "~/.ssh/id_rsa".to_string(),
                passphrase: None,
            },
        )
        .await;

    // 连接到服务器
    session.connect().await?;

    // 启动监控
    session.start().await?;

    // 获取最新指标
    if let Some(metrics) = session.get_latest_metrics().await {
        println!("CPU: {:.1}%", metrics.cpu_percent);
        println!("Memory: {:.1}%", metrics.memory_percent());
        println!("Disk: {:.1}%", metrics.disk_percent());
        println!("Load: {:?}", metrics.load_avg);
    }

    // 获取历史数据（最近 60 个点）
    let history = session.get_history(Some(60)).await;
    println!("History points: {}", history.len());

    // 停止监控
    session.stop().await;
    session.disconnect().await;

    Ok(())
}

/// 示例 2: 使用图表数据生成
fn example_chart_data() {
    // 创建模拟的历史数据
    let history = vec![
        SystemMetrics::new(
            10.0,
            4 * 1024 * 1024 * 1024,
            16 * 1024 * 1024 * 1024,
            100,
            500,
            0,
            0,
            [0.1, 0.1, 0.1],
        ),
        SystemMetrics::new(
            20.0,
            5 * 1024 * 1024 * 1024,
            16 * 1024 * 1024 * 1024,
            110,
            500,
            1024,
            512,
            [0.2, 0.2, 0.2],
        ),
        SystemMetrics::new(
            30.0,
            6 * 1024 * 1024 * 1024,
            16 * 1024 * 1024 * 1024,
            120,
            500,
            2048,
            1024,
            [0.3, 0.3, 0.3],
        ),
        SystemMetrics::new(
            25.0,
            5 * 1024 * 1024 * 1024,
            16 * 1024 * 1024 * 1024,
            115,
            500,
            1536,
            768,
            [0.25, 0.25, 0.25],
        ),
    ];

    // 生成 sparkline 数据（紧凑折线图）
    let cpu_sparkline = ChartData::generate_sparkline(&history, "cpu");
    println!("CPU Sparkline: {:?}", cpu_sparkline);

    // 生成时间序列数据
    let cpu_timeseries = ChartData::generate_timeseries(&history, "cpu");
    println!("CPU Time Series: {:?}", cpu_timeseries);

    // 计算统计信息
    let stats = ChartData::calculate_stats(&history, "cpu");
    println!(
        "CPU Stats: min={:.1}%, max={:.1}%, avg={:.1}%",
        stats.min, stats.max, stats.avg
    );

    // 生成资源对比数据
    let comparison = ChartData::generate_resource_comparison(&history);
    for (resource, percentage, color) in comparison {
        println!("{}: {:.1}% ({})", resource, percentage, color);
    }
}

/// 示例 3: 使用企业级监控管理器
async fn example_monitoring_manager() -> Result<(), Box<dyn std::error::Error>> {
    // 创建监控配置
    let config = MonitoringConfig {
        collection_interval_secs: 30,
        retention_days: 90,
        alert_check_interval_secs: 60,
        enable_predictive_alerts: true,
        enable_anomaly_detection: true,
        large_screen_refresh_secs: 5,
        default_dashboard_id: None,
    };

    // 创建监控管理器
    let manager = MonitoringManager::new(config).await?;

    // 添加服务器到监控
    let server_config = ServerConnectionConfig {
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "key".to_string(),
        password: None,
        private_key: Some("~/.ssh/id_rsa".to_string()),
        passphrase: None,
    };

    manager
        .add_server("server-001".to_string(), server_config)
        .await?;

    // 创建告警规则
    let alert_rule = AlertRule {
        id: "high-cpu".to_string(),
        name: "High CPU Usage".to_string(),
        description: Some("Alert when CPU usage exceeds 80%".to_string()),
        enabled: true,
        severity: AlertSeverity::Warning,
        metric_type: MetricType::CpuUsage,
        condition: AlertCondition::GreaterThan,
        threshold: 80.0,
        duration_secs: 300,
        cooldown_secs: 600,
        server_ids: vec!["server-001".to_string()],
        server_groups: vec![],
        tags: vec!["infrastructure".to_string()],
        notification_channels: vec!["email".to_string()],
        auto_resolve: true,
        resolve_after_secs: 600,
        runbook_url: None,
        dashboard_url: None,
        created_at: 0,
        updated_at: 0,
    };

    // 添加告警规则
    manager.alert_engine.upsert_rule(alert_rule).await?;

    // 启动监控
    manager.start().await?;

    // 获取健康摘要
    let summary = manager.get_health_summary().await?;
    println!("Total servers: {}", summary.total_servers);
    println!(
        "Healthy: {}, Warning: {}, Critical: {}",
        summary.healthy, summary.warning, summary.critical
    );

    // 获取实时指标
    let metrics = manager.get_realtime_metrics("server-001").await?;
    println!("CPU: {:.1}%", metrics.cpu_usage);

    // 获取历史数据
    let history = manager
        .get_historical_metrics("server-001", MetricType::CpuUsage, TimeRange::Last1Hour)
        .await?;
    println!("History points in last hour: {}", history.len());

    // 停止监控
    manager.stop().await?;

    Ok(())
}

/// 示例 4: 配置告警规则
fn example_alert_rules() -> Vec<AlertRule> {
    vec![
        // CPU 告警
        AlertRule {
            id: "cpu-critical".to_string(),
            name: "Critical CPU Usage".to_string(),
            description: Some("CPU usage above 90% for 5 minutes".to_string()),
            enabled: true,
            severity: AlertSeverity::Critical,
            metric_type: MetricType::CpuUsage,
            condition: AlertCondition::GreaterThan,
            threshold: 90.0,
            duration_secs: 300,
            cooldown_secs: 600,
            server_ids: vec![],
            server_groups: vec!["production".to_string()],
            tags: vec!["cpu".to_string(), "critical".to_string()],
            notification_channels: vec!["slack".to_string(), "pagerduty".to_string()],
            auto_resolve: true,
            resolve_after_secs: 600,
            runbook_url: Some("https://wiki.example.com/cpu-alerts".to_string()),
            dashboard_url: Some("https://grafana.example.com/d/cpu".to_string()),
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        },
        // 内存告警
        AlertRule {
            id: "memory-warning".to_string(),
            name: "High Memory Usage".to_string(),
            description: Some("Memory usage above 80%".to_string()),
            enabled: true,
            severity: AlertSeverity::Warning,
            metric_type: MetricType::MemoryUsage,
            condition: AlertCondition::GreaterThan,
            threshold: 80.0,
            duration_secs: 180,
            cooldown_secs: 300,
            server_ids: vec![],
            server_groups: vec![],
            tags: vec!["memory".to_string()],
            notification_channels: vec!["email".to_string()],
            auto_resolve: true,
            resolve_after_secs: 300,
            runbook_url: None,
            dashboard_url: None,
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        },
        // 磁盘告警
        AlertRule {
            id: "disk-warning".to_string(),
            name: "Disk Space Low".to_string(),
            description: Some("Disk usage above 85%".to_string()),
            enabled: true,
            severity: AlertSeverity::Warning,
            metric_type: MetricType::DiskUsage,
            condition: AlertCondition::GreaterThan,
            threshold: 85.0,
            duration_secs: 60,
            cooldown_secs: 3600,
            server_ids: vec![],
            server_groups: vec![],
            tags: vec!["disk".to_string()],
            notification_channels: vec!["email".to_string(), "slack".to_string()],
            auto_resolve: true,
            resolve_after_secs: 3600,
            runbook_url: None,
            dashboard_url: None,
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        },
        // 负载告警
        AlertRule {
            id: "load-warning".to_string(),
            name: "High Load Average".to_string(),
            description: Some("Load average above 2.0 for 10 minutes".to_string()),
            enabled: true,
            severity: AlertSeverity::Warning,
            metric_type: MetricType::CpuLoad1,
            condition: AlertCondition::GreaterThan,
            threshold: 2.0,
            duration_secs: 600,
            cooldown_secs: 600,
            server_ids: vec![],
            server_groups: vec![],
            tags: vec!["load".to_string()],
            notification_channels: vec!["slack".to_string()],
            auto_resolve: true,
            resolve_after_secs: 600,
            runbook_url: None,
            dashboard_url: None,
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        },
    ]
}

/// 示例 5: 监控指标数据结构
fn example_system_metrics() {
    // 创建新的系统指标
    let metrics = SystemMetrics::new(
        45.5,                     // CPU 使用率
        4 * 1024 * 1024 * 1024,   // 内存使用: 4GB
        16 * 1024 * 1024 * 1024,  // 总内存: 16GB
        100 * 1024 * 1024 * 1024, // 磁盘使用: 100GB
        500 * 1024 * 1024 * 1024, // 总磁盘: 500GB
        1024 * 1024,              // 网络接收: 1MB
        512 * 1024,               // 网络发送: 512KB
        [0.5, 0.3, 0.2],          // 负载平均值 (1min, 5min, 15min)
    );

    // 使用指标
    println!("CPU: {:.1}%", metrics.cpu_percent);
    println!("Memory: {:.1}%", metrics.memory_percent());
    println!("Disk: {:.1}%", metrics.disk_percent());
    println!("Load: {:?}", metrics.load_avg);

    // 检查健康状态
    match metrics.overall_health() {
        ServerHealthStatus::Healthy => println!("System is healthy"),
        ServerHealthStatus::Warning => println!("System has warnings"),
        ServerHealthStatus::Critical => println!("System is critical!"),
        _ => println!("Unknown status"),
    }

    // 转换为完整的服务器指标格式
    let server_metrics = metrics.to_server_metrics("server-001");
    println!("Server health: {:?}", server_metrics.health_status());
}

/// 运行所有示例
#[tokio::main]
async fn main() {
    println!("=== EasySSH 监控模块示例 ===\n");

    // 示例 2: 图表数据（同步）
    println!("示例 2: 图表数据生成");
    example_chart_data();
    println!();

    // 示例 5: 系统指标（同步）
    println!("示例 5: 系统指标数据结构");
    example_system_metrics();
    println!();

    // 示例 4: 告警规则（同步）
    println!("示例 4: 告警规则配置");
    let rules = example_alert_rules();
    for rule in &rules {
        println!(
            "  - {}: {} [{}]",
            rule.id,
            rule.name,
            format!("{:?}", rule.severity)
        );
    }
    println!();

    // 异步示例（需要实际服务器连接）
    println!("示例 1 & 3: 监控会话和管理器（需要实际服务器）");
    println!("  注意: 这些示例需要实际的 SSH 服务器连接");
    println!("  在生产环境中使用这些代码前，请确保:");
    println!("    1. 配置了正确的服务器地址和认证信息");
    println!("    2. 服务器支持 SSH 连接");
    println!("    3. 有读取 /proc 文件的权限");

    println!("\n=== 示例运行完成 ===");
}
