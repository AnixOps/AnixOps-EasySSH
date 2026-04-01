//! Event collection and buffering
//!
//! Collects telemetry events and buffers them for batch reporting.
//! Supports in-memory and persistent buffering.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{
    AnonymousId, ConsentManager, PlatformInfo, TelemetryError, TelemetryEvent,
    TelemetryEventRecord,
};

/// Event filter for selective collection
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Allowed event types (empty = all)
    pub allowed_types: Vec<String>,
    /// Blocked event types
    pub blocked_types: Vec<String>,
    /// Minimum severity to collect
    pub min_severity: Option<String>,
    /// Sampling rate (0.0 - 1.0)
    pub sampling_rate: f64,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            allowed_types: vec![],
            blocked_types: vec!["debug".to_string()],
            min_severity: None,
            sampling_rate: 1.0,
        }
    }
}

impl EventFilter {
    /// Check if event should be collected
    pub fn should_collect(&self, event: &TelemetryEvent) -> bool {
        // Check sampling
        if self.sampling_rate < 1.0 {
            let should_sample = rand::random::<f64>() < self.sampling_rate;
            if !should_sample {
                return false;
            }
        }

        // Get event type name
        let event_type = Self::get_event_type_name(event);

        // Check blocked types
        if self.blocked_types.contains(&event_type) {
            return false;
        }

        // Check allowed types (if specified)
        if !self.allowed_types.is_empty() && !self.allowed_types.contains(&event_type) {
            return false;
        }

        true
    }

    fn get_event_type_name(event: &TelemetryEvent) -> String {
        match event {
            TelemetryEvent::AppStarted { .. } => "app_started".to_string(),
            TelemetryEvent::AppClosed { .. } => "app_closed".to_string(),
            TelemetryEvent::FeatureUsed { feature, .. } => format!("feature_{}", feature),
            TelemetryEvent::ScreenViewed { .. } => "screen_viewed".to_string(),
            TelemetryEvent::PerformanceMetric { .. } => "performance".to_string(),
            TelemetryEvent::SshConnected { .. } => "ssh_connected".to_string(),
            TelemetryEvent::SshDisconnected { .. } => "ssh_disconnected".to_string(),
            TelemetryEvent::ErrorOccurred { .. } => "error".to_string(),
            TelemetryEvent::FeedbackSubmitted { .. } => "feedback".to_string(),
            TelemetryEvent::ConsentChanged { .. } => "consent_changed".to_string(),
        }
    }
}

/// Event collector
pub struct EventCollector {
    storage: Arc<dyn EventStorage>,
    consent_manager: Arc<ConsentManager>,
    buffer: Arc<Mutex<VecDeque<TelemetryEventRecord>>>,
    batch_size: usize,
    filter: Arc<Mutex<EventFilter>>,
}

/// Event storage trait
#[async_trait::async_trait]
pub trait EventStorage: Send + Sync {
    /// Store events
    async fn store(&self, events: Vec<TelemetryEventRecord>) -> Result<(), TelemetryError>;

    /// Retrieve events for reporting
    async fn retrieve(&self, batch_size: usize) -> Result<Vec<TelemetryEventRecord>, TelemetryError>;

    /// Delete events
    async fn delete(&self, event_ids: Vec<String>) -> Result<(), TelemetryError>;

    /// Get event count
    async fn count(&self) -> Result<usize, TelemetryError>;

    /// Clear all events
    async fn clear(&self) -> Result<(), TelemetryError>;
}

impl EventCollector {
    pub fn new(
        storage: Arc<dyn EventStorage>,
        consent_manager: Arc<ConsentManager>,
        batch_size: usize,
    ) -> Self {
        Self {
            storage,
            consent_manager,
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(batch_size * 2))),
            batch_size,
            filter: Arc::new(Mutex::new(EventFilter::default())),
        }
    }

    /// Set event filter
    pub fn set_filter(&self, filter: EventFilter) {
        *self.filter.lock().unwrap() = filter;
    }

    /// Collect an event
    pub async fn collect(&self, record: TelemetryEventRecord) {
        // Check filter
        if !self.filter.lock().unwrap().should_collect(&record.event) {
            return;
        }

        // Add to buffer
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push_back(record);

        // Flush if buffer is full
        if buffer.len() >= self.batch_size {
            drop(buffer);
            self.flush().await.ok();
        }
    }

    /// Collect event (convenience method)
    pub async fn collect_event(&self, event: TelemetryEvent) {
        let record = TelemetryEventRecord::new(
            AnonymousId::new(), // Would use actual anonymous ID from manager
            PlatformInfo::current(crate::telemetry::TelemetryEdition::Lite),
            event,
            "session".to_string(), // Would use actual session ID
        );
        self.collect(record).await;
    }

    /// Flush buffer to storage
    pub async fn flush(&self) -> Result<(), TelemetryError> {
        let events: Vec<TelemetryEventRecord> = {
            let mut buffer = self.buffer.lock().unwrap();
            let to_drain = buffer.len().min(self.batch_size);
            buffer.drain(..to_drain).collect()
        };

        if !events.is_empty() {
            self.storage.store(events).await?;
        }

        Ok(())
    }

    /// Get pending count in buffer
    pub fn pending_count(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    /// Get events from storage
    pub async fn retrieve_events(&self, count: usize) -> Result<Vec<TelemetryEventRecord>, TelemetryError> {
        self.storage.retrieve(count).await
    }
}

/// In-memory event storage (for testing/debugging)
pub struct InMemoryStorage {
    events: Arc<Mutex<Vec<TelemetryEventRecord>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl EventStorage for InMemoryStorage {
    async fn store(&self, events: Vec<TelemetryEventRecord>) -> Result<(), TelemetryError> {
        let mut storage = self.events.lock().unwrap();
        storage.extend(events);
        Ok(())
    }

    async fn retrieve(&self, batch_size: usize) -> Result<Vec<TelemetryEventRecord>, TelemetryError> {
        let events = self.events.lock().unwrap();
        Ok(events.iter().take(batch_size).cloned().collect())
    }

    async fn delete(&self, event_ids: Vec<String>) -> Result<(), TelemetryError> {
        let mut events = self.events.lock().unwrap();
        events.retain(|e| !event_ids.contains(&e.id));
        Ok(())
    }

    async fn count(&self) -> Result<usize, TelemetryError> {
        Ok(self.events.lock().unwrap().len())
    }

    async fn clear(&self) -> Result<(), TelemetryError> {
        self.events.lock().unwrap().clear();
        Ok(())
    }
}

/// SQLite event storage (persistent)
pub struct SqliteStorage {
    db_path: std::path::PathBuf,
}

impl SqliteStorage {
    pub fn new(db_path: std::path::PathBuf) -> Result<Self, TelemetryError> {
        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Initialize database
        let conn = rusqlite::Connection::open(&db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS telemetry_events (
                id TEXT PRIMARY KEY,
                anonymous_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT NOT NULL,
                platform_data TEXT NOT NULL,
                session_id TEXT NOT NULL,
                synced INTEGER DEFAULT 0
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON telemetry_events(timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_synced ON telemetry_events(synced)",
            [],
        )?;

        Ok(Self { db_path })
    }
}

#[async_trait::async_trait]
impl EventStorage for SqliteStorage {
    async fn store(&self, events: Vec<TelemetryEventRecord>) -> Result<(), TelemetryError> {
        use rusqlite::params;

        let mut conn = rusqlite::Connection::open(&self.db_path)?;
        let tx = conn.transaction()?;

        for event in events {
            let event_data = serde_json::to_string(&event.event)?;
            let platform_data = serde_json::to_string(&event.platform)?;
            let event_type = match &event.event {
                TelemetryEvent::AppStarted { .. } => "app_started",
                TelemetryEvent::AppClosed { .. } => "app_closed",
                TelemetryEvent::FeatureUsed { .. } => "feature_used",
                TelemetryEvent::ScreenViewed { .. } => "screen_viewed",
                TelemetryEvent::PerformanceMetric { .. } => "performance",
                TelemetryEvent::SshConnected { .. } => "ssh_connected",
                TelemetryEvent::SshDisconnected { .. } => "ssh_disconnected",
                TelemetryEvent::ErrorOccurred { .. } => "error",
                TelemetryEvent::FeedbackSubmitted { .. } => "feedback",
                TelemetryEvent::ConsentChanged { .. } => "consent_changed",
            };

            tx.execute(
                "INSERT INTO telemetry_events
                 (id, anonymous_id, timestamp, event_type, event_data, platform_data, session_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    event.id,
                    event.anonymous_id.as_str(),
                    event.timestamp as i64,
                    event_type,
                    event_data,
                    platform_data,
                    event.session_id,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    async fn retrieve(&self, batch_size: usize) -> Result<Vec<TelemetryEventRecord>, TelemetryError> {
        use rusqlite::params;

        let conn = rusqlite::Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, anonymous_id, timestamp, event_type, event_data, platform_data, session_id
             FROM telemetry_events
             WHERE synced = 0
             ORDER BY timestamp ASC
             LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![batch_size as i64], |row| {
            let id: String = row.get(0)?;
            let anonymous_id = AnonymousId(row.get(1)?);
            let timestamp: i64 = row.get(2)?;
            let _event_type: String = row.get(3)?;
            let event_data: String = row.get(4)?;
            let platform_data: String = row.get(5)?;
            let session_id: String = row.get(6)?;

            let event: TelemetryEvent = serde_json::from_str(&event_data)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let platform: PlatformInfo = serde_json::from_str(&platform_data)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            Ok(TelemetryEventRecord {
                id,
                anonymous_id,
                timestamp: timestamp as u64,
                platform,
                event,
                session_id,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }

        Ok(events)
    }

    async fn delete(&self, event_ids: Vec<String>) -> Result<(), TelemetryError> {
        use rusqlite::params;

        let mut conn = rusqlite::Connection::open(&self.db_path)?;
        let tx = conn.transaction()?;

        for id in event_ids {
            tx.execute("DELETE FROM telemetry_events WHERE id = ?1", params![id])?;
        }

        tx.commit()?;
        Ok(())
    }

    async fn count(&self) -> Result<usize, TelemetryError> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM telemetry_events",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    async fn clear(&self) -> Result<(), TelemetryError> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        conn.execute("DELETE FROM telemetry_events", [])?;
        Ok(())
    }
}

/// Batch event configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Max events per batch
    pub max_size: usize,
    /// Max time to wait before sending
    pub max_wait_ms: u64,
    /// Retry attempts
    pub retry_attempts: u32,
    /// Retry delay
    pub retry_delay_ms: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_size: 50,
            max_wait_ms: 30000,
            retry_attempts: 3,
            retry_delay_ms: 5000,
        }
    }
}
