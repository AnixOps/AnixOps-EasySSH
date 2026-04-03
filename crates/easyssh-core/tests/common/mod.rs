//! Test utilities for EasySSH Core tests
//!
//! Provides common utilities for unit and integration tests including:
//! - Temporary database creation
//! - Test data loading
//! - Mock object creation
//! - Async test helpers
//!
//! Note: Some functions may be unused in certain test configurations,
//! but are kept for completeness.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use easyssh_core::db::Database;
use serde_json::Value;
use tempfile::TempDir;

/// Load test fixtures from JSON file
pub fn load_test_fixtures() -> Value {
    let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_data.json");

    let content = std::fs::read_to_string(&fixtures_path).unwrap_or_else(|e| {
        panic!(
            "Failed to load test fixtures from {:?}: {}",
            fixtures_path, e
        )
    });

    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse test fixtures: {}", e))
}

/// Create a temporary database for testing
pub fn create_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(db_path).expect("Failed to create test database");
    db.init().expect("Failed to initialize database");

    (db, temp_dir)
}

/// Create an in-memory database for testing
pub fn create_in_memory_db() -> Database {
    let db = Database::new(PathBuf::from(":memory:")).expect("Failed to create in-memory database");
    db.init().expect("Failed to initialize in-memory database");
    db
}

/// Create a test database wrapped in Arc<Mutex<>> for service tests
pub fn create_test_db_arc() -> (Arc<Mutex<Database>>, TempDir) {
    let (db, temp_dir) = create_test_db();
    (Arc::new(Mutex::new(db)), temp_dir)
}

/// Create a test database wrapped in Arc (without Mutex) for SearchService tests
#[allow(clippy::arc_with_non_send_sync)]
pub fn create_test_db_arc_direct() -> (Arc<Database>, TempDir) {
    let (db, temp_dir) = create_test_db();
    (Arc::new(db), temp_dir)
}

/// Get test master password
pub fn test_master_password() -> &'static str {
    "TestMasterPass123!"
}

/// Get wrong password for testing error cases
pub fn test_wrong_password() -> &'static str {
    "WrongPass456!"
}

/// Get test encryption data
pub fn test_encryption_data() -> &'static [u8] {
    b"This is test data for encryption testing"
}

/// A test server fixture
#[derive(Debug, Clone)]
pub struct TestServerFixture {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub password: Option<String>,
    pub identity_file: Option<String>,
    pub group_id: Option<String>,
}

impl TestServerFixture {
    /// Load all server fixtures from test data
    pub fn load_all() -> Vec<Self> {
        let fixtures = load_test_fixtures();
        let servers = fixtures
            .get("servers")
            .expect("No servers in fixtures")
            .as_array()
            .expect("Servers not an array");

        servers
            .iter()
            .map(|s| Self {
                id: s
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: s
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                host: s
                    .get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                port: s.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16,
                username: s
                    .get("username")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                auth_type: s
                    .get("auth_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("agent")
                    .to_string(),
                password: s
                    .get("password")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                identity_file: s
                    .get("identity_file")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                group_id: s
                    .get("group_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
            .collect()
    }

    /// Get first server fixture
    pub fn first() -> Self {
        Self::load_all()
            .into_iter()
            .next()
            .expect("No server fixtures found")
    }
}

/// Async test runtime setup
#[cfg(test)]
pub mod async_test {
    use tokio::runtime::Runtime;

    /// Create a single-threaded runtime for tests
    pub fn create_runtime() -> Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    }
}

/// Assertion helpers
#[cfg(test)]
pub mod assertions {
    /// Assert that a result is an error containing specific text
    pub fn assert_error_contains<T>(result: Result<T, impl std::fmt::Display>, expected: &str) {
        match result {
            Ok(_) => panic!("Expected error containing '{}', but got Ok", expected),
            Err(e) => {
                let error_string = e.to_string();
                assert!(
                    error_string.contains(expected),
                    "Expected error containing '{}', but got: {}",
                    expected,
                    error_string
                );
            }
        }
    }

    /// Assert that two byte vectors are equal
    pub fn assert_bytes_eq(a: &[u8], b: &[u8]) {
        assert_eq!(a, b, "Byte vectors are not equal");
    }
}

/// Test setup and cleanup utilities
pub struct TestContext {
    pub temp_dirs: Vec<TempDir>,
}

/// Data generation utilities
pub mod data_generator;

impl TestContext {
    pub fn new() -> Self {
        Self {
            temp_dirs: Vec::new(),
        }
    }

    pub fn create_temp_dir(&mut self) -> PathBuf {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().to_path_buf();
        self.temp_dirs.push(temp_dir);
        path
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}
