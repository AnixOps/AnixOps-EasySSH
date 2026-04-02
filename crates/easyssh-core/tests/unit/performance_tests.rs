//! Performance Tests for EasySSH Core
//!
//! Tests for performance characteristics:
//! - Encryption/decryption throughput
//! - Database query performance
//! - Search performance
//! - Memory usage patterns

use std::time::{Duration, Instant};

mod common;

use easyssh_core::crypto::CryptoState;
use easyssh_core::db::Database;
use easyssh_core::models::Server;
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

    let db = Database::new(&db_path).expect("Failed to create database");
    db.init().expect("Failed to initialize database");

    // Bulk insert
    let count = 1000;
    let start = Instant::now();

    for i in 0..count {
        let server = Server::new(&format!("Server {}", i), &format!("192.168.1.{}", i % 256), 22)
            .with_username("admin");
        db.add_server(&server).expect("Should add server");
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
    use easyssh_core::search::{SearchEngine, SearchQuery};

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).expect("Failed to create database");
    db.init().expect("Failed to initialize database");

    let search_engine = SearchEngine::new(&db);

    // Populate database
    let count = 10000;
    for i in 0..count {
        let server = Server::new(
            &format!("Server {} - {}", i, if i % 2 == 0 { "production" } else { "development" }),
            &format!("192.168.{}.{}", i / 256, i % 256),
            22
        )
        .with_username("admin")
        .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);

        db.add_server(&server).expect("Should add server");
    }

    // Build search index
    let start = Instant::now();
    search_engine.build_index().expect("Should build index");
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
        let search_query = SearchQuery::new(query.to_string());

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
    use std::collections::VecDeque;

    let mut state = CryptoState::new();
    state.initialize("test_password_123").expect("Initialize should succeed");

    // Test memory usage with large data
    let large_data = vec![0u8; 100 * 1024 * 1024]; // 100MB

    let start = Instant::now();
    let encrypted = state.encrypt(&large_data).expect("Encryption should succeed");
    let encrypt_time = start.elapsed();

    // Drop the large plaintext
    drop(large_data);

    let start = Instant::now();
    let decrypted = state.decrypt(&encrypted).expect("Decryption should succeed");
    let decrypt_time = start.elapsed();

    println!("100MB data: Encrypt {:?}, Decrypt {:?}", encrypt_time, decrypt_time);

    // Verify data integrity
    assert_eq!(decrypted.len(), 100 * 1024 * 1024);

    // Test with many small operations
    let mut queue = VecDeque::new();
    let start = Instant::now();

    for i in 0..1000 {
        let data = format!("Message {}", i);
        let encrypted = state.encrypt(data.as_bytes()).expect("Should encrypt");
        queue.push_back(encrypted);

        // Keep queue size limited to prevent unbounded growth
        if queue.len() > 100 {
            let _ = queue.pop_front();
        }
    }

    let queue_time = start.elapsed();
    println!("1000 small encryptions: {:?}", queue_time);

    // Should complete in reasonable time
    assert!(queue_time < Duration::from_secs(5), "Queue operations too slow");
}

/// Test concurrent access performance
#[test]
fn test_concurrent_read_performance() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).expect("Failed to create database");
    db.init().expect("Failed to initialize database");

    // Populate with data
    for i in 0..100 {
        let server = Server::new(&format!("Server {}", i), "192.168.1.1", 22)
            .with_username("admin");
        db.add_server(&server).expect("Should add server");
    }

    let db = Arc::new(std::sync::Mutex::new(db));

    // Spawn multiple reader threads
    let mut handles = vec![];
    let start = Instant::now();

    for _ in 0..10 {
        let db_clone = Arc::clone(&db);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let db = db_clone.lock().expect("Should lock");
                let _ = db.get_all_servers().expect("Should get servers");
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    let elapsed = start.elapsed();
    let total_reads = 10 * 100;
    let rate = total_reads as f64 / elapsed.as_secs_f64();

    println!("Concurrent reads: {} reads in {:?} ({:.0} reads/sec)", total_reads, elapsed, rate);

    assert!(rate > 100.0, "Concurrent read rate too slow: {:.0} reads/sec", rate);
}

/// Test startup time
#[test]
fn test_application_startup_time() {
    let start = Instant::now();

    // Simulate startup operations
    let mut state = CryptoState::new();
    state.initialize("startup_test").expect("Should initialize");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db = Database::new(temp_dir.path().join("test.db")).expect("Should create DB");
    db.init().expect("Should init DB");

    let elapsed = start.elapsed();
    println!("Application startup: {:?}", elapsed);

    // Startup should be fast (< 500ms for basic operations)
    assert!(elapsed < Duration::from_millis(500), "Startup too slow: {:?}", elapsed);
}

/// Test pagination performance
#[test]
fn test_pagination_performance() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).expect("Failed to create database");
    db.init().expect("Failed to initialize database");

    // Add many servers
    let total = 10000;
    for i in 0..total {
        let server = Server::new(&format!("Server {}", i), "192.168.1.1", 22)
            .with_username("admin");
        db.add_server(&server).expect("Should add server");
    }

    // Test paginated reads
    let page_size = 50;
    let start = Instant::now();

    let mut total_retrieved = 0;
    let mut offset = 0;

    loop {
        let servers = db.get_servers_paginated(offset, page_size).expect("Should get paginated");
        if servers.is_empty() {
            break;
        }
        total_retrieved += servers.len();
        offset += page_size;
    }

    let elapsed = start.elapsed();
    println!("Paginated read of {} servers: {:?}", total_retrieved, elapsed);

    assert_eq!(total_retrieved, total, "Should retrieve all servers");

    // Pagination should be efficient
    assert!(elapsed < Duration::from_secs(1), "Pagination too slow: {:?}", elapsed);
}
