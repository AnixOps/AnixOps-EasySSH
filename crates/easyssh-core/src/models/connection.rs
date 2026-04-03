//! Connection Model
//!
//! This module defines the Connection domain model for recording
//! connection history and managing active connections.
//!
//! Connections track SSH sessions from start to finish, including metadata
//! about the session such as duration, bytes transferred, and commands executed.
//!
//! # Examples
//!
//! ```
//! use easyssh_core::models::{Connection, ConnectionStatus};
//!
//! let mut conn = Connection::new(
//!     "srv-123".to_string(),
//!     "Production".to_string(),
//!     "192.168.1.1".to_string(),
//!     22,
//!     "admin".to_string(),
//!     "key".to_string(),
//! );
//!
//! conn.mark_connected();
//! // ... connection is active ...
//! conn.mark_disconnected();
//!
//! assert!(conn.was_successful());
//! ```

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::{
    is_valid_host, is_valid_port as validate_port, Validatable, ValidationError, DEFAULT_SSH_PORT,
    MAX_USERNAME_LENGTH,
};

/// Connection status
///
/// Tracks the lifecycle of a connection from initial connection through disconnection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    /// Connection is being established
    #[default]
    Connecting,
    /// Connection is active and established
    Connected,
    /// Connection is being closed
    Disconnecting,
    /// Connection has been closed normally
    Disconnected,
    /// Connection failed (authentication error, network error, etc.)
    Failed,
    /// Connection timed out
    Timeout,
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
    ///
    /// Active connections can transfer data and execute commands.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ConnectionStatus::Connecting | ConnectionStatus::Connected
        )
    }

    /// Check if the connection is established
    ///
    /// Established connections are ready for commands.
    pub fn is_established(&self) -> bool {
        matches!(self, ConnectionStatus::Connected)
    }

    /// Check if the connection is closed (disconnected or failed or timeout)
    ///
    /// Closed connections cannot be used for new commands.
    pub fn is_closed(&self) -> bool {
        matches!(
            self,
            ConnectionStatus::Disconnected | ConnectionStatus::Failed | ConnectionStatus::Timeout
        )
    }

    /// Check if the connection has ended (not connecting or connected)
    ///
    /// This is a convenience method that checks if the connection
    /// is in any terminal state.
    pub fn has_ended(&self) -> bool {
        !self.is_active()
    }

    /// Check if the connection ended with an error
    pub fn is_error(&self) -> bool {
        matches!(self, ConnectionStatus::Failed | ConnectionStatus::Timeout)
    }

    /// Convert to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionStatus::Connecting => "connecting",
            ConnectionStatus::Connected => "connected",
            ConnectionStatus::Disconnecting => "disconnecting",
            ConnectionStatus::Disconnected => "disconnected",
            ConnectionStatus::Failed => "failed",
            ConnectionStatus::Timeout => "timeout",
        }
    }

    /// Parse from string
    pub fn from_status_str(s: &str) -> Self {
        match s {
            "connecting" => ConnectionStatus::Connecting,
            "connected" => ConnectionStatus::Connected,
            "disconnecting" => ConnectionStatus::Disconnecting,
            "disconnected" => ConnectionStatus::Disconnected,
            "failed" => ConnectionStatus::Failed,
            "timeout" => ConnectionStatus::Timeout,
            _ => ConnectionStatus::Connecting,
        }
    }
}

/// Connection record for a specific session
///
/// This represents a single connection attempt/session with a server.
/// It tracks all relevant metadata about the connection for auditing
/// and analytics purposes.
///
/// # Fields
///
/// * `id` - Unique connection identifier
/// * `server_id` - Reference to the server that was connected to
/// * `server_name` - Server name at time of connection (denormalized for history)
/// * `host`, `port`, `username` - Connection parameters (denormalized)
/// * `status` - Current connection status
/// * `started_at`, `ended_at` - Session timestamps
/// * `duration_seconds` - Calculated session duration
/// * `bytes_sent`, `bytes_received` - Transfer statistics
/// * `commands_executed` - Command count for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Unique connection identifier (UUID)
    pub id: String,
    /// Server ID that was connected to
    pub server_id: String,
    /// Server name at time of connection
    pub server_name: String,
    /// Server host at time of connection
    pub host: String,
    /// Server port at time of connection
    #[serde(default = "default_port")]
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

fn default_port() -> u16 {
    DEFAULT_SSH_PORT
}

fn default_protocol() -> String {
    "ssh".to_string()
}

impl Connection {
    /// Create a new connection record
    ///
    /// # Arguments
    /// * `server_id` - The server being connected to
    /// * `server_name` - The server name (for history display)
    /// * `host` - The host address
    /// * `port` - The SSH port
    /// * `username` - The authentication username
    /// * `auth_method` - String representation of auth method used
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::models::Connection;
    ///
    /// let conn = Connection::new(
    ///     "srv-123".to_string(),
    ///     "Production".to_string(),
    ///     "192.168.1.1".to_string(),
    ///     22,
    ///     "root".to_string(),
    ///     "agent".to_string(),
    /// );
    ///
    /// assert!(conn.status.is_active());
    /// ```
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
    ///
    /// Should be called when the SSH handshake completes successfully.
    pub fn mark_connected(&mut self) {
        self.status = ConnectionStatus::Connected;
    }

    /// Mark the connection as disconnected
    ///
    /// Should be called when the connection closes normally.
    pub fn mark_disconnected(&mut self) {
        let now = Utc::now();
        self.status = ConnectionStatus::Disconnected;
        self.ended_at = Some(now);
        self.calculate_duration();
    }

    /// Mark the connection as failed
    ///
    /// # Arguments
    /// * `error` - Human-readable error message
    /// * `code` - Optional machine-readable error code
    pub fn mark_failed(&mut self, error: String, code: Option<String>) {
        let now = Utc::now();
        self.status = ConnectionStatus::Failed;
        self.ended_at = Some(now);
        self.error_message = Some(error);
        self.error_code = code;
        self.calculate_duration();
    }

    /// Mark the connection as timed out
    ///
    /// Sets the error code to "TIMEOUT".
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
    ///
    /// Returns the duration from start to now for active connections.
    pub fn current_duration(&self) -> Duration {
        let end = self.ended_at.unwrap_or_else(Utc::now);
        end.signed_duration_since(self.started_at)
    }

    /// Get duration as human-readable string
    ///
    /// Format: "1h 30m 45s" or "30m 45s" or "45s" depending on duration.
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

    /// Get duration formatted for display (compact)
    ///
    /// Format: "1:30:45" for hours, "30:45" for minutes, "45s" for seconds.
    pub fn duration_compact(&self) -> String {
        let seconds = self
            .duration_seconds
            .unwrap_or_else(|| self.current_duration().num_seconds().max(0) as u64);

        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}:{:02}", seconds / 60, seconds % 60)
        } else {
            format!(
                "{}:{:02}:{:02}",
                seconds / 3600,
                (seconds % 3600) / 60,
                seconds % 60
            )
        }
    }

    /// Add bytes transferred
    ///
    /// # Arguments
    /// * `sent` - Bytes sent during this transfer
    /// * `received` - Bytes received during this transfer
    pub fn add_transfer(&mut self, sent: u64, received: u64) {
        self.bytes_sent += sent;
        self.bytes_received += received;
    }

    /// Increment commands executed count
    ///
    /// Should be called when a command completes execution.
    pub fn record_command(&mut self) {
        self.commands_executed += 1;
    }

    /// Add a tag to the connection
    ///
    /// Tags are unique - adding the same tag twice has no effect.
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag from the connection
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    /// Check if the connection has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }

    /// Set metadata value
    ///
    /// Metadata is stored as key-value pairs for extensibility.
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Remove metadata entry
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        self.metadata.remove(key)
    }

    /// Get total bytes transferred
    pub fn total_bytes(&self) -> u64 {
        self.bytes_sent + self.bytes_received
    }

    /// Check if connection was successful
    ///
    /// Returns true if the connection reached the connected or disconnected state.
    pub fn was_successful(&self) -> bool {
        matches!(
            self.status,
            ConnectionStatus::Connected | ConnectionStatus::Disconnected
        )
    }

    /// Get connection summary
    ///
    /// Format: `username@host:port [status] - duration`
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

    /// Get a display label for the connection
    pub fn display_label(&self) -> String {
        format!("{} - {}", self.server_name, self.summary())
    }

    /// Clone without sensitive metadata
    ///
    /// Creates a copy with sensitive fields redacted for logging.
    pub fn clone_redacted(&self) -> Self {
        let mut redacted_metadata = self.metadata.clone();
        redacted_metadata.insert("password".to_string(), "***".to_string());
        redacted_metadata.insert("passphrase".to_string(), "***".to_string());

        Self {
            id: self.id.clone(),
            server_id: self.server_id.clone(),
            server_name: self.server_name.clone(),
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            user_id: self.user_id.clone(),
            status: self.status,
            started_at: self.started_at,
            ended_at: self.ended_at,
            duration_seconds: self.duration_seconds,
            error_message: self.error_message.clone(),
            error_code: self.error_code.clone(),
            protocol: self.protocol.clone(),
            auth_method: self.auth_method.clone(),
            client_ip: self.client_ip.clone(),
            session_id: self.session_id.clone(),
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
            commands_executed: self.commands_executed,
            terminal_type: self.terminal_type.clone(),
            tags: self.tags.clone(),
            metadata: redacted_metadata,
            schema_version: self.schema_version,
        }
    }
}

impl Validatable for Connection {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        // Validate server_id
        if self.server_id.trim().is_empty() {
            errors.push(ValidationError::missing_field("server_id"));
        }

        // Validate host
        if self.host.trim().is_empty() {
            errors.push(ValidationError::missing_field("host"));
        } else if !is_valid_host(&self.host) {
            errors.push(ValidationError::invalid_format(
                "host",
                "valid hostname or IP address",
            ));
        }

        // Validate port
        if !validate_port(self.port) {
            errors.push(ValidationError::invalid_field(
                "port",
                format!("Invalid port: {}. Must be 1-65535", self.port),
            ));
        }

        // Validate username
        if self.username.trim().is_empty() {
            errors.push(ValidationError::missing_field("username"));
        } else if self.username.len() > MAX_USERNAME_LENGTH {
            errors.push(ValidationError::invalid_field(
                "username",
                format!("Username too long (max {} characters)", MAX_USERNAME_LENGTH),
            ));
        }

        // Validate that ended_at is after started_at if present
        if let Some(ended) = self.ended_at {
            if ended < self.started_at {
                errors.push(ValidationError::invalid_field(
                    "ended_at",
                    "End time must be after start time",
                ));
            }
        }

        // Validate duration consistency
        if let Some(duration) = self.duration_seconds {
            if let Some(ended) = self.ended_at {
                let calculated = ended
                    .signed_duration_since(self.started_at)
                    .num_seconds()
                    .max(0) as u64;
                // Allow small discrepancy due to rounding
                if duration > calculated + 1 {
                    errors.push(ValidationError::invalid_field(
                        "duration_seconds",
                        "Duration does not match start/end times",
                    ));
                }
            }
        }

        ValidationError::combine(errors)
    }
}

/// Connection history summary
///
/// Aggregated statistics about a user's connection history.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionHistory {
    /// Total number of connections
    #[serde(default)]
    pub total_connections: u64,
    /// Number of successful connections
    #[serde(default)]
    pub successful_connections: u64,
    /// Number of failed connections
    #[serde(default)]
    pub failed_connections: u64,
    /// Total time connected (in seconds)
    #[serde(default)]
    pub total_duration_seconds: u64,
    /// Most recent connection timestamp
    pub last_connection: Option<DateTime<Utc>>,
    /// Most frequently connected server ID
    pub favorite_server_id: Option<String>,
    /// Connection counts per server
    #[serde(default)]
    pub connections_per_server: std::collections::HashMap<String, u64>,
}

impl ConnectionHistory {
    /// Create empty history
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a connection to the history
    ///
    /// Updates all aggregated statistics based on the connection record.
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

    /// Get success rate as percentage (0.0 - 100.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_connections == 0 {
            0.0
        } else {
            (self.successful_connections as f64 / self.total_connections as f64) * 100.0
        }
    }

    /// Get average duration per connection in seconds
    pub fn average_duration(&self) -> u64 {
        if self.total_connections == 0 {
            0
        } else {
            self.total_duration_seconds / self.total_connections
        }
    }

    /// Get total bytes transferred across all connections
    ///
    /// Note: This requires the Connection objects to calculate accurately.
    /// This method returns 0 from the history alone; use record_connection_with_stats
    /// to track bytes across connections.
    pub fn total_bytes_transferred(&self) -> u64 {
        // This is a placeholder - in a real implementation you'd store
        // bytes in the history or pass Connection objects
        0
    }

    /// Reset all statistics
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

/// Filter for querying connections
///
/// Provides a flexible way to filter connection records.
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
    #[serde(default)]
    pub tags: Vec<String>,
    /// Filter by protocol
    pub protocol: Option<String>,
    /// Search query (matches host, username, server_name)
    pub search: Option<String>,
    /// Only successful connections
    #[serde(default)]
    pub only_successful: bool,
    /// Only failed connections
    #[serde(default)]
    pub only_failed: bool,
    /// Minimum duration in seconds
    pub min_duration_seconds: Option<u64>,
    /// Maximum duration in seconds
    pub max_duration_seconds: Option<u64>,
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

    /// Filter by duration range
    pub fn duration_range(mut self, min: Option<u64>, max: Option<u64>) -> Self {
        self.min_duration_seconds = min;
        self.max_duration_seconds = max;
        self
    }

    /// Check if connection matches this filter
    ///
    /// All specified filters must match for the connection to be included.
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

        // Check duration range
        if let Some(min_duration) = self.min_duration_seconds {
            let duration = connection.duration_seconds.unwrap_or(0);
            if duration < min_duration {
                return false;
            }
        }
        if let Some(max_duration) = self.max_duration_seconds {
            let duration = connection.duration_seconds.unwrap_or(u64::MAX);
            if duration > max_duration {
                return false;
            }
        }

        true
    }

    /// Check if this filter has any constraints
    pub fn is_empty(&self) -> bool {
        self.server_id.is_none()
            && self.user_id.is_none()
            && self.status.is_none()
            && self.started_after.is_none()
            && self.started_before.is_none()
            && self.tags.is_empty()
            && self.protocol.is_none()
            && self.search.is_none()
            && !self.only_successful
            && !self.only_failed
            && self.min_duration_seconds.is_none()
            && self.max_duration_seconds.is_none()
    }
}

/// DTO for creating a new connection record
#[derive(Debug, Clone, Deserialize)]
pub struct CreateConnectionDto {
    /// Server ID (required)
    pub server_id: String,
    /// Server name for display
    pub server_name: String,
    /// Host address
    pub host: String,
    /// SSH port (default: 22)
    #[serde(default = "default_port_dto")]
    pub port: u16,
    /// Username for authentication
    pub username: String,
    /// Authentication method used
    pub auth_method: String,
    /// Optional user ID
    pub user_id: Option<String>,
}

fn default_port_dto() -> u16 {
    DEFAULT_SSH_PORT
}

/// DTO for updating a connection record
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConnectionDto {
    /// New status
    pub status: Option<ConnectionStatus>,
    /// End timestamp
    pub ended_at: Option<DateTime<Utc>>,
    /// Error message
    pub error_message: Option<String>,
    /// Error code
    pub error_code: Option<String>,
    /// Bytes sent
    pub bytes_sent: Option<u64>,
    /// Bytes received
    pub bytes_received: Option<u64>,
    /// Commands executed
    pub commands_executed: Option<u32>,
    /// Connection tags
    pub tags: Option<Vec<String>>,
}

/// Connection record for database persistence (simplified)
///
/// This is a lighter-weight version for storage efficiency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRecord {
    /// Connection ID
    pub id: String,
    /// Server ID
    pub server_id: String,
    /// User ID
    pub user_id: Option<String>,
    /// Status string
    pub status: String,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// End timestamp
    pub ended_at: Option<DateTime<Utc>>,
    /// Duration in seconds
    pub duration_seconds: Option<u64>,
    /// Error message
    pub error_message: Option<String>,
    /// Protocol used
    pub protocol: String,
    /// Auth method used
    pub auth_method: String,
}

impl From<&Connection> for ConnectionRecord {
    fn from(conn: &Connection) -> Self {
        Self {
            id: conn.id.clone(),
            server_id: conn.server_id.clone(),
            user_id: conn.user_id.clone(),
            status: conn.status.as_str().to_string(),
            started_at: conn.started_at,
            ended_at: conn.ended_at,
            duration_seconds: conn.duration_seconds,
            error_message: conn.error_message.clone(),
            protocol: conn.protocol.clone(),
            auth_method: conn.auth_method.clone(),
        }
    }
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
        assert!(!conn.id.is_empty());
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
        assert!(conn.status.is_active());
        assert!(conn.status.is_established());

        // Connected -> Disconnected
        std::thread::sleep(std::time::Duration::from_millis(10));
        conn.mark_disconnected();
        assert!(matches!(conn.status, ConnectionStatus::Disconnected));
        assert!(conn.ended_at.is_some());
        assert!(conn.duration_seconds.is_some());
        assert!(conn.status.is_closed());
        assert!(!conn.status.is_active());
        assert!(conn.was_successful());
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
        assert!(conn.status.is_error());
        assert!(conn.status.has_ended());
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
        assert!(conn.ended_at.is_some());
        assert!(!conn.was_successful());
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

        // Test with ended_at
        conn.duration_seconds = Some(45);
        assert_eq!(conn.duration_text(), "45s");

        conn.duration_seconds = Some(125);
        assert_eq!(conn.duration_text(), "2m 5s");

        conn.duration_seconds = Some(3661);
        assert_eq!(conn.duration_text(), "1h 1m 1s");
    }

    #[test]
    fn test_connection_duration_compact() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        conn.duration_seconds = Some(45);
        assert_eq!(conn.duration_compact(), "45s");

        conn.duration_seconds = Some(125);
        assert_eq!(conn.duration_compact(), "2:05");

        conn.duration_seconds = Some(3661);
        assert_eq!(conn.duration_compact(), "1:01:01");
    }

    #[test]
    fn test_connection_current_duration() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        conn.started_at = Utc::now() - Duration::seconds(60);

        let duration = conn.current_duration();
        assert!(duration.num_seconds() >= 60);
    }

    #[test]
    fn test_connection_status_helpers() {
        assert!(ConnectionStatus::Connecting.is_active());
        assert!(ConnectionStatus::Connected.is_active());
        assert!(!ConnectionStatus::Disconnected.is_active());
        assert!(!ConnectionStatus::Failed.is_active());
        assert!(!ConnectionStatus::Timeout.is_active());

        assert!(ConnectionStatus::Connecting.is_established() == false);
        assert!(ConnectionStatus::Connected.is_established());

        assert!(ConnectionStatus::Disconnected.is_closed());
        assert!(ConnectionStatus::Failed.is_closed());
        assert!(ConnectionStatus::Timeout.is_closed());
        assert!(!ConnectionStatus::Connected.is_closed());

        assert!(ConnectionStatus::Failed.is_error());
        assert!(ConnectionStatus::Timeout.is_error());
        assert!(!ConnectionStatus::Disconnected.is_error());

        assert!(ConnectionStatus::Disconnected.has_ended());
        assert!(ConnectionStatus::Failed.has_ended());
        assert!(!ConnectionStatus::Connecting.has_ended());
    }

    #[test]
    fn test_connection_status_from_str() {
        assert!(matches!(
            ConnectionStatus::from_status_str("connecting"),
            ConnectionStatus::Connecting
        ));
        assert!(matches!(
            ConnectionStatus::from_status_str("connected"),
            ConnectionStatus::Connected
        ));
        assert!(matches!(
            ConnectionStatus::from_status_str("disconnected"),
            ConnectionStatus::Disconnected
        ));
        assert!(matches!(
            ConnectionStatus::from_status_str("failed"),
            ConnectionStatus::Failed
        ));
        assert!(matches!(
            ConnectionStatus::from_status_str("timeout"),
            ConnectionStatus::Timeout
        ));
        assert!(matches!(
            ConnectionStatus::from_status_str("invalid"),
            ConnectionStatus::Connecting
        ));
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
    fn test_connection_record_command() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        assert_eq!(conn.commands_executed, 0);
        conn.record_command();
        assert_eq!(conn.commands_executed, 1);
        conn.record_command();
        assert_eq!(conn.commands_executed, 2);
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
    fn test_connection_validation_empty_server_id() {
        let conn = Connection::new(
            "".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        let result = conn.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("server_id"));
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
        let result = conn.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("host"));
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
        let result = conn.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("port"));
    }

    #[test]
    fn test_connection_validation_empty_username() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "".to_string(),
            "agent".to_string(),
        );
        let result = conn.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("username"));
    }

    #[test]
    fn test_connection_validation_invalid_time_order() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        // Set ended_at before started_at
        conn.ended_at = Some(conn.started_at - Duration::seconds(60));

        let result = conn.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("ended_at"));
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
        assert!(conn.has_tag("production"));
        assert!(conn.tags.contains(&"production".to_string()));
        assert_eq!(conn.tags.len(), 2);

        // Adding duplicate should not increase count
        conn.add_tag("production".to_string());
        assert_eq!(conn.tags.len(), 2);

        // Remove tag
        conn.remove_tag("production");
        assert!(!conn.has_tag("production"));
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

        // Remove metadata
        let removed = conn.remove_metadata("client_version");
        assert_eq!(removed, Some("1.0.0".to_string()));
        assert_eq!(conn.get_metadata("client_version"), None);
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
        assert!(summary.contains("connecting"));
    }

    #[test]
    fn test_connection_display_label() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Production".to_string(),
            "example.com".to_string(),
            22,
            "root".to_string(),
            "key".to_string(),
        );

        let label = conn.display_label();
        assert!(label.contains("Production"));
        assert!(label.contains("root@example.com:22"));
    }

    #[test]
    fn test_connection_clone_redacted() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "key".to_string(),
        );
        conn.set_metadata("password".to_string(), "secret123".to_string());
        conn.set_metadata("passphrase".to_string(), "myphrase".to_string());

        let redacted = conn.clone_redacted();
        assert_eq!(redacted.get_metadata("password"), Some(&"***".to_string()));
        assert_eq!(
            redacted.get_metadata("passphrase"),
            Some(&"***".to_string())
        );
    }

    #[test]
    fn test_connection_history() {
        let mut history = ConnectionHistory::new();
        assert_eq!(history.total_connections, 0);
        assert_eq!(history.success_rate(), 0.0);
        assert_eq!(history.average_duration(), 0);

        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        conn.mark_connected(); // Mark as successful

        history.record_connection(&conn);
        assert_eq!(history.total_connections, 1);
        assert!(history.last_connection.is_some());
        assert_eq!(history.successful_connections, 1);

        // Record a failed connection
        let mut failed_conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        failed_conn.mark_failed("Auth failed".to_string(), None);
        history.record_connection(&failed_conn);

        assert_eq!(history.total_connections, 2);
        assert_eq!(history.successful_connections, 1);
        assert_eq!(history.failed_connections, 1);
        assert_eq!(history.success_rate(), 50.0);
    }

    #[test]
    fn test_connection_history_favorite_server() {
        let mut history = ConnectionHistory::new();

        // Connect to srv-123 twice
        for _ in 0..2 {
            let conn = Connection::new(
                "srv-123".to_string(),
                "Server 1".to_string(),
                "192.168.1.1".to_string(),
                22,
                "root".to_string(),
                "agent".to_string(),
            );
            history.record_connection(&conn);
        }

        // Connect to srv-456 once
        let conn = Connection::new(
            "srv-456".to_string(),
            "Server 2".to_string(),
            "192.168.1.2".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        history.record_connection(&conn);

        assert_eq!(history.favorite_server_id, Some("srv-123".to_string()));
    }

    #[test]
    fn test_connection_history_clear() {
        let mut history = ConnectionHistory::new();

        let conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        history.record_connection(&conn);

        history.clear();
        assert_eq!(history.total_connections, 0);
        assert!(history.last_connection.is_none());
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

        // Filter by server
        let filter = ConnectionFilter::new()
            .by_server("srv-123")
            .search("192.168");
        assert!(filter.matches(&conn));
        assert!(!filter.is_empty());

        let no_match = ConnectionFilter::new().by_server("srv-999");
        assert!(!no_match.matches(&conn));

        // Empty filter matches everything
        let empty = ConnectionFilter::new();
        assert!(empty.matches(&conn));
        assert!(empty.is_empty());
    }

    #[test]
    fn test_connection_filter_by_status() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        conn.mark_connected();
        let filter = ConnectionFilter::new().by_status(ConnectionStatus::Connected);
        assert!(filter.matches(&conn));

        let wrong_status = ConnectionFilter::new().by_status(ConnectionStatus::Failed);
        assert!(!wrong_status.matches(&conn));
    }

    #[test]
    fn test_connection_filter_by_success() {
        let mut successful = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        successful.mark_disconnected();

        let mut failed = Connection::new(
            "srv-456".to_string(),
            "Test".to_string(),
            "192.168.1.2".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        failed.mark_failed("Error".to_string(), None);

        let success_filter = ConnectionFilter::new().successful_only();
        assert!(success_filter.matches(&successful));
        assert!(!success_filter.matches(&failed));

        let failed_filter = ConnectionFilter::new().failed_only();
        assert!(!failed_filter.matches(&successful));
        assert!(failed_filter.matches(&failed));
    }

    #[test]
    fn test_connection_filter_by_tags() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        conn.add_tag("production".to_string());
        conn.add_tag("critical".to_string());

        let tag_filter = ConnectionFilter::new().with_tag("production");
        assert!(tag_filter.matches(&conn));

        let multi_tag = ConnectionFilter::new()
            .with_tag("production")
            .with_tag("nonexistent");
        assert!(multi_tag.matches(&conn)); // Matches if ANY tag matches

        let no_match = ConnectionFilter::new().with_tag("nonexistent");
        assert!(!no_match.matches(&conn));
    }

    #[test]
    fn test_connection_filter_by_duration() {
        let mut conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );
        conn.duration_seconds = Some(60);

        let in_range = ConnectionFilter::new().duration_range(Some(30), Some(120));
        assert!(in_range.matches(&conn));

        let too_short = ConnectionFilter::new().duration_range(Some(120), None);
        assert!(!too_short.matches(&conn));

        let too_long = ConnectionFilter::new().duration_range(None, Some(30));
        assert!(!too_long.matches(&conn));
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
    fn test_connection_with_id() {
        let created_at = Utc::now();
        let ended_at = Some(created_at + Duration::seconds(60));

        let conn = Connection::with_id(
            "specific-id".to_string(),
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            Some("user-456".to_string()),
            ConnectionStatus::Disconnected,
            created_at,
            ended_at,
            "agent".to_string(),
        );

        assert_eq!(conn.id, "specific-id");
        assert_eq!(conn.user_id, Some("user-456".to_string()));
        assert_eq!(conn.duration_seconds, Some(60));
    }

    #[test]
    fn test_create_connection_dto() {
        let json = r##"{
            "server_id": "srv-123",
            "server_name": "Test",
            "host": "192.168.1.1",
            "port": 2222,
            "username": "admin",
            "auth_method": "key",
            "user_id": "user-456"
        }"##;

        let dto: CreateConnectionDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.server_id, "srv-123");
        assert_eq!(dto.port, 2222);
        assert_eq!(dto.user_id, Some("user-456".to_string()));
    }

    #[test]
    fn test_create_connection_dto_defaults() {
        let json = r##"{
            "server_id": "srv-123",
            "server_name": "Test",
            "host": "192.168.1.1",
            "username": "root",
            "auth_method": "agent"
        }"##;

        let dto: CreateConnectionDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.port, 22); // Default port
    }

    #[test]
    fn test_update_connection_dto() {
        let json = r##"{
            "status": "connected",
            "bytes_sent": 1024,
            "bytes_received": 2048,
            "commands_executed": 5
        }"##;

        let dto: UpdateConnectionDto = serde_json::from_str(json).unwrap();
        assert!(matches!(dto.status, Some(ConnectionStatus::Connected)));
        assert_eq!(dto.bytes_sent, Some(1024));
        assert_eq!(dto.bytes_received, Some(2048));
        assert_eq!(dto.commands_executed, Some(5));
    }

    #[test]
    fn test_connection_record() {
        let conn = Connection::new(
            "srv-123".to_string(),
            "Test".to_string(),
            "192.168.1.1".to_string(),
            22,
            "root".to_string(),
            "agent".to_string(),
        );

        let record = ConnectionRecord::from(&conn);
        assert_eq!(record.id, conn.id);
        assert_eq!(record.server_id, conn.server_id);
        assert_eq!(record.status, "connecting");
    }
}
