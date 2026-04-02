//! Connection Model
//!
//! This module defines the Connection domain model for recording
//! connection history and managing active connections.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::{
    is_valid_host, is_valid_port as validate_port, Validatable, ValidationError, DEFAULT_SSH_PORT,
};

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    /// Connection is being established
    Connecting,
    /// Connection is active and established
    Connected,
    /// Connection is being closed
    Disconnecting,
    /// Connection has been closed
    Disconnected,
    /// Connection failed
    Failed,
    /// Connection timed out
    Timeout,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Connecting
    }
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Connecting => write!(f, "connecting"),
            ConnectionStatus::Connected => write!(f, "connected"),
            ConnectionStatus::Disconnecting => write!(f, "disconnecting"),
            ConnectionStatus::Disconnected => write!(f, "disconnected"),
            ConnectionStatus::Failed => write!(f, "failed"),
            ConnectionStatus::Timeout => write!(f, "timeout"),
        }
    }
}

impl ConnectionStatus {
    /// Check if the connection is active (connecting or connected)
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ConnectionStatus::Connecting | ConnectionStatus::Connected
        )
    }

    /// Check if the connection is closed (disconnected or failed or timeout)
    pub fn is_closed(&self) -> bool {
        matches!(
            self,
            ConnectionStatus::Disconnected | ConnectionStatus::Failed | ConnectionStatus::Timeout
        )
    }

    /// Check if the connection has ended (not connecting or connected)
    pub fn has_ended(&self) -> bool {
        !self.is_active()
    }
}

/// Connection record for a specific session
///
/// This represents a single connection attempt/session with a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Unique connection identifier
    pub id: String,
    /// Server ID that was connected to
    pub server_id: String,
    /// Server name at time of connection
    pub server_name: String,
    /// Server host at time of connection
    pub host: String,
    /// Server port at time of connection
    pub port: u16,
    /// Username used for connection
    pub username: String,
    /// User ID who initiated the connection
    pub user_id: Option<String>,
    /// Connection status
    #[serde(default)]
    pub status: ConnectionStatus,
    /// Connection start time
    #[serde(default = "Utc::now")]
    pub started_at: DateTime<Utc>,
    /// Connection end time (if closed)
    pub ended_at: Option<DateTime<Utc>>,
    /// Connection duration (calculated when ended)
    pub duration_seconds: Option<u64>,
    /// Error message if connection failed
    pub error_message: Option<String>,
    /// Error code if connection failed
    pub error_code: Option<String>,
    /// Protocol used (SSH, SFTP, etc.)
    #[serde(default = "default_protocol")]
    pub protocol: String,
    /// Authentication method used
    pub auth_method: String,
    /// Client IP address
    pub client_ip: Option<String>,
    /// Session ID (for tracking active sessions)
    pub session_id: Option<String>,
    /// Bytes transmitted
    #[serde(default)]
    pub bytes_sent: u64,
    /// Bytes received
    #[serde(default)]
    pub bytes_received: u64,
    /// Commands executed count
    #[serde(default)]
    pub commands_executed: u32,
    /// Terminal type if SSH connection
    pub terminal_type: Option<String>,
    /// Connection tags for filtering
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
    /// Schema version
    #[serde(default)]
    pub schema_version: u32,
}

fn default_protocol() -> String {
    "ssh".to_string()
}

impl Connection {
    /// Create a new connection record
    pub fn new(
        server_id: String,
        server_name: String,
        host: String,
        port: u16,
        username: String,
        auth_method: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            server_id,
            server_name,
            host,
            port,
            username,
            user_id: None,
            status: ConnectionStatus::Connecting,
            started_at: Utc::now(),
            ended_at: None,
            duration_seconds: None,
            error_message: None,
            error_code: None,
            protocol: "ssh".to_string(),
            auth_method,
            client_ip: None,
            session_id: None,
            bytes_sent: 0,
            bytes_received: 0,
            commands_executed: 0,
            terminal_type: None,
            tags: vec![],
            metadata: std::collections::HashMap::new(),
            schema_version: 1,
        }
    }

    /// Create a connection with specific ID (for loading from database)
    #[allow(clippy::too_many_arguments)]
    pub fn with_id(
        id: String,
        server_id: String,
        server_name: String,
        host: String,
        port: u16,
        username: String,
        user_id: Option<String>,
        status: ConnectionStatus,
        started_at: DateTime<Utc>,
        ended_at: Option<DateTime<Utc>>,
        auth_method: String,
    ) -> Self {
        let duration = ended_at.map(|end| {
            let duration = end.signed_duration_since(started_at);
            duration.num_seconds().max(0) as u64
        });

        Self {
            id,
            server_id,
            server_name,
            host,
            port,
            username,
            user_id,
            status,
            started_at,
            ended_at,
            duration_seconds: duration,
            error_message: None,
            error_code: None,
            protocol: "ssh".to_string(),
            auth_method,
            client_ip: None,
            session_id: None,
            bytes_sent: 0,
            bytes_received: 0,
            commands_executed: 0,
            terminal_type: None,
            tags: vec![],
            metadata: std::collections::HashMap::new(),
            schema_version: 1,
        }
    }

    /// Mark the connection as established
    pub fn mark_connected(&mut self) {
        self.status = ConnectionStatus::Connected;
    }

    /// Mark the connection as disconnected
    pub fn mark_disconnected(&mut self) {
        let now = Utc::now();
        self.status = ConnectionStatus::Disconnected;
        self.ended_at = Some(now);
        self.calculate_duration();
    }

    /// Mark the connection as failed
    pub fn mark_failed(&mut self, error: String, code: Option<String>) {
        let now = Utc::now();
        self.status = ConnectionStatus::Failed;
        self.ended_at = Some(now);
        self.error_message = Some(error);
        self.error_code = code;
        self.calculate_duration();
    }

    /// Mark the connection as timed out
    pub fn mark_timeout(&mut self) {
        let now = Utc::now();
        self.status = ConnectionStatus::Timeout;
        self.ended_at = Some(now);
        self.error_code = Some("TIMEOUT".to_string());
        self.calculate_duration();
    }

    /// Calculate the connection duration
    fn calculate_duration(&mut self) {
        if let Some(end) = self.ended_at {
            let duration = end.signed_duration_since(self.started_at);
            self.duration_seconds = Some(duration.num_seconds().max(0) as u64);
        }
    }

    /// Get the current duration (for active connections)
    pub fn current_duration(&self) -> Duration {
        let end = self.ended_at.unwrap_or_else(Utc::now);
        end.signed_duration_since(self.started_at)
    }

    /// Get duration as human-readable string
    pub fn duration_text(&self) -> String {
        let seconds = self
            .duration_seconds
            .unwrap_or_else(|| self.current_duration().num_seconds().max(0) as u64);

        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else {
            format!(
                "{}h {}m {}s",
                seconds / 3600,
                (seconds % 3600) / 60,
                seconds % 60
            )
        }
    }

    /// Add bytes transferred
    pub fn add_transfer(&mut self, sent: u64, received: u64) {
        self.bytes_sent += sent;
        self.bytes_received += received;
    }

    /// Increment commands executed count
    pub fn record_command(&mut self) {
        self.commands_executed += 1;
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Get total bytes transferred
    pub fn total_bytes(&self) -> u64 {
        self.bytes_sent + self.bytes_received
    }

    /// Check if connection was successful
    pub fn was_successful(&self) -> bool {
        matches!(
            self.status,
            ConnectionStatus::Connected | ConnectionStatus::Disconnected
        )
    }

    /// Get connection summary
    pub fn summary(&self) -> String {
        format!(
            "{}@{}:{} [{}] - {}",
            self.username,
            self.host,
            self.port,
            self.status,
            self.duration_text()
        )
    }
}

impl Validatable for Connection {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate server_id
        if self.server_id.is_empty() {
            return Err(ValidationError::MissingField("server_id".to_string()));
        }

        // Validate host
        if self.host.is_empty() {
            return Err(ValidationError::MissingField("host".to_string()));
        }
        if !is_valid_host(&self.host) {
            return Err(ValidationError::InvalidFormat {
                field: "host".to_string(),
                expected: "valid hostname or IP address".to_string(),
            });
        }

        // Validate port
        if !validate_port(self.port) {
            return Err(ValidationError::InvalidField {
                field: "port".to_string(),
                message: format!("Invalid port: {}", self.port),
            });
        }

        // Validate username
        if self.username.is_empty() {
            return Err(ValidationError::MissingField("username".to_string()));
        }

        // Validate that ended_at is after started_at if present
        if let Some(ended) = self.ended_at {
            if ended < self.started_at {
                return Err(ValidationError::InvalidField {
                    field: "ended_at".to_string(),
                    message: "End time must be after start time".to_string(),
                });
            }
        }

        Ok(())
    }
}

/// Connection history summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHistory {
    /// Total number of connections
    pub total_connections: u64,
    /// Number of successful connections
    pub successful_connections: u64,
    /// Number of failed connections
    pub failed_connections: u64,
    /// Total time connected (in seconds)
    pub total_duration_seconds: u64,
    /// Most recent connection
    pub last_connection: Option<DateTime<Utc>>,
    /// Most frequently connected server
    pub favorite_server_id: Option<String>,
    /// Connection counts per server
    #[serde(default)]
    pub connections_per_server: std::collections::HashMap<String, u64>,
}

impl ConnectionHistory {
    /// Create empty history
    pub fn new() -> Self {
        Self {
            total_connections: 0,
            successful_connections: 0,
            failed_connections: 0,
            total_duration_seconds: 0,
            last_connection: None,
            favorite_server_id: None,
            connections_per_server: std::collections::HashMap::new(),
        }
    }

    /// Add a connection to the history
    pub fn record_connection(&mut self, connection: &Connection) {
        self.total_connections += 1;

        if connection.was_successful() {
            self.successful_connections += 1;
        } else {
            self.failed_connections += 1;
        }

        if let Some(duration) = connection.duration_seconds {
            self.total_duration_seconds += duration;
        }

        self.last_connection = Some(connection.started_at);

        // Update per-server count
        let count = self
            .connections_per_server
            .entry(connection.server_id.clone())
            .or_insert(0);
        *count += 1;

        // Update favorite server
        let mut max_count = 0u64;
        let mut favorite_id = None;
        for (server_id, count) in &self.connections_per_server {
            if *count > max_count {
                max_count = *count;
                favorite_id = Some(server_id.clone());
            }
        }
        self.favorite_server_id = favorite_id;
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_connections == 0 {
            0.0
        } else {
            (self.successful_connections as f64 / self.total_connections as f64) * 100.0
        }
    }

    /// Get average duration per connection
    pub fn average_duration(&self) -> u64 {
        if self.total_connections == 0 {
            0
        } else {
            self.total_duration_seconds / self.total_connections
        }
    }
}

impl Default for ConnectionHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter for querying connections
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionFilter {
    /// Filter by server ID
    pub server_id: Option<String>,
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by status
    pub status: Option<ConnectionStatus>,
    /// Filter by start date (inclusive)
    pub started_after: Option<DateTime<Utc>>,
    /// Filter by start date (inclusive)
    pub started_before: Option<DateTime<Utc>>,
    /// Filter by tags (any match)
    pub tags: Vec<String>,
    /// Filter by protocol
    pub protocol: Option<String>,
    /// Search query (matches host, username, server_name)
    pub search: Option<String>,
    /// Only successful connections
    pub only_successful: bool,
    /// Only failed connections
    pub only_failed: bool,
}

impl ConnectionFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by server
    pub fn by_server(mut self, server_id: impl Into<String>) -> Self {
        self.server_id = Some(server_id.into());
        self
    }

    /// Filter by user
    pub fn by_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Filter by status
    pub fn by_status(mut self, status: ConnectionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by date range
    pub fn date_range(
        mut self,
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
    ) -> Self {
        self.started_after = after;
        self.started_before = before;
        self
    }

    /// Filter by tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Filter by search query
    pub fn search(mut self, query: impl Into<String>) -> Self {
        self.search = Some(query.into());
        self
    }

    /// Only successful connections
    pub fn successful_only(mut self) -> Self {
        self.only_successful = true;
        self.only_failed = false;
        self
    }

    /// Only failed connections
    pub fn failed_only(mut self) -> Self {
        self.only_failed = true;
        self.only_successful = false;
        self
    }

    /// Check if connection matches this filter
    pub fn matches(&self, connection: &Connection) -> bool {
        // Check server_id
        if let Some(ref server_id) = self.server_id {
            if connection.server_id != *server_id {
                return false;
            }
        }

        // Check user_id
        if let Some(ref user_id) = self.user_id {
            if connection.user_id.as_ref() != Some(user_id) {
                return false;
            }
        }

        // Check status
        if let Some(status) = self.status {
            if connection.status != status {
                return false;
            }
        }

        // Check date range
        if let Some(after) = self.started_after {
            if connection.started_at < after {
                return false;
            }
        }
        if let Some(before) = self.started_before {
            if connection.started_at > before {
                return false;
            }
        }

        // Check tags
        if !self.tags.is_empty() {
            let has_matching_tag = self.tags.iter().any(|tag| connection.tags.contains(tag));
            if !has_matching_tag {
                return false;
            }
        }

        // Check protocol
        if let Some(ref protocol) = self.protocol {
            if connection.protocol != *protocol {
                return false;
            }
        }

        // Check search query
        if let Some(ref search) = self.search {
            let search_lower = search.to_lowercase();
            let matches_search = connection.host.to_lowercase().contains(&search_lower)
                || connection.username.to_lowercase().contains(&search_lower)
                || connection
                    .server_name
                    .to_lowercase()
                    .contains(&search_lower);
            if !matches_search {
                return false;
            }
        }

        // Check success/failure filters
        if self.only_successful && !connection.was_successful() {
            return false;
        }
        if self.only_failed && connection.was_successful() {
            return false;
        }

        true
    }
}

/// DTO for creating a new connection record
#[derive(Debug, Clone, Deserialize)]
pub struct CreateConnectionDto {
    pub server_id: String,
    pub server_name: String,
    pub host: String,
    #[serde(default = "default_port_dto")]
    pub port: u16,
    pub username: String,
    pub auth_method: String,
    pub user_id: Option<String>,
}

fn default_port_dto() -> u16 {
    DEFAULT_SSH_PORT
}

/// DTO for updating a connection record
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConnectionDto {
    pub status: Option<ConnectionStatus>,
    pub ended_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub bytes_sent: Option<u64>,
    pub bytes_received: Option<u64>,
    pub commands_executed: Option<u32>,
}

/// Connection record for database persistence (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRecord {
    pub id: String,
    pub server_id: String,
    pub user_id: Option<String>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub error_message: Option<String>,
    pub protocol: String,
    pub auth_method: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_new() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Test Server".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        assert_eq!(conn.server_id, "srv-123");
        assert_eq!(conn.server_name, "Test Server");
        assert_eq!(conn.host, "192.168.1.1");
        assert_eq!(conn.port, 22);
        assert_eq!(conn.username, "root");
        assert_eq!(conn.auth_method, "agent");
        assert_eq!(conn.protocol, "ssh");
        assert!(matches!(conn.status, ConnectionStatus::Connecting));
    }

    #[test]
    fn test_connection_lifecycle() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        // Connecting -> Connected
        conn.mark_connected();
        assert!(matches!(conn.status, ConnectionStatus::Connected));
        assert!(conn.ended_at.is_none());

        // Connected -> Disconnected
        std::thread::sleep(std::time::Duration::from_millis(10));
        conn.mark_disconnected();
        assert!(matches!(conn.status, ConnectionStatus::Disconnected));
        assert!(conn.ended_at.is_some());
        assert!(conn.duration_seconds.is_some());
    }

    #[test]
    fn test_connection_failed() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "password".to_string(),
        );

        conn.mark_failed(
            "Authentication failed".to_string(),
            Some("AUTH_FAILED".to_string()),
        );

        assert!(matches!(conn.status, ConnectionStatus::Failed));
        assert_eq!(
            conn.error_message,
            Some("Authentication failed".to_string())
        );
        assert_eq!(conn.error_code, Some("AUTH_FAILED".to_string()));
        assert!(!conn.was_successful());
    }

    #[test]
    fn test_connection_timeout() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        conn.mark_timeout();
        assert!(matches!(conn.status, ConnectionStatus::Timeout));
        assert_eq!(conn.error_code, Some("TIMEOUT".to_string()));
    }

    #[test]
    fn test_connection_duration_text() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        // Test with no ended_at (active connection)
        conn.started_at = Utc::now() - Duration::seconds(45);
        assert!(conn.duration_text().contains("m") || conn.duration_text().contains("s"));

        // Test with ended_at
        conn.duration_seconds = Some(3661);
        assert!(conn.duration_text().contains("h"));
    }

    #[test]
    fn test_connection_status_helpers() {
        assert!(ConnectionStatus::Connecting.is_active());
        assert!(ConnectionStatus::Connected.is_active());
        assert!(!ConnectionStatus::Disconnected.is_active());

        assert!(ConnectionStatus::Disconnected.is_closed());
        assert!(ConnectionStatus::Failed.is_closed());
        assert!(ConnectionStatus::Timeout.is_closed());
        assert!(!ConnectionStatus::Connected.is_closed());
    }

    #[test]
    fn test_connection_transfer() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        conn.add_transfer(1000, 2000);
        assert_eq!(conn.bytes_sent, 1000);
        assert_eq!(conn.bytes_received, 2000);
        assert_eq!(conn.total_bytes(), 3000);

        conn.add_transfer(500, 500);
        assert_eq!(conn.bytes_sent, 1500);
        assert_eq!(conn.bytes_received, 2500);
    }

    #[test]
    fn test_connection_validation() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        assert!(conn.validate().is_ok());
    }

    #[test]
    fn test_connection_validation_empty_host() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        assert!(
            matches!(conn.validate(), Err(ValidationError::MissingField(field)) if field == "host")
        );
    }

    #[test]
    fn test_connection_validation_invalid_port() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            0,
            "root".to_string(),
            "agent".to_string(),
        );
        assert!(
            matches!(conn.validate(), Err(ValidationError::InvalidField { field, .. }) if field == "port")
        );
    }

    #[test]
    fn test_connection_tags() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        conn.add_tag("production".to_string());
        conn.add_tag("important".to_string());
        assert!(conn.tags.contains(&"production".to_string()));
        assert_eq!(conn.tags.len(), 2);

        // Adding duplicate should not increase count
        conn.add_tag("production".to_string());
        assert_eq!(conn.tags.len(), 2);

        conn.remove_tag("production");
        assert!(!conn.tags.contains(&"production".to_string()));
    }

    #[test]
    fn test_connection_metadata() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        conn.set_metadata("client_version".to_string(), "1.0.0".to_string());
        assert_eq!(
            conn.get_metadata("client_version"),
            Some(&"1.0.0".to_string())
        );
        assert_eq!(conn.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_connection_history() {
        let mut history = ConnectionHistory::new();
        assert_eq!(history.total_connections, 0);

        let conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        history.record_connection(&conn);
        assert_eq!(history.total_connections, 1);
        assert!(history.last_connection.is_some());
    }

    #[test]
    fn test_connection_filter() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        let filter = ConnectionFilter::new()
            .by_server("srv-123")
            .search("192.168");

        assert!(filter.matches(&conn));

        let no_match = ConnectionFilter::new().by_server("srv-999");
        assert!(!no_match.matches(&conn));
    }

    #[test]
    fn test_connection_serialization() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        let json = serde_json::to_string(&conn).unwrap();
        assert!(json.contains("srv-123"));
        assert!(json.contains("192.168.1.1"));

        let deserialized: Connection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.server_id, "srv-123");
        assert_eq!(deserialized.port, 22);
    }

    #[test]
    fn test_create_connection_dto() {
        let json = r##"{
            "server_id": "srv-123",
            "server_name": "Test",
            "host": "192.168.1.1",
            "port": 2222,
            "username": "admin",
            "auth_method": "key"
        }"##;

        let dto: CreateConnectionDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.server_id, "srv-123");
        assert_eq!(dto.port, 2222);
    }

    #[test]
    fn test_connection_summary() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Production".to_string(),
            "example.com".to_string(),
            22,
            "deploy".to_string(),
            "key".to_string(),
        );

        let summary = conn.summary();
        assert!(summary.contains("deploy"));
        assert!(summary.contains("example.com"));
        assert!(summary.contains("22"));
    }
}
