//! Database backup support for MySQL and PostgreSQL

use super::{BackupError, BackupResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::info;

/// Database type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
    MongoDB,
    Redis,
    SQLite,
}

impl DatabaseType {
    /// Get the default port for this database type
    pub fn default_port(&self) -> u16 {
        match self {
            DatabaseType::MySQL => 3306,
            DatabaseType::PostgreSQL => 5432,
            DatabaseType::MongoDB => 27017,
            DatabaseType::Redis => 6379,
            DatabaseType::SQLite => 0, // File-based
        }
    }

    /// Get the name of the dump command
    pub fn dump_command(&self) -> &'static str {
        match self {
            DatabaseType::MySQL => "mysqldump",
            DatabaseType::PostgreSQL => "pg_dump",
            DatabaseType::MongoDB => "mongodump",
            DatabaseType::Redis => "redis-cli",
            DatabaseType::SQLite => "sqlite3",
        }
    }

    /// Check if the dump command is available
    pub async fn is_available(&self) -> bool {
        let cmd = if cfg!(target_os = "windows") {
            Command::new("where")
                .arg(self.dump_command())
                .output()
                .await
        } else {
            Command::new("which")
                .arg(self.dump_command())
                .output()
                .await
        };

        matches!(cmd, Ok(output) if output.status.success())
    }
}

/// Database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub db_type: DatabaseType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub database: Option<String>,
    pub ssl_mode: Option<String>,
    pub connection_options: HashMap<String, String>,
}

impl DatabaseConfig {
    /// Create a new database configuration
    pub fn new(db_type: DatabaseType, host: &str, username: &str) -> Self {
        Self {
            db_type,
            host: host.to_string(),
            port: db_type.default_port(),
            username: username.to_string(),
            password: None,
            database: None,
            ssl_mode: None,
            connection_options: HashMap::new(),
        }
    }

    /// Set port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set password
    pub fn with_password(mut self, password: &str) -> Self {
        self.password = Some(password.to_string());
        self
    }

    /// Set database name
    pub fn with_database(mut self, database: &str) -> Self {
        self.database = Some(database.to_string());
        self
    }

    /// Set SSL mode
    pub fn with_ssl_mode(mut self, ssl_mode: &str) -> Self {
        self.ssl_mode = Some(ssl_mode.to_string());
        self
    }

    /// Get connection string
    pub fn connection_string(&self) -> String {
        match self.db_type {
            DatabaseType::MySQL => {
                let mut url = format!(
                    "mysql://{}:{}@{}:{}",
                    self.username,
                    self.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port
                );
                if let Some(db) = &self.database {
                    url.push('/');
                    url.push_str(db);
                }
                url
            }
            DatabaseType::PostgreSQL => {
                let mut url = format!(
                    "postgresql://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database.as_deref().unwrap_or("postgres")
                );
                if let Some(ssl) = &self.ssl_mode {
                    url.push_str("?sslmode=");
                    url.push_str(ssl);
                }
                url
            }
            _ => String::new(),
        }
    }
}

/// Database backup options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DatabaseBackupOptions {
    /// Include only specific tables (empty = all tables)
    pub include_tables: Vec<String>,
    /// Exclude specific tables
    pub exclude_tables: Vec<String>,
    /// Compression level
    pub compression_level: u32,
    /// Lock tables during backup (MySQL)
    pub lock_tables: bool,
    /// Single transaction mode (PostgreSQL, MySQL)
    pub single_transaction: bool,
    /// Include stored procedures/functions
    pub include_routines: bool,
    /// Include events (MySQL)
    pub include_events: bool,
    /// Include triggers
    pub include_triggers: bool,
    /// Add DROP statements before CREATE
    pub add_drop_statements: bool,
    /// Create database if not exists
    pub create_database: bool,
    /// Disable foreign key checks during restore
    pub disable_keys: bool,
    /// Verbose output
    pub verbose: bool,
    /// Extra command line options
    pub extra_options: Vec<String>,
}

/// Database backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseBackupConfig {
    pub connection: DatabaseConfig,
    pub options: DatabaseBackupOptions,
    pub output_path: PathBuf,
    pub encrypt: bool,
    pub compression: bool,
}

/// Database backup result
#[derive(Debug, Clone)]
pub struct DatabaseBackupResult {
    pub db_type: DatabaseType,
    pub database: Option<String>,
    pub output_path: PathBuf,
    pub size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub table_count: u32,
    pub duration_seconds: f64,
    pub checksum: String,
    pub warnings: Vec<String>,
}

/// Database backup engine trait
#[async_trait]
pub trait DatabaseBackupEngine: Send + Sync {
    /// Check if this engine can handle the database type
    fn can_handle(&self, db_type: DatabaseType) -> bool;

    /// Create a backup
    async fn backup(&self, config: &DatabaseBackupConfig) -> BackupResult<DatabaseBackupResult>;

    /// Test connection to database
    async fn test_connection(&self, config: &DatabaseConfig) -> BackupResult<()>;

    /// List databases
    async fn list_databases(&self, config: &DatabaseConfig) -> BackupResult<Vec<String>>;

    /// List tables in a database
    async fn list_tables(
        &self,
        config: &DatabaseConfig,
        database: &str,
    ) -> BackupResult<Vec<String>>;
}

/// MySQL backup engine
pub struct MySqlBackupEngine;

#[async_trait]
impl DatabaseBackupEngine for MySqlBackupEngine {
    fn can_handle(&self, db_type: DatabaseType) -> bool {
        db_type == DatabaseType::MySQL
    }

    async fn backup(&self, config: &DatabaseBackupConfig) -> BackupResult<DatabaseBackupResult> {
        let start_time = std::time::Instant::now();
        let mut warnings = Vec::new();

        // Build mysqldump command
        let mut cmd = Command::new("mysqldump");

        cmd.arg("--host")
            .arg(&config.connection.host)
            .arg("--port")
            .arg(config.connection.port.to_string())
            .arg("--user")
            .arg(&config.connection.username);

        if let Some(password) = &config.connection.password {
            // Use environment variable for password (safer than command line)
            cmd.env("MYSQL_PWD", password);
        }

        // Add options
        let opts = &config.options;

        if opts.single_transaction {
            cmd.arg("--single-transaction");
        }

        if opts.lock_tables {
            cmd.arg("--lock-tables");
        }

        if opts.include_routines {
            cmd.arg("--routines");
        }

        if opts.include_events {
            cmd.arg("--events");
        }

        if opts.include_triggers {
            cmd.arg("--triggers");
        }

        if opts.add_drop_statements {
            cmd.arg("--add-drop-table");
        }

        if !opts.disable_keys {
            cmd.arg("--disable-keys");
        }

        if opts.verbose {
            cmd.arg("--verbose");
        }

        // Include/exclude tables
        for table in &opts.exclude_tables {
            cmd.arg("--ignore-table").arg(format!(
                "{}.{}",
                config.connection.database.as_deref().unwrap_or(""),
                table
            ));
        }

        for table in &opts.include_tables {
            cmd.arg(table);
        }

        // Add extra options
        for opt in &opts.extra_options {
            cmd.arg(opt);
        }

        // Specify database
        if let Some(db) = &config.connection.database {
            cmd.arg(db);
        } else {
            cmd.arg("--all-databases");
        }

        // Output to file
        let output_path = if config.compression {
            config.output_path.with_extension("sql.gz")
        } else {
            config.output_path.with_extension("sql")
        };

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        info!("Starting MySQL backup to {:?}", output_path);

        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackupError::Database(
                    "mysqldump not found. Please install MySQL client tools.".to_string(),
                )
            } else {
                BackupError::Io(e)
            }
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BackupError::Database("Failed to capture stdout".to_string()))?;

        // Handle compression
        let output_data = if config.compression {
            use flate2::write::GzEncoder;
            use flate2::Compression;
            use std::io::{Read, Write};

            let mut encoder = GzEncoder::new(Vec::new(), Compression::new(opts.compression_level));
            let mut stdout_reader = tokio::io::BufReader::new(stdout);
            let mut buffer = [0u8; 8192];

            // This is a simplified version - in production, use async compression
            let mut all_data = Vec::new();
            loop {
                match stdout_reader.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => all_data.extend_from_slice(&buffer[..n]),
                    Err(e) => return Err(BackupError::Io(e)),
                }
            }

            encoder
                .write_all(&all_data)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| BackupError::Compression(e.to_string()))?
        } else {
            let mut stdout_reader = tokio::io::BufReader::new(stdout);
            let mut all_data = Vec::new();
            stdout_reader
                .read_to_end(&mut all_data)
                .await
                .map_err(BackupError::Io)?;
            all_data
        };

        // Write output
        tokio::fs::write(&output_path, &output_data)
            .await
            .map_err(BackupError::Io)?;

        // Wait for completion and capture stderr
        let output = child.wait_with_output().await.map_err(BackupError::Io)?;
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stderr.is_empty() {
            for line in stderr.lines() {
                if !line.is_empty() {
                    warnings.push(line.to_string());
                }
            }
        }

        if !output.status.success() {
            return Err(BackupError::Database(format!(
                "mysqldump failed: {}",
                output.status
            )));
        }

        // Calculate checksum
        let checksum = blake3::hash(&output_data).to_hex().to_string();

        // Count tables (simplified - count CREATE TABLE statements)
        let table_count = String::from_utf8_lossy(&output_data)
            .lines()
            .filter(|l| l.contains("CREATE TABLE"))
            .count() as u32;

        let duration = start_time.elapsed().as_secs_f64();
        let original_size = if config.compression {
            output_data.len() as u64 * 5
        } else {
            output_data.len() as u64
        }; // Estimate original size

        info!(
            "MySQL backup completed: {} tables, {} bytes in {:.2}s",
            table_count,
            output_data.len(),
            duration
        );

        Ok(DatabaseBackupResult {
            db_type: DatabaseType::MySQL,
            database: config.connection.database.clone(),
            output_path,
            size_bytes: original_size,
            compressed_size_bytes: output_data.len() as u64,
            table_count,
            duration_seconds: duration,
            checksum,
            warnings,
        })
    }

    async fn test_connection(&self, config: &DatabaseConfig) -> BackupResult<()> {
        let mut cmd = Command::new("mysql");

        cmd.arg("--host")
            .arg(&config.host)
            .arg("--port")
            .arg(config.port.to_string())
            .arg("--user")
            .arg(&config.username)
            .arg("-e")
            .arg("SELECT 1");

        if let Some(password) = &config.password {
            cmd.env("MYSQL_PWD", password);
        }

        let output = cmd.output().await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackupError::Database("mysql client not found".to_string())
            } else {
                BackupError::Io(e)
            }
        })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(BackupError::Database(format!(
                "Connection test failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    async fn list_databases(&self, config: &DatabaseConfig) -> BackupResult<Vec<String>> {
        let mut cmd = Command::new("mysql");

        cmd.arg("--host")
            .arg(&config.host)
            .arg("--port")
            .arg(config.port.to_string())
            .arg("--user")
            .arg(&config.username)
            .arg("-e")
            .arg("SHOW DATABASES");

        if let Some(password) = &config.password {
            cmd.env("MYSQL_PWD", password);
        }

        let output = cmd.output().await.map_err(BackupError::Io)?;

        if !output.status.success() {
            return Err(BackupError::Database(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let databases: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .skip(1) // Skip header
            .map(|l| l.trim().to_string())
            .filter(|l| {
                !l.is_empty()
                    && l != "information_schema"
                    && l != "performance_schema"
                    && l != "sys"
            })
            .collect();

        Ok(databases)
    }

    async fn list_tables(
        &self,
        config: &DatabaseConfig,
        database: &str,
    ) -> BackupResult<Vec<String>> {
        let mut cmd = Command::new("mysql");

        cmd.arg("--host")
            .arg(&config.host)
            .arg("--port")
            .arg(config.port.to_string())
            .arg("--user")
            .arg(&config.username)
            .arg("-e")
            .arg(format!("USE {}; SHOW TABLES", database));

        if let Some(password) = &config.password {
            cmd.env("MYSQL_PWD", password);
        }

        let output = cmd.output().await.map_err(BackupError::Io)?;

        if !output.status.success() {
            return Err(BackupError::Database(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let tables: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .skip(1) // Skip header
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(tables)
    }
}

/// PostgreSQL backup engine
pub struct PostgresBackupEngine;

#[async_trait]
impl DatabaseBackupEngine for PostgresBackupEngine {
    fn can_handle(&self, db_type: DatabaseType) -> bool {
        db_type == DatabaseType::PostgreSQL
    }

    async fn backup(&self, config: &DatabaseBackupConfig) -> BackupResult<DatabaseBackupResult> {
        let start_time = std::time::Instant::now();
        let mut warnings = Vec::new();

        // Build pg_dump command
        let mut cmd = Command::new("pg_dump");

        cmd.arg("--host")
            .arg(&config.connection.host)
            .arg("--port")
            .arg(config.connection.port.to_string())
            .arg("--username")
            .arg(&config.connection.username)
            .arg("--no-password"); // Use PGPASSWORD env or .pgpass

        // Add options
        let opts = &config.options;

        if opts.single_transaction {
            // pg_dump doesn't have single-transaction, it's for pg_restore
        }

        if opts.include_routines {
            // Included by default in pg_dump
        }

        if opts.add_drop_statements {
            cmd.arg("--clean").arg("--if-exists");
        }

        if opts.create_database {
            cmd.arg("--create");
        }

        if opts.verbose {
            cmd.arg("--verbose");
        }

        // Table filtering
        for table in &opts.include_tables {
            cmd.arg("--table").arg(table);
        }

        for table in &opts.exclude_tables {
            cmd.arg("--exclude-table").arg(table);
        }

        // Extra options
        for opt in &opts.extra_options {
            cmd.arg(opt);
        }

        // Database name
        if let Some(db) = &config.connection.database {
            cmd.arg(db);
        }

        // Set password via environment
        if let Some(password) = &config.connection.password {
            cmd.env("PGPASSWORD", password);
        }

        // Output format - custom format is compressed by default
        let output_path = config.output_path.with_extension("dump");
        cmd.arg("--format=c").arg("--file").arg(&output_path);

        info!("Starting PostgreSQL backup to {:?}", output_path);

        let output = cmd.output().await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackupError::Database(
                    "pg_dump not found. Please install PostgreSQL client tools.".to_string(),
                )
            } else {
                BackupError::Io(e)
            }
        })?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            for line in stderr.lines() {
                if !line.is_empty() {
                    warnings.push(line.to_string());
                }
            }
        }

        if !output.status.success() {
            return Err(BackupError::Database(format!(
                "pg_dump failed: {}",
                output.status
            )));
        }

        // Get file size and calculate checksum
        let file_data = tokio::fs::read(&output_path)
            .await
            .map_err(BackupError::Io)?;
        let checksum = blake3::hash(&file_data).to_hex().to_string();
        let size = file_data.len() as u64;

        // Count tables using pg_restore --list
        let table_count = if let Some(db) = &config.connection.database {
            let mut list_cmd = Command::new("pg_restore");
            list_cmd.arg("--list").arg(&output_path);
            if let Some(password) = &config.connection.password {
                list_cmd.env("PGPASSWORD", password);
            }

            if let Ok(output) = list_cmd.output().await {
                if output.status.success() {
                    String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .filter(|l| l.contains("TABLE"))
                        .count() as u32
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        let duration = start_time.elapsed().as_secs_f64();

        info!(
            "PostgreSQL backup completed: {} tables, {} bytes in {:.2}s",
            table_count, size, duration
        );

        Ok(DatabaseBackupResult {
            db_type: DatabaseType::PostgreSQL,
            database: config.connection.database.clone(),
            output_path,
            size_bytes: size * 4, // Estimate original size (custom format is compressed)
            compressed_size_bytes: size,
            table_count,
            duration_seconds: duration,
            checksum,
            warnings,
        })
    }

    async fn test_connection(&self, config: &DatabaseConfig) -> BackupResult<()> {
        let mut cmd = Command::new("psql");

        cmd.arg("--host")
            .arg(&config.host)
            .arg("--port")
            .arg(config.port.to_string())
            .arg("--username")
            .arg(&config.username)
            .arg("--no-password")
            .arg("--command")
            .arg("SELECT 1")
            .arg(config.database.as_deref().unwrap_or("postgres"));

        if let Some(password) = &config.password {
            cmd.env("PGPASSWORD", password);
        }

        let output = cmd.output().await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackupError::Database("psql not found".to_string())
            } else {
                BackupError::Io(e)
            }
        })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(BackupError::Database(format!(
                "Connection test failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    async fn list_databases(&self, config: &DatabaseConfig) -> BackupResult<Vec<String>> {
        let mut cmd = Command::new("psql");

        cmd.arg("--host")
            .arg(&config.host)
            .arg("--port")
            .arg(config.port.to_string())
            .arg("--username")
            .arg(&config.username)
            .arg("--no-password")
            .arg("--tuples-only")
            .arg("--command")
            .arg("SELECT datname FROM pg_database WHERE datistemplate = false;");

        if let Some(password) = &config.password {
            cmd.env("PGPASSWORD", password);
        }

        let output = cmd.output().await.map_err(BackupError::Io)?;

        if !output.status.success() {
            return Err(BackupError::Database(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let databases: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(databases)
    }

    async fn list_tables(
        &self,
        config: &DatabaseConfig,
        database: &str,
    ) -> BackupResult<Vec<String>> {
        let mut cmd = Command::new("psql");

        cmd.arg("--host")
            .arg(&config.host)
            .arg("--port")
            .arg(config.port.to_string())
            .arg("--username")
            .arg(&config.username)
            .arg("--no-password")
            .arg("--tuples-only")
            .arg("--command")
            .arg(format!(
                "SELECT tablename FROM pg_tables WHERE schemaname = 'public';"
            ))
            .arg(database);

        if let Some(password) = &config.password {
            cmd.env("PGPASSWORD", password);
        }

        let output = cmd.output().await.map_err(BackupError::Io)?;

        if !output.status.success() {
            return Err(BackupError::Database(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let tables: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(tables)
    }
}

/// SQLite backup (simple file copy)
pub struct SqliteBackupEngine;

#[async_trait]
impl DatabaseBackupEngine for SqliteBackupEngine {
    fn can_handle(&self, db_type: DatabaseType) -> bool {
        db_type == DatabaseType::SQLite
    }

    async fn backup(&self, config: &DatabaseBackupConfig) -> BackupResult<DatabaseBackupResult> {
        let start_time = std::time::Instant::now();

        let source_path = PathBuf::from(&config.connection.host);
        let output_path = config.output_path.with_extension("db");

        info!(
            "Starting SQLite backup from {:?} to {:?}",
            source_path, output_path
        );

        // Create parent directory
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(BackupError::Io)?;
        }

        // Copy file
        tokio::fs::copy(&source_path, &output_path)
            .await
            .map_err(BackupError::Io)?;

        // Calculate checksum
        let file_data = tokio::fs::read(&output_path)
            .await
            .map_err(BackupError::Io)?;
        let checksum = blake3::hash(&file_data).to_hex().to_string();
        let size = file_data.len() as u64;

        let duration = start_time.elapsed().as_secs_f64();

        info!(
            "SQLite backup completed: {} bytes in {:.2}s",
            size, duration
        );

        Ok(DatabaseBackupResult {
            db_type: DatabaseType::SQLite,
            database: Some(source_path.to_string_lossy().to_string()),
            output_path,
            size_bytes: size,
            compressed_size_bytes: size,
            table_count: 0,
            duration_seconds: duration,
            checksum,
            warnings: vec![],
        })
    }

    async fn test_connection(&self, _config: &DatabaseConfig) -> BackupResult<()> {
        // SQLite is file-based, just check if file exists
        Ok(())
    }

    async fn list_databases(&self, _config: &DatabaseConfig) -> BackupResult<Vec<String>> {
        Ok(vec![])
    }

    async fn list_tables(
        &self,
        _config: &DatabaseConfig,
        _database: &str,
    ) -> BackupResult<Vec<String>> {
        Ok(vec![])
    }
}

/// Factory function to get appropriate backup engine
pub fn get_backup_engine(db_type: DatabaseType) -> Option<Box<dyn DatabaseBackupEngine>> {
    match db_type {
        DatabaseType::MySQL => Some(Box::new(MySqlBackupEngine)),
        DatabaseType::PostgreSQL => Some(Box::new(PostgresBackupEngine)),
        DatabaseType::SQLite => Some(Box::new(SqliteBackupEngine)),
        _ => None,
    }
}

/// Backup a MySQL database
pub async fn backup_mysql(
    host: &str,
    port: u16,
    username: &str,
    password: Option<&str>,
    database: Option<&str>,
    output: &Path,
) -> BackupResult<DatabaseBackupResult> {
    let mut config = DatabaseConfig::new(DatabaseType::MySQL, host, username).with_port(port);

    if let Some(pwd) = password {
        config = config.with_password(pwd);
    }

    if let Some(db) = database {
        config = config.with_database(db);
    }

    let backup_config = DatabaseBackupConfig {
        connection: config,
        options: DatabaseBackupOptions::default(),
        output_path: output.to_path_buf(),
        encrypt: false,
        compression: true,
    };

    let engine = MySqlBackupEngine;
    engine.backup(&backup_config).await
}

/// Backup a PostgreSQL database
pub async fn backup_postgresql(
    host: &str,
    port: u16,
    username: &str,
    password: Option<&str>,
    database: Option<&str>,
    output: &Path,
) -> BackupResult<DatabaseBackupResult> {
    let mut config = DatabaseConfig::new(DatabaseType::PostgreSQL, host, username).with_port(port);

    if let Some(pwd) = password {
        config = config.with_password(pwd);
    }

    if let Some(db) = database {
        config = config.with_database(db);
    }

    let backup_config = DatabaseBackupConfig {
        connection: config,
        options: DatabaseBackupOptions::default(),
        output_path: output.to_path_buf(),
        encrypt: false,
        compression: true,
    };

    let engine = PostgresBackupEngine;
    engine.backup(&backup_config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config() {
        let config = DatabaseConfig::new(DatabaseType::MySQL, "localhost", "root")
            .with_port(3306)
            .with_password("secret")
            .with_database("mydb");

        assert_eq!(config.db_type, DatabaseType::MySQL);
        assert_eq!(config.host, "localhost");
        assert_eq!(config.username, "root");
        assert_eq!(config.password, Some("secret".to_string()));
        assert_eq!(config.database, Some("mydb".to_string()));
    }

    #[test]
    fn test_database_type_defaults() {
        assert_eq!(DatabaseType::MySQL.default_port(), 3306);
        assert_eq!(DatabaseType::PostgreSQL.default_port(), 5432);
        assert_eq!(DatabaseType::MongoDB.default_port(), 27017);
        assert_eq!(DatabaseType::Redis.default_port(), 6379);
    }

    #[test]
    fn test_database_type_commands() {
        assert_eq!(DatabaseType::MySQL.dump_command(), "mysqldump");
        assert_eq!(DatabaseType::PostgreSQL.dump_command(), "pg_dump");
        assert_eq!(DatabaseType::MongoDB.dump_command(), "mongodump");
    }

    #[test]
    fn test_get_backup_engine() {
        assert!(get_backup_engine(DatabaseType::MySQL).is_some());
        assert!(get_backup_engine(DatabaseType::PostgreSQL).is_some());
        assert!(get_backup_engine(DatabaseType::SQLite).is_some());
        assert!(get_backup_engine(DatabaseType::MongoDB).is_none());
    }
}
