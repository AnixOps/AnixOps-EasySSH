# EasySSH Telemetry & Analytics System

Privacy-first analytics implementation for EasySSH with GDPR/CCPA compliance.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           EasySSH Telemetry System                           │
├─────────────────────────────────────────────────────────────────────────────┤
│  Core Modules:                                                              │
│  ├── consent.rs       → GDPR/CCPA compliant consent management              │
│  ├── metrics.rs       → Performance metrics collection                      │
│  ├── error_tracker.rs → Error tracking & crash reporting                    │
│  ├── feature_flags.rs → A/B testing & gradual rollouts                      │
│  ├── feedback.rs      → In-app feedback collection                          │
│  ├── health_monitor.rs→ Service health monitoring                           │
│  ├── collector.rs     → Event collection & buffering                        │
│  ├── reporter.rs      → PostHog/Segment/ClickHouse integration              │
│  └── storage.rs       → Local data retention & compliance                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Features

### 1. Anonymous Usage Statistics

All data is anonymized with rotating anonymous IDs:

```rust
// Track feature usage
use easyssh_core::telemetry::{TelemetryManager, TelemetryConfig, TelemetryEdition};

let config = TelemetryConfig {
    edition: TelemetryEdition::Standard,
    api_key: Some("ph_project_api_key".to_string()),
    endpoint: Some("https://app.posthog.com".to_string()),
    ..Default::default()
};

let telemetry = TelemetryManager::new(config)?;
telemetry.initialize().await?;

// Track events
telemetry.track_feature("sftp_upload", HashMap::new()).await;
telemetry.track_screen("server_list", Some(5000)).await;
```

### 2. Performance Telemetry

Track startup time, operation latency, memory usage:

```rust
// Track startup performance
let timer = telemetry.start_timer("app_startup");
// ... initialization code ...
drop(timer); // Auto-records duration

// Track specific operations
telemetry.record_performance("db_query", 45.5, "ms").await;
telemetry.record_performance("ssh_connect", 1200.0, "ms").await;
```

### 3. Consent Management (GDPR/CCPA)

Explicit opt-in required for EU/California users:

```rust
// Check if consent is needed
if telemetry.consent().needs_consent_dialog() {
    // Show consent UI
    let model = telemetry.consent().get_dialog_model();
    // ... display dialog with categories ...
}

// User accepts
telemetry.consent().accept_all().await?;

// Or granular control
telemetry.consent().set_consent(ConsentCategory::Analytics, true).await?;
telemetry.consent().set_consent(ConsentCategory::CrashReporting, false).await?;
```

### 4. Feature Flags (A/B Testing)

Gradual rollouts and experiments:

```rust
// Check if feature is enabled
if telemetry.is_feature_enabled("new_terminal_ui") {
    // Show new UI
}

// Get variant for A/B test
match telemetry.get_feature_variant("quick_connect") {
    Some(Variant { name, .. }) if name == "new_flow" => {
        // Show new quick connect flow
    }
    _ => {
        // Show control variant
    }
}
```

### 5. Error Tracking

Privacy-safe error collection:

```rust
// Track errors (automatically sanitized)
let context = ErrorContext::new("ssh_timeout", "connection")
    .with_message("Connection timed out")
    .with_severity(Severity::Medium);

telemetry.error_tracker().track_error(context).await;

// Setup panic handler
telemetry.error_tracker().setup_panic_handler();
```

### 6. In-App Feedback

Collect user feedback with categorization:

```rust
// Submit rating
telemetry.feedback()
    .submit_rating(FeedbackRating::Stars(4), "ui", user_id, platform, session)
    .await?;

// Detailed feedback
telemetry.feedback()
    .submit_detailed(
        FeedbackType::BugReport,
        FeedbackRating::Stars(2),
        "SFTP transfer is slow",
        "performance",
        context,
        user_id,
        platform,
        session,
    )
    .await?;
```

### 7. Health Monitoring

Service availability tracking:

```rust
// Register health checks
let monitor = telemetry.health_monitor();
monitor.register_check(Box::new(DatabaseHealthCheck::new()));
monitor.register_check(Box::new(SshLibraryHealthCheck::new()));
monitor.register_check(Box::new(SystemResourcesHealthCheck::new()));

monitor.start().await?;

// Get health status
let summary = monitor.get_health_summary();
println!("Overall: {:?}", summary.overall_status);
```

### 8. GDPR/CCPA Compliance

Data export and deletion:

```rust
// Export user data (right to portability)
let export = telemetry.export_user_data().await?;
std::fs::write("my_data_export.json", export)?;

// Delete all data (right to erasure)
telemetry.delete_user_data().await?;
```

## Privacy Features

### Data Anonymization

- Rotating anonymous IDs (no PII)
- Hostnames, IPs, usernames redacted
- No SSH credentials collected
- Stack traces in debug builds only

### Consent Categories

| Category | Description | Default (GDPR) | Default (Other) |
|----------|-------------|----------------|-----------------|
| Essential | App health metrics | ✅ Always | ✅ Always |
| Analytics | Feature usage | ❌ Opt-in | ✅ Enabled |
| Performance | Speed metrics | ❌ Opt-in | ✅ Enabled |
| Crash Reporting | Errors | ❌ Opt-in | ✅ Enabled |
| Experimentation | A/B tests | ❌ Opt-in | ✅ Enabled |
| Feedback | User input | ❌ Opt-in | ✅ Enabled |

### Regional Compliance

- **EU (GDPR)**: Explicit opt-in required
- **California (CCPA)**: Explicit opt-in, 1-year retention
- **Other regions**: Opt-out available

## Backend Integration

### PostHog Configuration

```rust
let config = TelemetryConfig {
    edition: TelemetryEdition::Standard,
    api_key: std::env::var("POSTHOG_API_KEY").ok(),
    endpoint: Some("https://app.posthog.com".to_string()),
    batch_size: 50,
    flush_interval_secs: 30,
    ..Default::default()
};
```

### ClickHouse Data Warehouse

```rust
// For long-term analytics
let clickhouse = ClickHouseClient::new(
    "https://clickhouse.easyssh.io",
    "analytics"
).with_credentials("user", "pass");

clickhouse.insert_events(&events).await?;
```

### Grafana Dashboards

```rust
// Create dashboard
let grafana = GrafanaClient::new("https://grafana.easyssh.io")
    .with_api_key("grafana_api_key");

grafana.upsert_dashboard(GRAFANA_DASHBOARD_TEMPLATE).await?;
```

## Data Retention

```rust
// Configure retention policy
let policy = DataRetentionPolicy {
    retention_days: 90,
    max_local_events: 10000,
    auto_delete: true,
    export_before_delete: false,
};

// GDPR-compliant preset
let gdpr_policy = DataRetentionPolicy::gdpr_compliant(); // 30 days

// CCPA-compliant preset
let ccpa_policy = DataRetentionPolicy::ccpa_compliant(); // 1 year
```

## Events Collected

### Application Events
- `app_started` - Startup time, cold vs warm start
- `app_closed` - Session duration

### SSH Events (Anonymized)
- `ssh_connected` - Auth method, connection time
- `ssh_disconnected` - Session duration, bytes transferred

### Feature Events
- `feature_used` - Feature name, context
- `screen_viewed` - Screen name, time spent

### Performance Events
- `performance_metric` - Metric name, value, unit

### Error Events
- `error_occurred` - Error type, severity, component

### Feedback Events
- `feedback_submitted` - Rating, category

## API Reference

### TelemetryManager

Main entry point for telemetry operations.

```rust
// Lifecycle
TelemetryManager::new(config) -> Result<Self, TelemetryError>
TelemetryManager::initialize(&mut self) -> Result<(), TelemetryError>
TelemetryManager::shutdown(&self) -> Result<(), TelemetryError>

// Event tracking
TelemetryManager::track_event(&self, event: TelemetryEvent)
TelemetryManager::track_feature(&self, feature: &str, context: HashMap)
TelemetryManager::track_screen(&self, screen: &str, time_spent: Option<u64>)
TelemetryManager::track_ssh_connected(&self, auth_method: &str, duration_ms: u64)
TelemetryManager::track_ssh_disconnected(&self, session_duration_ms: u64)

// Consent
TelemetryManager::consent(&self) -> &ConsentManager

// Feature flags
TelemetryManager::is_feature_enabled(&self, flag_name: &str) -> bool
TelemetryManager::get_feature_variant(&self, flag_name: &str) -> Option<Variant>

// Compliance
TelemetryManager::export_user_data(&self) -> Result<String, TelemetryError>
TelemetryManager::delete_user_data(&self) -> Result<(), TelemetryError>
```

## Security Considerations

1. **No sensitive data**: SSH credentials, hostnames, IPs are never collected
2. **Anonymized IDs**: User IDs are random UUIDs, rotated periodically
3. **Local-only storage**: Events buffered locally, no cloud by default
4. **Opt-in only**: EU users must explicitly consent
5. **Encrypted transmission**: HTTPS for all backend communication
6. **Data minimization**: Only collect what's necessary

## Testing

```rust
#[tokio::test]
async fn test_telemetry_consent() {
    let config = TelemetryConfig::default();
    let telemetry = TelemetryManager::new(config).unwrap();

    // Check consent needed for GDPR region
    assert!(telemetry.consent().needs_consent_dialog());

    // Accept consent
    telemetry.consent().accept_all().await.unwrap();
    assert!(!telemetry.consent().needs_consent_dialog());
}

#[tokio::test]
async fn test_feature_flags() {
    let manager = FeatureFlagManager::new().unwrap();
    manager.load_defaults();

    let user_id = AnonymousId::new();
    assert!(manager.is_enabled("sftp_file_preview", &user_id));
}
```

## Dashboard Metrics

The system provides data for:

- **DAU/MAU** - Daily/Monthly Active Users
- **Feature Adoption** - Which features are used most
- **Performance** - Startup time, operation latency
- **Error Rates** - Crashes and errors by component
- **Retention** - User return rate
- **A/B Test Results** - Experiment outcomes

## Implementation Status

| Feature | Status |
|---------|--------|
| Anonymous Usage Stats | ✅ Implemented |
| Performance Telemetry | ✅ Implemented |
| GDPR/CCPA Consent | ✅ Implemented |
| Feature Flags | ✅ Implemented |
| Error Tracking | ✅ Implemented |
| In-App Feedback | ✅ Implemented |
| Health Monitoring | ✅ Implemented |
| PostHog Integration | ✅ Implemented |
| ClickHouse Integration | ✅ Implemented |
| Grafana Dashboards | ✅ Template Provided |

## Example Integration

```rust
use easyssh_core::telemetry::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize telemetry
    let config = TelemetryConfig {
        edition: TelemetryEdition::Standard,
        api_key: std::env::var("POSTHOG_API_KEY").ok(),
        endpoint: Some("https://app.posthog.com".to_string()),
        batch_size: 50,
        flush_interval_secs: 30,
        enable_local_buffer: true,
        retention_days: 90,
        debug_mode: cfg!(debug_assertions),
    };

    let mut telemetry = TelemetryManager::new(config)?;
    telemetry.initialize().await?;

    // Check consent
    if telemetry.consent().needs_consent_dialog() {
        println!("Please accept analytics to help improve EasySSH");
        telemetry.consent().accept_all().await?;
    }

    // Track startup
    telemetry.track_event(TelemetryEvent::AppStarted {
        startup_time_ms: 1200,
        cold_start: true,
    }).await;

    // Check feature flag
    if telemetry.is_feature_enabled("new_ui") {
        println!("New UI enabled!");
    }

    // Track feature usage
    let mut context = HashMap::new();
    context.insert("server_count".to_string(), serde_json::json!(5));
    telemetry.track_feature("server_list_view", context).await;

    // Track performance
    telemetry.record_performance("db_query", 45.5, "ms").await;

    // Shutdown
    telemetry.shutdown().await?;
    Ok(())
}
```
