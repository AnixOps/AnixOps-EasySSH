//! Integration Tests for EasySSH Core
//!
//! Tests for end-to-end workflows including:
//! - Server creation and connection workflow
//! - Import and search workflow
//! - Encryption and storage workflow
//! - Cross-module interactions

use std::sync::{Arc, Mutex};

use easyssh_core::crypto::{CryptoState, ServerCredential};
use easyssh_core::db::{Database, NewGroup, NewHost, NewServer};
use easyssh_core::models::server::{AuthMethod as ServerAuthMethod, ServerStatus};
use easyssh_core::services::search_service::{SearchFilter, SearchService};
use easyssh_core::services::server_service::{CreateServerDto, ServerService};

#[path = "../common/mod.rs"]
mod common;
use common::{create_test_db, create_test_db_arc, test_master_password};

/// Test: Create encrypted server and retrieve it
#[test]
fn test_create_encrypted_server_workflow() {
    let (db, _temp) = create_test_db();

    // Initialize crypto
    let mut crypto = CryptoState::new();
    crypto
        .initialize(test_master_password())
        .expect("Crypto init should succeed");

    // Create credential
    let credential =
        ServerCredential::with_password("srv-001", "192.168.1.100", "admin", "secret_password");

    // Encrypt credential
    let encrypted = credential
        .encrypt(&crypto)
        .expect("Encryption should succeed");

    // Store in database (simulated - in real code, this would be part of the server record)
    let new_server = NewServer {
        id: "srv-001",
        name: "Test Server",
        host: "192.168.1.100",
        port: 22,
        username: "admin",
        auth_type: "password",
        identity_file: None,
        password_encrypted: Some(encrypted.encrypted_data.clone()),
        group_id: None,
        status: "unknown",
    };

    db.create_server(&new_server)
        .expect("Create server should succeed");

    // Retrieve from database
    let stored = db
        .get_server("srv-001")
        .expect("Get should succeed")
        .expect("Server should exist");
    assert_eq!(stored.name, "Test Server");

    // Decrypt and verify
    let encrypted_data = stored
        .password_encrypted
        .expect("Should have encrypted data");
    let decrypted = crypto
        .decrypt(&encrypted_data)
        .expect("Decryption should succeed");
    assert_eq!(decrypted, b"secret_password");
}

/// Test: Full server management workflow
#[test]
fn test_full_server_management_workflow() {
    let (service, db_arc, _temp) = {
        let (db_arc, temp) = create_test_db_arc();
        let service = ServerService::new(db_arc.clone());
        (service, db_arc, temp)
    };

    // Create groups
    {
        let db = db_arc.lock().unwrap();
        let groups = vec![
            NewGroup {
                id: "prod",
                name: "Production",
                color: "#ff0000".to_string(),
            },
            NewGroup {
                id: "dev",
                name: "Development",
                color: "#00ff00".to_string(),
            },
        ];
        for group in &groups {
            db.create_group(group).expect("Create group should succeed");
        }
    }

    // Create servers in different groups
    let servers = vec![
        CreateServerDto {
            name: "Web Server".to_string(),
            host: "192.168.1.10".to_string(),
            port: Some(22),
            username: "admin".to_string(),
            auth_method: ServerAuthMethod::Password {
                password: "web_pass".to_string(),
            },
            group_id: Some("prod".to_string()),
        },
        CreateServerDto {
            name: "Database Server".to_string(),
            host: "192.168.1.11".to_string(),
            port: Some(22),
            username: "dbadmin".to_string(),
            auth_method: ServerAuthMethod::PrivateKey {
                key_path: "~/.ssh/id_rsa".to_string(),
                passphrase: None,
            },
            group_id: Some("prod".to_string()),
        },
        CreateServerDto {
            name: "Dev Server".to_string(),
            host: "dev.local".to_string(),
            port: Some(2222),
            username: "developer".to_string(),
            auth_method: ServerAuthMethod::Agent,
            group_id: Some("dev".to_string()),
        },
    ];

    let mut created_ids = vec![];
    for server in &servers {
        let created = service
            .create_server(server.clone())
            .expect("Create should succeed");
        created_ids.push(created.id.clone());
    }

    // List all servers
    let all_servers = service.list_all_servers().expect("List should succeed");
    assert_eq!(all_servers.len(), 3);

    // Get servers by group
    let prod_servers = service
        .get_servers_by_group("prod")
        .expect("Get by group should succeed");
    assert_eq!(prod_servers.len(), 2);

    let dev_servers = service
        .get_servers_by_group("dev")
        .expect("Get by group should succeed");
    assert_eq!(dev_servers.len(), 1);
    assert_eq!(dev_servers[0].name, "Dev Server");

    // Update server status
    service
        .update_server_status(&created_ids[0], ServerStatus::Online)
        .expect("Update status should succeed");

    let updated = service.get_server(&created_ids[0]).unwrap().unwrap();
    assert_eq!(updated.status, ServerStatus::Online);

    // Delete a server
    service
        .delete_server(&created_ids[2])
        .expect("Delete should succeed");

    let remaining = service.list_all_servers().expect("List should succeed");
    assert_eq!(remaining.len(), 2);
}

/// Test: Search and filter workflow
#[test]
fn test_search_and_filter_workflow() {
    let (db_arc, temp) = create_test_db_arc();
    let search_service = SearchService::new(db_arc.clone());

    // Create groups and hosts
    {
        let db = db.lock().unwrap();

        // Create groups
        let groups = vec![
            NewGroup {
                id: "web",
                name: "Web Servers",
                color: "#000000".to_string(),
            },
            NewGroup {
                id: "db",
                name: "Database",
                color: "#000000".to_string(),
            },
        ];
        for group in &groups {
            db.create_group(group).expect("Create group should succeed");
        }

        // Create hosts with various attributes
        let hosts = vec![
            (
                "host-001",
                "Production Web Server",
                "10.0.1.10",
                "web",
                "password",
                "online",
            ),
            (
                "host-002",
                "Staging Web Server",
                "10.0.2.10",
                "web",
                "key",
                "offline",
            ),
            (
                "host-003",
                "Primary Database",
                "10.0.1.20",
                "db",
                "password",
                "online",
            ),
            (
                "host-004",
                "Replica Database",
                "10.0.2.20",
                "db",
                "password",
                "online",
            ),
            (
                "host-005",
                "Cache Server",
                "10.0.1.30",
                None,
                "agent",
                "online",
            ),
        ];

        for (id, name, host, group_id, auth_type, status) in hosts {
            let new_host = NewHost {
                id,
                name,
                host,
                port: 22,
                username: "admin",
                auth_type,
                identity_file: None,
                identity_id: None,
                group_id: Some(group_id),
                notes: None,
                color: None,
                environment: None,
                region: None,
                purpose: None,
                status,
            };
            db.create_host(&new_host)
                .expect("Create host should succeed");
        }
    }

    // Search tests

    // 1. Basic text search
    let filter = SearchFilter {
        query: Some("web".to_string()),
        ..Default::default()
    };
    let results = search_service
        .search_hosts(&filter)
        .expect("Search should succeed");
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|h| h.name.contains("Web")));

    // 2. IP search
    let filter = SearchFilter {
        query: Some("10.0.1".to_string()),
        ..Default::default()
    };
    let results = search_service
        .search_hosts(&filter)
        .expect("Search should succeed");
    assert_eq!(results.len(), 3); // Production Web, Primary DB, Cache Server

    // 3. Group filter
    let filter = SearchFilter {
        group_ids: Some(vec!["db".to_string()]),
        ..Default::default()
    };
    let results = search_service
        .search_hosts(&filter)
        .expect("Search should succeed");
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|h| h.name.contains("Database")));

    // 4. Status filter
    let filter = SearchFilter {
        connection_status: Some(vec![
            easyssh_core::services::search_service::ConnectionStatus::Online,
        ]),
        ..Default::default()
    };
    let results = search_service
        .search_hosts(&filter)
        .expect("Search should succeed");
    assert_eq!(results.len(), 4);

    // 5. Combined filters
    let filter = SearchFilter {
        group_ids: Some(vec!["web".to_string()]),
        query: Some("production".to_string()),
        connection_status: Some(vec![
            easyssh_core::services::search_service::ConnectionStatus::Online,
        ]),
        ..Default::default()
    };
    let results = search_service
        .search_hosts(&filter)
        .expect("Search should succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Production Web Server");

    // 6. Record and retrieve search history
    search_service
        .record_search("web servers")
        .expect("Record should succeed");
    search_service
        .record_search("production database")
        .expect("Record should succeed");

    let history = search_service
        .get_search_history(10)
        .expect("Get history should succeed");
    assert_eq!(history.len(), 2);
}

/// Test: Import and search workflow
#[test]
fn test_import_and_search_workflow() {
    let (db_arc, _temp) = create_test_db_arc();
    let server_service = ServerService::new(db_arc.clone());
    let search_service = SearchService::new(db_arc.clone());

    // Import multiple servers
    let servers = vec![
        CreateServerDto {
            name: "Imported Server 1".to_string(),
            host: "192.168.1.100".to_string(),
            port: Some(22),
            username: "admin".to_string(),
            auth_method: ServerAuthMethod::Password {
                password: "pass1".to_string(),
            },
            group_id: None,
        },
        CreateServerDto {
            name: "Imported Server 2".to_string(),
            host: "192.168.1.101".to_string(),
            port: Some(22),
            username: "root".to_string(),
            auth_method: ServerAuthMethod::Agent,
            group_id: None,
        },
        CreateServerDto {
            name: "Imported Database".to_string(),
            host: "192.168.1.102".to_string(),
            port: Some(22),
            username: "dbadmin".to_string(),
            auth_method: ServerAuthMethod::PrivateKey {
                key_path: "~/.ssh/db_key".to_string(),
                passphrase: None,
            },
            group_id: None,
        },
    ];

    let import_result = server_service
        .import_servers(servers)
        .expect("Import should succeed");
    assert_eq!(import_result.total, 3);
    assert_eq!(import_result.imported, 3);

    // Search for imported servers
    let all = server_service
        .list_all_servers()
        .expect("List should succeed");
    assert_eq!(all.len(), 3);

    // Search by name
    let results = server_service
        .search_servers("Database", None)
        .expect("Search should succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Imported Database");

    // Verify authentication methods were preserved
    let db_result = results[0].clone();
    assert!(matches!(
        db_result.auth_method,
        ServerAuthMethod::PrivateKey { .. }
    ));
}

/// Test: Crypto + Database encryption workflow
#[test]
fn test_encryption_decryption_workflow() {
    let (db, _temp) = create_test_db();

    // Initialize crypto
    let mut crypto = CryptoState::new();
    crypto
        .initialize(test_master_password())
        .expect("Init should succeed");

    let salt = crypto.get_salt().expect("Should have salt");

    // Encrypt multiple pieces of data
    let data_to_encrypt = vec![
        ("password1", b"secret_password_123" as &[u8]),
        ("password2", b"another_secret_pass"),
        ("key_passphrase", b"key_passphrase_456"),
    ];

    let mut encrypted_data = vec![];

    for (label, data) in data_to_encrypt {
        let encrypted = crypto
            .encrypt(data)
            .expect(&format!("Encrypt {} should succeed", label));
        encrypted_data.push((label, encrypted));
    }

    // Lock and re-unlock with same password
    crypto.lock();
    assert!(!crypto.is_unlocked());

    crypto.set_salt(salt.try_into().expect("Salt should be 32 bytes"));
    crypto
        .unlock(test_master_password())
        .expect("Unlock should succeed");
    assert!(crypto.is_unlocked());

    // Decrypt and verify all data
    for (label, encrypted) in &encrypted_data {
        let decrypted = crypto
            .decrypt(encrypted)
            .expect(&format!("Decrypt {} should succeed", label));

        // Find original data
        let original = data_to_encrypt
            .iter()
            .find(|(l, _)| *l == *label)
            .unwrap()
            .1;
        assert_eq!(
            decrypted, original,
            "Decrypted {} should match original",
            label
        );
    }
}

/// Test: Concurrent operations workflow
#[test]
fn test_concurrent_operations_workflow() {
    use std::thread;
    use std::time::Duration;

    let (db_arc, _temp) = create_test_db_arc();

    // Spawn threads for concurrent operations
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let db_clone = Arc::clone(&db_arc);
            thread::spawn(move || {
                let db = db_clone.lock().unwrap();

                // Create server
                let new_server = NewServer {
                    id: &format!("concurrent-{}", i),
                    name: &format!("Concurrent Server {}", i),
                    host: &format!("192.168.{}.1", i),
                    port: 22,
                    username: "admin",
                    auth_type: "password",
                    identity_file: None,
                    password_encrypted: None,
                    group_id: None,
                    status: "unknown",
                };

                db.create_server(&new_server)
                    .expect("Create should succeed");

                // Retrieve it
                let retrieved = db
                    .get_server(&format!("concurrent-{}", i))
                    .expect("Get should succeed")
                    .expect("Server should exist");

                assert_eq!(retrieved.name, format!("Concurrent Server {}", i));

                // Update status
                let update = easyssh_core::db::UpdateServer {
                    id: &format!("concurrent-{}", i),
                    name: None,
                    host: None,
                    port: None,
                    username: None,
                    auth_type: None,
                    identity_file: None,
                    password_encrypted: None,
                    group_id: None,
                    status: Some("online"),
                };

                db.update_server(&update).expect("Update should succeed");

                // Verify update
                let updated = db
                    .get_server(&format!("concurrent-{}", i))
                    .unwrap()
                    .unwrap();
                assert_eq!(updated.status, "online");

                thread::sleep(Duration::from_millis(10));

                // Delete
                db.delete_server(&format!("concurrent-{}", i))
                    .expect("Delete should succeed");
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread should complete without panic");
    }

    // Verify all servers were deleted
    let db = db_arc.lock().unwrap();
    let remaining = db.list_servers().expect("List should succeed");
    assert!(remaining.is_empty(), "All servers should have been deleted");
}

/// Test: Error handling workflow
#[test]
fn test_error_handling_workflow() {
    let (db, _temp) = create_test_db();
    let (db_arc, _temp2) = create_test_db_arc();
    let service = ServerService::new(db_arc);

    // 1. Try to get non-existent server
    let result = db.get_server("nonexistent");
    assert!(result.is_ok(), "Get nonexistent should not error");
    assert!(result.unwrap().is_none(), "Nonexistent should return None");

    // 2. Try to update non-existent server
    let update = easyssh_core::db::UpdateServer {
        id: "nonexistent",
        name: Some("New Name"),
        host: None,
        port: None,
        username: None,
        auth_type: None,
        identity_file: None,
        password_encrypted: None,
        group_id: None,
        status: None,
    };

    // Database update might succeed without affecting rows
    let result = db.update_server(&update);
    assert!(
        result.is_ok(),
        "Update nonexistent should return Ok (no rows affected)"
    );

    // But service should return NotFound error
    let service_update = easyssh_core::models::server::UpdateServerDto {
        id: "nonexistent".to_string(),
        name: Some("New Name".to_string()),
        host: None,
        port: None,
        username: None,
        auth_method: None,
        group_id: None,
    };

    let result = service.update_server(service_update);
    assert!(result.is_err(), "Service update nonexistent should error");
    match result.unwrap_err() {
        easyssh_core::services::server_service::ServerServiceError::NotFound(_) => {}
        _ => panic!("Expected NotFound error"),
    }

    // 3. Try to delete non-existent server
    let result = db.delete_server("nonexistent");
    assert!(
        result.is_ok(),
        "Delete nonexistent should return Ok (no rows affected)"
    );

    // 4. Try to create server with duplicate ID
    let new_server = NewServer {
        id: "duplicate-test",
        name: "First",
        host: "host1",
        port: 22,
        username: "admin",
        auth_type: "password",
        identity_file: None,
        password_encrypted: None,
        group_id: None,
        status: "unknown",
    };

    db.create_server(&new_server)
        .expect("First create should succeed");

    let duplicate = NewServer {
        id: "duplicate-test",
        name: "Second",
        host: "host2",
        port: 22,
        username: "admin",
        auth_type: "password",
        identity_file: None,
        password_encrypted: None,
        group_id: None,
        status: "unknown",
    };

    let result = db.create_server(&duplicate);
    assert!(result.is_err(), "Duplicate ID should fail");
}
