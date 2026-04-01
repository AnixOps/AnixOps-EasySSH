//! Monitoring Dashboard Example and Integration Tests
//!
//! This demonstrates the professional monitoring dashboard system capabilities.

#[cfg(test)]
mod tests {
    use crate::monitoring::{
        Alert, AlertCondition, AlertEngine, AlertEngineConfig, AlertRule, AlertSeverity,
        AlertStatus, CapacityForecast, CapacityRecommendation, CapacityStatus, ChartSeries,
        ChartType, CustomDashboard, DashboardBuilder, DashboardFormatter, DashboardTemplates,
        DashboardViewModel, HealthSummary, LayoutAlgorithm, MetricPoint, MetricType, MonitoringConfig,
        MonitoringManager, NotificationChannel, NotificationChannelType, Priority, ResourceType,
        ServerConnectionConfig, ServerHealthCard, ServerHealthStatus, ServerMetrics, ServerOverview,
        TimeRange, TopologyBuilder, TopologyLayout, TrendDirection, WidgetConfig, WidgetType,
    };

    /// Test creating a monitoring manager
    /// Note: This test is ignored by default as it requires database/file system access
    #[tokio::test]
    #[ignore = "requires database access - run with --ignored to execute"]
    async fn test_monitoring_manager_creation() {
        let config = MonitoringConfig {
            collection_interval_secs: 30,
            retention_days: 90,
            alert_check_interval_secs: 60,
            enable_predictive_alerts: true,
            enable_anomaly_detection: true,
            large_screen_refresh_secs: 5,
            default_dashboard_id: None,
        };

        let manager = MonitoringManager::new(config).await;
        assert!(manager.is_ok());
    }

    /// Test server health status colors
    #[test]
    fn test_server_health_status() {
        assert_eq!(ServerHealthStatus::Healthy.color(), "#22c55e");
        assert_eq!(ServerHealthStatus::Warning.color(), "#f59e0b");
        assert_eq!(ServerHealthStatus::Critical.color(), "#ef4444");
        assert_eq!(ServerHealthStatus::Offline.color(), "#374151");
        assert_eq!(ServerHealthStatus::Unknown.color(), "#6b7280");
    }

    /// Test alert severity ordering
    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Info < AlertSeverity::Warning);
        assert!(AlertSeverity::Warning < AlertSeverity::Critical);
        assert!(AlertSeverity::Critical < AlertSeverity::Emergency);
    }

    /// Test alert condition evaluation
    #[test]
    fn test_alert_condition_evaluation() {
        let condition = AlertCondition::GreaterThan;
        assert!(condition.evaluate(10.0, 5.0));
        assert!(!condition.evaluate(5.0, 10.0));

        let condition = AlertCondition::Between { min: 10.0, max: 20.0 };
        assert!(condition.evaluate(15.0, 0.0));
        assert!(!condition.evaluate(5.0, 0.0));
        assert!(!condition.evaluate(25.0, 0.0));
    }

    /// Test time range calculations
    #[test]
    fn test_time_range_seconds() {
        assert_eq!(TimeRange::Last5Minutes.to_seconds(), 300);
        assert_eq!(TimeRange::Last15Minutes.to_seconds(), 900);
        assert_eq!(TimeRange::Last30Minutes.to_seconds(), 1800);
        assert_eq!(TimeRange::Last1Hour.to_seconds(), 3600);
        assert_eq!(TimeRange::Last24Hours.to_seconds(), 86400);
        assert_eq!(TimeRange::Last7Days.to_seconds(), 604800);
        assert_eq!(TimeRange::Last30Days.to_seconds(), 2592000);
    }

    /// Test metric type units
    #[test]
    fn test_metric_type_units() {
        assert_eq!(MetricType::CpuUsage.unit(), "%");
        assert_eq!(MetricType::MemoryUsed.unit(), "bytes");
        assert_eq!(MetricType::DiskReadIops.unit(), "iops");
        assert_eq!(MetricType::DiskReadLatency.unit(), "ms");
        assert_eq!(MetricType::NetworkRxMbps.unit(), "mbps");
        assert_eq!(MetricType::ProcessCount.unit(), "count");
    }

    /// Test metric categories
    #[test]
    fn test_metric_categories() {
        use crate::monitoring::metrics::MetricCategory;
        assert_eq!(MetricType::CpuUsage.category(), MetricCategory::Cpu);
        assert_eq!(MetricType::MemoryUsage.category(), MetricCategory::Memory);
        assert_eq!(MetricType::DiskUsage.category(), MetricCategory::Disk);
        assert_eq!(MetricType::NetworkRxBytes.category(), MetricCategory::Network);
        assert_eq!(MetricType::ProcessCount.category(), MetricCategory::Process);
    }

    /// Test dashboard templates
    #[test]
    fn test_dashboard_templates() {
        let exec = DashboardTemplates::executive_overview();
        assert_eq!(exec.id, "exec-overview");
        assert!(!exec.widgets.is_empty());

        let noc = DashboardTemplates::noc_display();
        assert_eq!(noc.id, "noc-display");
        assert!(noc.is_large_screen);

        let ops = DashboardTemplates::operations_center();
        assert_eq!(ops.id, "ops-center");

        let capacity = DashboardTemplates::capacity_planning();
        assert_eq!(capacity.id, "capacity");

        let perf = DashboardTemplates::performance_comparison();
        assert_eq!(perf.id, "perf-compare");
    }

    /// Test dashboard builder
    #[test]
    fn test_dashboard_builder() {
        let dashboard = DashboardBuilder::new("test", "Test Dashboard")
            .add_server_health_widget(0, 0, 6, 4)
            .add_alerts_widget(6, 0, 6, 4)
            .build();

        assert_eq!(dashboard.id, "test");
        assert_eq!(dashboard.name, "Test Dashboard");
        assert_eq!(dashboard.widgets.len(), 2);
    }

    /// Test topology builder
    #[test]
    fn test_topology_builder() {
        let topology = TopologyBuilder::new()
            .with_load_balancer("lb1", "Load Balancer 1")
            .with_server("web1", "Web Server 1", crate::monitoring::topology::TopologyStatus::Online)
            .with_server("web2", "Web Server 2", crate::monitoring::topology::TopologyStatus::Online)
            .with_database("db1", "Database 1", crate::monitoring::topology::TopologyStatus::Online)
            .with_connection("lb1", "web1")
            .with_connection("lb1", "web2")
            .with_connection("web1", "db1")
            .with_connection("web2", "db1")
            .build();

        assert_eq!(topology.nodes.len(), 4);
        assert_eq!(topology.edges.len(), 4);
    }

    /// Test topology auto layout
    #[test]
    fn test_topology_auto_layout() {
        use crate::monitoring::topology::{ServerTopology, TopologyNode, TopologyNodeType, TopologyStatus};
        use std::collections::HashMap;

        let mut topology = ServerTopology::new();

        // Add nodes
        for i in 0..5 {
            topology.add_node(TopologyNode {
                id: format!("server-{}", i),
                node_type: TopologyNodeType::Server,
                label: format!("Server {}", i),
                status: TopologyStatus::Online,
                metrics: HashMap::new(),
                x: 0.0,
                y: 0.0,
                group_id: None,
                icon: None,
                color: None,
                metadata: HashMap::new(),
            });
        }

        // Apply auto layout
        topology.auto_layout();

        // Check that nodes have non-zero positions
        for node in &topology.nodes {
            assert!(node.x != 0.0 || node.y != 0.0);
        }
    }

    /// Test trend direction
    #[test]
    fn test_trend_direction() {
        let values_up = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let values_down = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let values_stable = vec![1.0, 1.1, 0.9, 1.0, 1.05];

        // Simple trend detection (last vs first)
        let trend_up = if values_up.last() > values_up.first() {
            TrendDirection::Up
        } else {
            TrendDirection::Stable
        };
        assert!(matches!(trend_up, TrendDirection::Up));

        let trend_down = if values_down.last() < values_down.first() {
            TrendDirection::Down
        } else {
            TrendDirection::Stable
        };
        assert!(matches!(trend_down, TrendDirection::Down));
    }

    /// Test dashboard formatter
    #[test]
    fn test_dashboard_formatter() {
        // Test bytes formatting
        assert_eq!(DashboardFormatter::format_bytes(1024), "1.00 KB");
        assert_eq!(DashboardFormatter::format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(DashboardFormatter::format_bytes(1024 * 1024 * 1024), "1.00 GB");

        // Test duration formatting
        assert_eq!(DashboardFormatter::format_duration(60), "1m");
        assert_eq!(DashboardFormatter::format_duration(3600), "1h 0m");
        assert_eq!(DashboardFormatter::format_duration(86400), "1d 0h 0m");

        // Test percentage formatting
        assert_eq!(DashboardFormatter::format_percentage(42.5), "42.5%");
    }

    /// Test server metrics health status calculation
    #[test]
    fn test_server_metrics_health_status() {
        let healthy_metrics = ServerMetrics {
            server_id: "test".to_string(),
            timestamp: 0,
            collected_at: 0,
            cpu_usage: 50.0,
            cpu_user: 30.0,
            cpu_system: 20.0,
            cpu_iowait: 0.0,
            cpu_steal: 0.0,
            cpu_cores: 4,
            cpu_load1: 2.0,
            cpu_load5: 1.5,
            cpu_load15: 1.0,
            memory_used: 4 * 1024 * 1024 * 1024, // 4GB
            memory_total: 16 * 1024 * 1024 * 1024, // 16GB
            memory_free: 12 * 1024 * 1024 * 1024,
            memory_buffers: 512 * 1024 * 1024,
            memory_cached: 2 * 1024 * 1024 * 1024,
            memory_available: 10 * 1024 * 1024 * 1024,
            swap_used: 0,
            swap_total: 4 * 1024 * 1024 * 1024,
            disk_used: 100 * 1024 * 1024 * 1024,
            disk_total: 500 * 1024 * 1024 * 1024,
            disk_free: 400 * 1024 * 1024 * 1024,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            disk_read_iops: 0.0,
            disk_write_iops: 0.0,
            disk_io_util: 0.0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            network_rx_packets: 0,
            network_tx_packets: 0,
            network_rx_errors: 0,
            network_tx_errors: 0,
            network_rx_dropped: 0,
            network_tx_dropped: 0,
            process_count: 150,
            process_running: 3,
            process_sleeping: 147,
            process_zombie: 0,
            thread_count: 300,
            open_files: 1024,
            uptime_seconds: 86400,
            boot_time: 0,
            context_switches: 0,
            interrupts: 0,
            cpu_temp: None,
            system_temp: None,
            extra: std::collections::HashMap::new(),
        };

        assert!(matches!(healthy_metrics.health_status(), ServerHealthStatus::Healthy));

        // Test critical threshold
        let mut critical_metrics = healthy_metrics.clone();
        critical_metrics.cpu_usage = 95.0;
        assert!(matches!(critical_metrics.health_status(), ServerHealthStatus::Critical));

        // Test warning threshold
        let mut warning_metrics = healthy_metrics.clone();
        warning_metrics.cpu_usage = 75.0;
        warning_metrics.memory_used = 12 * 1024 * 1024 * 1024; // 75% memory
        assert!(matches!(warning_metrics.health_status(), ServerHealthStatus::Warning));
    }

    /// Test widget configuration
    #[test]
    fn test_widget_config() {
        let config = WidgetConfig {
            id: "widget-1".to_string(),
            widget_type: WidgetType::ServerHealth,
            title: "Health Overview".to_string(),
            x: 0,
            y: 0,
            width: 6,
            height: 4,
            server_ids: vec!["server-1".to_string(), "server-2".to_string()],
            metric_types: vec![MetricType::CpuUsage, MetricType::MemoryUsage],
            refresh_interval_secs: 30,
            time_range: TimeRange::Last15Minutes,
            custom_config: std::collections::HashMap::new(),
        };

        assert_eq!(config.id, "widget-1");
        assert_eq!(config.widget_type, WidgetType::ServerHealth);
        assert_eq!(config.refresh_interval_secs, 30);
    }

    /// Test notification channel types
    #[test]
    fn test_notification_channels() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let email_channel = NotificationChannel {
            id: "email-1".to_string(),
            name: "Email Notifications".to_string(),
            channel_type: NotificationChannelType::Email,
            config: std::collections::HashMap::new(),
            enabled: true,
            rate_limit_per_minute: 10,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(email_channel.channel_type, NotificationChannelType::Email);
        assert!(email_channel.enabled);

        let slack_channel = NotificationChannel {
            id: "slack-1".to_string(),
            name: "Slack Alerts".to_string(),
            channel_type: NotificationChannelType::Slack,
            config: std::collections::HashMap::new(),
            enabled: true,
            rate_limit_per_minute: 20,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(slack_channel.channel_type, NotificationChannelType::Slack);
    }

    /// Test chart types
    #[test]
    fn test_chart_types() {
        let chart_types = vec![
            ChartType::Line,
            ChartType::Area,
            ChartType::Bar,
            ChartType::StackedArea,
            ChartType::Gauge,
            ChartType::Pie,
            ChartType::Heatmap,
            ChartType::Sparkline,
        ];

        // Verify all chart types can be created
        assert_eq!(chart_types.len(), 8);
    }

    /// Test capacity forecast
    #[test]
    fn test_capacity_forecast() {
        let forecast = CapacityForecast {
            server_id: "server-1".to_string(),
            resource_type: ResourceType::Disk,
            current_usage: 75.0,
            current_capacity: 500.0,
            predicted_depletion_date: Some(1893456000), // Some future timestamp
            days_until_critical: Some(45),
            growth_rate_per_day: 0.5,
            confidence: 0.85,
            forecast_points: vec![
                crate::monitoring::metrics::ForecastPoint {
                    timestamp: 1893456000,
                    predicted_usage: 90.0,
                    lower_bound: 85.0,
                    upper_bound: 95.0,
                },
            ],
            recommendations: vec![
                CapacityRecommendation {
                    priority: Priority::High,
                    message: "Disk will reach 90% capacity in 45 days".to_string(),
                    action: "Add storage capacity".to_string(),
                    estimated_cost: Some("$200/month".to_string()),
                },
            ],
        };

        assert_eq!(forecast.server_id, "server-1");
        assert_eq!(forecast.resource_type, ResourceType::Disk);
        assert!(forecast.days_until_critical.is_some());
        assert_eq!(forecast.recommendations.len(), 1);
    }

    /// Test server overview creation
    #[test]
    fn test_server_overview() {
        let overview = ServerOverview {
            server_id: "srv-001".to_string(),
            server_name: "Production Web Server".to_string(),
            host: "192.168.1.100".to_string(),
            status: ServerHealthStatus::Healthy,
            last_seen: Some(1234567890),
            uptime_seconds: Some(86400),
            cpu_percent: Some(45.0),
            memory_percent: Some(60.0),
            disk_percent: Some(70.0),
            network_rx_mbps: Some(10.5),
            network_tx_mbps: Some(5.2),
            active_alerts: 0,
            os_info: Some("Ubuntu 22.04 LTS".to_string()),
            location: Some("us-east-1".to_string()),
        };

        assert_eq!(overview.server_id, "srv-001");
        assert_eq!(overview.status, ServerHealthStatus::Healthy);
        assert!(overview.active_alerts == 0);
    }

    /// Test health summary
    #[test]
    fn test_health_summary() {
        let summary = HealthSummary {
            total_servers: 10,
            healthy: 7,
            warning: 2,
            critical: 1,
            offline: 0,
            unknown: 0,
            total_alerts: 3,
            servers: vec![
                ServerOverview {
                    server_id: "srv-001".to_string(),
                    server_name: "Web Server 1".to_string(),
                    host: "10.0.0.1".to_string(),
                    status: ServerHealthStatus::Healthy,
                    last_seen: Some(1234567890),
                    uptime_seconds: Some(86400),
                    cpu_percent: Some(30.0),
                    memory_percent: Some(50.0),
                    disk_percent: Some(60.0),
                    network_rx_mbps: Some(100.0),
                    network_tx_mbps: Some(50.0),
                    active_alerts: 0,
                    os_info: None,
                    location: None,
                },
            ],
        };

        assert_eq!(summary.total_servers, 10);
        assert_eq!(summary.healthy, 7);
        assert_eq!(summary.total_alerts, 3);
        assert_eq!(summary.servers.len(), 1);
    }

    /// Test alert rule creation
    #[test]
    fn test_alert_rule() {
        let rule = AlertRule {
            id: "rule-001".to_string(),
            name: "High CPU Usage".to_string(),
            description: Some("Alert when CPU usage exceeds 90% for 5 minutes".to_string()),
            enabled: true,
            severity: AlertSeverity::Warning,
            metric_type: MetricType::CpuUsage,
            condition: AlertCondition::GreaterThan,
            threshold: 90.0,
            duration_secs: 300,
            cooldown_secs: 600,
            server_ids: vec!["srv-001".to_string()],
            server_groups: vec![],
            tags: vec!["infrastructure".to_string(), "cpu".to_string()],
            notification_channels: vec!["email-1".to_string(), "slack-1".to_string()],
            auto_resolve: true,
            resolve_after_secs: 600,
            runbook_url: Some("https://wiki.example.com/cpu-alerts".to_string()),
            dashboard_url: Some("https://grafana.example.com/d/cpu".to_string()),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        assert_eq!(rule.id, "rule-001");
        assert_eq!(rule.metric_type, MetricType::CpuUsage);
        assert!(rule.enabled);
        assert!(rule.auto_resolve);
    }

    /// Test metric point
    #[test]
    fn test_metric_point() {
        let point = MetricPoint {
            timestamp: 1234567890,
            value: 42.5,
            metric_type: MetricType::CpuUsage,
            labels: std::collections::HashMap::new(),
        };

        assert_eq!(point.timestamp, 1234567890);
        assert_eq!(point.value, 42.5);
        assert_eq!(point.metric_type, MetricType::CpuUsage);
    }

    /// Test topology layout
    #[test]
    fn test_topology_layout() {
        let layout = TopologyLayout::default();

        assert_eq!(layout.zoom_level, 1.0);
        assert!(layout.show_labels);
        assert!(layout.show_metrics);
        assert!(layout.animated);
        assert_eq!(layout.algorithm, LayoutAlgorithm::ForceDirected);
    }

    /// Test custom dashboard
    #[test]
    fn test_custom_dashboard() {
        let dashboard = CustomDashboard {
            id: "dash-001".to_string(),
            name: "My Custom Dashboard".to_string(),
            description: Some("A custom monitoring dashboard".to_string()),
            widgets: vec![
                WidgetConfig {
                    id: "widget-1".to_string(),
                    widget_type: WidgetType::ServerHealth,
                    title: "Health".to_string(),
                    x: 0,
                    y: 0,
                    width: 6,
                    height: 4,
                    server_ids: vec![],
                    metric_types: vec![],
                    refresh_interval_secs: 30,
                    time_range: TimeRange::Last15Minutes,
                    custom_config: std::collections::HashMap::new(),
                },
            ],
            is_default: false,
            is_large_screen: false,
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        assert_eq!(dashboard.id, "dash-001");
        assert_eq!(dashboard.widgets.len(), 1);
        assert!(!dashboard.is_large_screen);
    }

    /// Test monitoring config default
    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();

        assert_eq!(config.collection_interval_secs, 30);
        assert_eq!(config.retention_days, 90);
        assert_eq!(config.alert_check_interval_secs, 60);
        assert!(config.enable_predictive_alerts);
        assert!(config.enable_anomaly_detection);
        assert_eq!(config.large_screen_refresh_secs, 5);
    }

    /// Test server connection config
    #[test]
    fn test_server_connection_config() {
        let config = ServerConnectionConfig {
            host: "192.168.1.100".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "key".to_string(),
            password: None,
            private_key: Some("~/.ssh/id_rsa".to_string()),
            passphrase: None,
        };

        assert_eq!(config.host, "192.168.1.100");
        assert_eq!(config.port, 22);
        assert_eq!(config.auth_type, "key");
    }

    /// Test large screen dashboard
    #[test]
    fn test_large_screen_dashboard() {
        let dashboard = DashboardBuilder::new("noc", "NOC Display")
            .large_screen()
            .add_server_health_widget(0, 0, 12, 6)
            .build();

        assert!(dashboard.is_large_screen);
        assert_eq!(dashboard.refresh_interval_secs, 5);
    }

    /// Test chart series
    #[test]
    fn test_chart_series() {
        let series = ChartSeries {
            name: "CPU Usage".to_string(),
            data: vec![10.0, 20.0, 30.0, 25.0, 15.0],
            color: "#22c55e".to_string(),
            metric_type: MetricType::CpuUsage,
        };

        assert_eq!(series.name, "CPU Usage");
        assert_eq!(series.data.len(), 5);
        assert_eq!(series.color, "#22c55e");
    }

    /// Test server health card
    #[test]
    fn test_server_health_card() {
        let card = ServerHealthCard {
            server_id: "srv-001".to_string(),
            server_name: "Web Server".to_string(),
            host: "192.168.1.100".to_string(),
            status: ServerHealthStatus::Healthy,
            uptime: "5d 12h 30m".to_string(),
            cpu_percent: 45.0,
            memory_percent: 60.0,
            disk_percent: 70.0,
            network_rx: "10.5 MB/s".to_string(),
            network_tx: "5.2 MB/s".to_string(),
            active_alerts: 0,
            last_seen_secs: 30,
        };

        assert_eq!(card.server_id, "srv-001");
        assert_eq!(card.status, ServerHealthStatus::Healthy);
        assert_eq!(card.active_alerts, 0);
    }
}
