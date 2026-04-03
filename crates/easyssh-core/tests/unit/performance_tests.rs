//! Performance Tests for EasySSH Core
//!
//! Tests for performance characteristics:
//! - Encryption/decryption throughput
//! - Database query performance
//! - Search performance
//! - Memory usage patterns

use std::time::{Duration, Instant};

#[path = "../common/mod.rs"]
mod common;

use easyssh_core::crypto::CryptoState;
use easyssh_core::db::Database;
use easyssh_core::models::{Server, AuthMethod};
use tempfile::TempDir;

/// Benchmark encryption throughput
#[test]
fn test_encryption_throughput() {
    let mut state = CryptoState::new();
    state.initialize("test_password_123").expect("Initialize should succeed");

    let sizes = vec![
        (1024, "1KB"),
        (1024 * 1024, "1MB"),
        (10 * 1024 * 1024, "10MB"),
    ];

    for (size, label) in sizes {
        let data = vec![0u8; size];

        let start = Instant::now();
        let encrypted = state.encrypt(&data).expect("Encryption should succeed");
        let encrypt_time = start.elapsed();

        let start = Instant::now();
        let _ = state.decrypt(&encrypted).expect("Decryption should succeed");
        let decrypt_time = start.elapsed();

        let encrypt_throughput = size as f64 / 1024.0 / 1024.0 / encrypt_time.as_secs_f64();
        let decrypt_throughput = size as f64 / 1024.0 / 1024.0 / decrypt_time.as_secs_f64();

        println!(
            "{}: Encrypt: {:.2} MB/s, Decrypt: {:.2} MB/s",
            label, encrypt_throughput, decrypt_throughput
        );

        // Sanity check: should be able to encrypt/decrypt at least 1 MB/s
        assert!(encrypt_throughput > 1.0, "Encryption too slow for {}", label);
        assert!(decrypt_throughput > 1.0, "Decryption too slow for {}", label);
    }
}

/// Test database bulk operations performance
#[test]
fn test_database_bulk_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(db_path).expect("Failed to create database");
    db.init().expect("Failed to initialize database");

    // Bulk insert
    let count = 1000;
    let start = Instant::now();

    for i in 0..count {
        let server = easyssh_core::db::NewServer {
            id: format!("srv-{}", i),
            name: format!("Server {}", i),
            host: format!("192.168.1.{}", i % 256),
            port: 22,
            username: "admin",
            auth_type: "agent",
            identity_file: None,
            password_encrypted: None,
            group_id: None,
            status: "unknown",
        };
        db.create_server(&server).expect("Should add server");
    }

    let insert_time = start.elapsed();
    let insert_rate = count as f64 / insert_time.as_secs_f64();
    println!("Bulk insert: {} servers in {:?} ({:.0} ops/sec)", count, insert_time, insert_rate);

    // Bulk read
    let start = Instant::now();
    let all_servers = db.get_all_servers().expect("Should get all servers");
    let read_time = start.elapsed();
    let read_rate = count as f64 / read_time.as_secs_f64();
    println!("Bulk read: {} servers in {:?} ({:.0} ops/sec)", count, read_time, read_rate);

    assert_eq!(all_servers.len(), count, "Should retrieve all servers");

    // Performance assertions
    assert!(insert_rate > 100.0, "Insert rate too slow: {:.0} ops/sec", insert_rate);
    assert!(read_rate > 1000.0, "Read rate too slow: {:.0} ops/sec", read_rate);
}

/// Test search performance with large dataset
#[test]
fn test_search_performance() {
    use easyssh_core::services::search_service::{SearchService, SearchQuery};

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(db_path).expect("Failed to create database");
    db.init().expect("Failed to initialize database");

    let search_engine = SearchService::new(std::sync::Arc::new(db.clone())).expect("Should create search service");

    // Populate database
    let count = 1000;
    for i in 0..count {
        let server = easyssh_core::db::NewServer {
            id: format!("srv-{}", i),
            name: format!("Server {} - {}", i, if i % 2 == 0 { "production" } else { "development" }),
            host: format!("192.168.{}.{}", i / 256, i % 256),
            port: 22,
            username: "admin",
            auth_type: "agent",
            identity_file: None,
            password_encrypted: None,
            group_id: None,
            status: "unknown",
        };

        db.create_server(&server).expect("Should add server");
    }

    // Build search index
    let start = Instant::now();
    search_engine.rebuild_index().expect("Should build index");
    let index_time = start.elapsed();
    println!("Search index built in {:?}", index_time);

    // Test search performance
    let queries = vec![
        "production",
        "Server 500",
        "192.168",
        "admin",
    ];

    for query in &queries {
        let search_query = SearchQuery {
            keyword: Some(query.to_string()),
            ..Default::default()
        };

        let start = Instant::now();
        let results = search_engine.search(&search_query).expect("Should search");
        let search_time = start.elapsed();

        println!("Search '{}' found {} results in {:?}", query, results.len(), search_time);

        // Search should be fast (< 100ms for this dataset)
        assert!(search_time < Duration::from_millis(100), "Search too slow for '{}'", query);
    }
}

/// Test memory usage during operations
#[test]
fn test_memory_usage_patterns() {
    // This test would require actual memory profiling tools
    // For now, we just ensure operations don't cause obvious memory issues

    let mut state = CryptoState::new();
    state.initialize("test_password").expect("Should initialize");

    // Process multiple large payloads
    for _ in 0..100 {
        let data = vec![0u8; 1024 * 1024]; // 1MB
        let encrypted = state.encrypt(&data).expect("Should encrypt");
        let decrypted = state.decrypt(&encrypted).expect("Should decrypt");
        assert_eq!(decrypted, data);
    }
}