//! Fuzz Tests for EasySSH Core
//!
//! Property-based and fuzz testing for robustness:
//! - Random input handling
//! - Configuration parsing edge cases
//! - Protocol compliance

use std::str::FromStr;

mod common;

/// Test handling of random byte sequences as input
#[test]
fn test_random_input_handling() {
    use easyssh_core::crypto::CryptoState;

    let mut state = CryptoState::new();
    state.initialize("test_password").expect("Should initialize");

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
    use easyssh_core::config::SshConfig;

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
        // Should not panic, may return error
        let _ = SshConfig::from_str(config);
    }
}

/// Test with extreme integer values
#[test]
fn test_extreme_integer_values() {
    use easyssh_core::models::Server;

    let extreme_ports = vec![
        0,
        1,
        22,
        443,
        8080,
        65535,
        65536, // Overflow
    ];

    for port in &extreme_ports {
        let server = Server::new("test", "192.168.1.1", *port)
            .with_username("admin");

        // Port should be clamped to valid range
        assert!(server.port <= 65535, "Port should not exceed u16 max");
    }
}

/// Test string boundary conditions
#[test]
fn test_string_boundary_conditions() {
    use easyssh_core::models::Server;

    let boundary_strings = vec![
        "",
        "a",
        "ab",
        "a".repeat(255),
        "a".repeat(256),
        "a".repeat(1000),
        "测试中文",
        "🎉emoji",
        "special!@#$%^&*()chars",
        "\n\r\t whitespace",
    ];

    for name in &boundary_strings {
        let server = Server::new(name, "192.168.1.1", 22)
            .with_username("admin");

        // Name should be stored correctly
        assert_eq!(server.name, *name, "Name should be preserved: {}", name.len());
    }
}

/// Test with pathological regex patterns
#[test]
fn test_pathological_patterns() {
    use easyssh_core::search::SearchQuery;

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
        let query = SearchQuery::new(pattern.to_string());
        let _ = query.validate();
    }
}

/// Test concurrent fuzzing simulation
#[test]
fn test_concurrent_stress() {
    use easyssh_core::db::Database;
    use std::sync::Arc;
    use std::thread;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Arc::new(std::sync::Mutex::new(
        Database::new(&db_path).expect("Failed to create database")
    ));
    db.lock().unwrap().init().expect("Failed to initialize database");

    let mut handles = vec![];

    // Spawn threads that perform random operations
    for thread_id in 0..5 {
        let db_clone = Arc::clone(&db);
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let op = (thread_id + i) % 4;

                match op {
                    0 => {
                        // Add server
                        let server = easyssh_core::models::Server::new(
                            &format!("Thread{}-Server{}", thread_id, i),
                            "192.168.1.1",
                            22
                        );
                        let _ = db_clone.lock().unwrap().add_server(&server);
                    }
                    1 => {
                        // Get all servers
                        let _ = db_clone.lock().unwrap().get_all_servers();
                    }
                    2 => {
                        // Try to get non-existent server
                        let _ = db_clone.lock().unwrap().get_server("non-existent-id");
                    }
                    3 => {
                        // Update random server
                        let _ = db_clone.lock().unwrap().update_server(
                            &format!("random-id-{}", i),
                            &easyssh_core::models::ServerUpdate::default()
                        );
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
    let final_count = db.lock().unwrap().get_all_servers().expect("Should get servers").len();
    println!("Final server count after stress test: {}", final_count);
}

/// Test handling of corrupted data
#[test]
fn test_corrupted_data_recovery() {
    use easyssh_core::crypto::CryptoState;

    let mut state = CryptoState::new();
    state.initialize("test_password").expect("Should initialize");

    let plaintext = b"important data that must not be lost";
    let encrypted = state.encrypt(plaintext).expect("Should encrypt");

    // Try various corruption patterns
    let corruption_patterns = vec![
        // Corrupt first byte
        {
            let mut corrupted = encrypted.clone();
            if !corrupted.is_empty() {
                corrupted[0] ^= 0xFF;
            }
            corrupted
        },
        // Corrupt middle byte
        {
            let mut corrupted = encrypted.clone();
            let mid = corrupted.len() / 2;
            if mid < corrupted.len() {
                corrupted[mid] ^= 0xFF;
            }
            corrupted
        },
        // Corrupt last byte
        {
            let mut corrupted = encrypted.clone();
            if !corrupted.is_empty() {
                let last = corrupted.len() - 1;
                corrupted[last] ^= 0xFF;
            }
            corrupted
        },
        // Truncate
        encrypted[..encrypted.len().saturating_sub(5)].to_vec(),
        // Extend with garbage
        {
            let mut extended = encrypted.clone();
            extended.extend_from_slice(&[0xFF; 100]);
            extended
        },
    ];

    for (i, corrupted) in corruption_patterns.iter().enumerate() {
        // Should fail gracefully, not panic
        let result = state.decrypt(corrupted);
        assert!(
            result.is_err(),
            "Corrupted data (pattern {}) should fail decryption",
            i
        );
    }
}

/// Test with unusual but valid unicode
#[test]
fn test_unusual_unicode_handling() {
    use easyssh_core::models::Server;

    let unusual_strings = vec![
        "\u{0000}",              // Null character
        "\u{0001}",              // Control character
        "\u{200B}",              // Zero-width space
        "\u{FEFF}",              // BOM
        "\u{1F600}",             // Emoji
        "العربية",               // Arabic
        "עברית",                 // Hebrew (RTL)
        "日本語テキスト",         // Japanese
        "한국어",                 // Korean
        "𝕦𝕟𝕚𝕔𝕠𝕕𝕖",              // Mathematical letters
    ];

    for (i, s) in unusual_strings.iter().enumerate() {
        let server = Server::new(s, "192.168.1.1", 22)
            .with_username("admin");

        // Should handle unusual unicode gracefully
        assert!(!server.name.is_empty(), "Name {} should not be empty", i);
    }
}
