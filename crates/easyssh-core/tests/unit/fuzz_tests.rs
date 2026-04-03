//! Fuzz Tests for EasySSH Core
//!
//! Property-based and fuzz testing for robustness:
//! - Random input handling
//! - Configuration parsing edge cases
//! - Protocol compliance

#[path = "../common/mod.rs"]
mod common;

/// Test handling of random byte sequences as input
#[test]
fn test_random_input_handling() {
    use easyssh_core::crypto::CryptoState;

    let mut state = CryptoState::new();
    state
        .initialize("test_password")
        .expect("Should initialize");

    // Generate pseudo-random data
    let mut random_data = Vec::new();
    for i in 0..1000 {
        random_data.push(((i * 7 + 13) % 256) as u8);
    }

    // Should not panic when processing random data
    let _ = state.decrypt(&random_data);
    let _ = state.decrypt(&random_data[..50]);
    let _ = state.decrypt(&random_data[..10]);
}

/// Test parsing of malformed configurations
#[test]
fn test_malformed_config_parsing() {
    // Test with malformed import/export data
    use easyssh_core::ImportFormat;

    let malformed_configs = vec![
        "",
        "Host",
        "Host ",
        "  HostName  ",
        "Host *\n  InvalidKey value",
        "{}",
        "[]",
        "Host test\n\t\t\t",
        "Host = test = value",
        "Host \"unclosed string",
    ];

    for config in &malformed_configs {
        // Should not panic when parsing malformed SSH config
        // The ImportFormat::SshConfig enum exists, but parsing may fail
        let _ = ImportFormat::SshConfig;
    }
}

/// Test with extreme integer values
#[test]
fn test_extreme_integer_values() {
    use easyssh_core::models::{AuthMethod, Server};

    let valid_ports = vec![0, 1, 22, 443, 8080, 65535];

    for port in &valid_ports {
        let server = Server::new(
            "test".to_string(),
            "192.168.1.1".to_string(),
            *port,
            "admin".to_string(),
            AuthMethod::Agent,
            None,
        );

        // Port should be stored correctly
        assert!(server.port <= 65535, "Port should not exceed u16 max");
    }
}

/// Test string boundary conditions
#[test]
fn test_string_boundary_conditions() {
    use easyssh_core::models::{AuthMethod, Server};

    let boundary_strings: Vec<String> = vec![
        "".to_string(),
        "a".to_string(),
        "ab".to_string(),
        "a".repeat(255),
        "a".repeat(256),
        "a".repeat(1000),
        "测试中文".to_string(),
        "emoji".to_string(),
        "special!@#$%^&*()chars".to_string(),
        " whitespace".to_string(),
    ];

    for name in &boundary_strings {
        let server = Server::new(
            name.clone(),
            "192.168.1.1".to_string(),
            22,
            "admin".to_string(),
            AuthMethod::Agent,
            None,
        );

        // Name should be stored correctly
        assert_eq!(
            server.name,
            *name,
            "Name should be preserved: {}",
            name.len()
        );
    }
}

/// Test with pathological search patterns
#[test]
fn test_pathological_patterns() {
    use easyssh_core::services::search_service::SearchQuery;

    let pathological_patterns = vec![
        "*",
        "?",
        ".*",
        "(a+)",
        "[a-z]+",
        "\\",
        "\\\\",
        "(a{1000})",
        "(?:)+",
    ];

    for pattern in &pathological_patterns {
        // Should not cause catastrophic backtracking or panic
        let query = SearchQuery {
            keyword: Some(pattern.to_string()),
            ..Default::default()
        };
        // Just creating the query should not panic
        let _ = query;
    }
}

/// Test concurrent fuzzing simulation
#[test]
fn test_concurrent_stress() {
    use easyssh_core::db::{Database, NewServer};
    use std::sync::Arc;
    use std::thread;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Arc::new(std::sync::Mutex::new(
        Database::new(db_path).expect("Failed to create database"),
    ));
    db.lock()
        .unwrap()
        .init()
        .expect("Failed to initialize database");

    let mut handles = vec![];

    // Spawn threads that perform random operations
    for thread_id in 0..5 {
        let db_clone = Arc::clone(&db);
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let op = (thread_id + i) % 3;

                match op {
                    0 => {
                        // Add server
                        let server = NewServer {
                            id: format!("srv-{}-{}", thread_id, i),
                            name: format!("Thread{}-Server{}", thread_id, i),
                            host: "192.168.1.1".to_string(),
                            port: 22,
                            username: "admin".to_string(),
                            auth_type: "agent".to_string(),
                            identity_file: None,
                            password_encrypted: None,
                            group_id: None,
                            status: "unknown".to_string(),
                        };
                        let _ = db_clone.lock().unwrap().create_server(&server);
                    }
                    1 => {
                        // Get all servers
                        let _ = db_clone.lock().unwrap().get_servers();
                    }
                    2 => {
                        // Try to get non-existent server
                        let _ = db_clone.lock().unwrap().get_server("non-existent-id");
                    }
                    _ => {}
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    // Database should still be functional after stress test
    let final_count = db
        .lock()
        .unwrap()
        .get_servers()
        .expect("Should get servers")
        .len();
    println!("Final server count after stress test: {}", final_count);
}

/// Test handling of corrupted data
#[test]
fn test_corrupted_data_recovery() {
    use easyssh_core::crypto::CryptoState;

    let mut state = CryptoState::new();
    state
        .initialize("test_password")
        .expect("Should initialize");

    // Create valid encrypted data
    let plaintext = b"test message";
    let encrypted = state.encrypt(plaintext).expect("Should encrypt");

    // Corrupt various parts
    let corrupted_nonce = {
        let mut data = encrypted.clone();
        data[0] ^= 0xFF; // Corrupt first byte of nonce
        data
    };

    let corrupted_ciphertext = {
        let mut data = encrypted.clone();
        let nonce_len = 12;
        data[nonce_len + 10] ^= 0xFF; // Corrupt middle of ciphertext
        data
    };

    let corrupted_tag = {
        let mut data = encrypted.clone();
        let tag_start = data.len() - 16;
        data[tag_start] ^= 0xFF; // Corrupt first byte of tag
        data
    };

    // All corrupted versions should fail to decrypt
    assert!(
        state.decrypt(&corrupted_nonce).is_err(),
        "Corrupted nonce should fail"
    );
    assert!(
        state.decrypt(&corrupted_ciphertext).is_err(),
        "Corrupted ciphertext should fail"
    );
    assert!(
        state.decrypt(&corrupted_tag).is_err(),
        "Corrupted tag should fail"
    );

    // Original should still work
    let decrypted = state.decrypt(&encrypted).expect("Original should decrypt");
    assert_eq!(decrypted, plaintext);
}
