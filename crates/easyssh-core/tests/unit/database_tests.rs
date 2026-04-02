//! Database Module Unit Tests
//!
//! Tests for database operations including:
//! - CRUD operations for servers, groups, identities
//! - Transaction support
//! - Concurrent access safety
//! - Schema migrations
//! - Foreign key constraints

use std::sync::Arc;
use std::thread;

use easyssh_core::db::{Database, NewServer, UpdateServer, NewGroup, NewHost, HostFilter};
use easyssh_core::models::server::{CreateServerDto, ServerBuilder, AuthMethod};

mod common;
use common::{create_test_db, create_in_memory_db, create_test_db_arc, TestServerFixture};

#[test]
fn test_database_creation() {
    let (db, _temp) = create_test_db();
    // Database should be created and initialized without errors
    drop(db);
}

#[test]
fn test_database_init_creates_tables() {
    let db = create_in_memory_db();

    // Try to query each expected table
    let tables = [
        "groups", "servers", "schema_migrations", "hosts",
        "tags", "host_tags", "identities", "snippets",
        "sessions", "layouts", "sync_state", "audit_events"
    ];

    for table in &tables {
        let result: Result<i64, _> = db.query_row(
            &format!("SELECT COUNT(*) FROM {}", table),
            [],
            |row| row.get(0)
        );
        assert!(result.is_ok(), "Table {} should exist: {:?}", table, result.err());
    }
}

#[test]
fn test_create_server() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "srv-001",
        name: "Test Server",
        host: "192.168.1.100",
        port: 22,
        username: "admin",
        auth_type: "password",
        identity_file: None,
        password_encrypted: Some(vec![1, 2, 3, 4]), // Simulated encrypted password
        group_id: None,
        status: "unknown",
    };

    let result = db.create_server(&new_server);
    assert!(result.is_ok(), "Create server should succeed: {:?}", result.err());
}

#[test]
fn test_get_server_by_id() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "srv-002",
        name: "Test Server",
        host: "192.168.1.100",
        port: 22,
        username: "admin",
        auth_type: "password",
        identity_file: None,
        password_encrypted: None,
        group_id: None,
        status: "unknown",
    };

    db.create_server(&new_server).expect("Create server should succeed");

    let server = db.get_server("srv-002");
    assert!(server.is_ok(), "Get server should succeed: {:?}", server.err());

    let server = server.unwrap();
    assert!(server.is_some(), "Server should exist");
    let server = server.unwrap();
    assert_eq!(server.name, "Test Server");
    assert_eq!(server.host, "192.168.1.100");
}

#[test]
fn test_get_nonexistent_server() {
    let (db, _temp) = create_test_db();

    let server = db.get_server("nonexistent");
    assert!(server.is_ok(), "Get nonexistent server should not error");
    assert!(server.unwrap().is_none(), "Nonexistent server should return None");
}

#[test]
fn test_update_server() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "srv-003",
        name: "Original Name",
        host: "192.168.1.100",
        port: 22,
        username: "admin",
        auth_type: "password",
        identity_file: None,
        password_encrypted: None,
        group_id: None,
        status: "unknown",
    };

    db.create_server(&new_server).expect("Create server should succeed");

    let update = UpdateServer {
        id: "srv-003",
        name: Some("Updated Name"),
        host: None,
        port: None,
        username: None,
        auth_type: None,
        identity_file: None,
        password_encrypted: None,
        group_id: None,
        status: Some("online"),
    };

    let result = db.update_server(&update);
    assert!(result.is_ok(), "Update server should succeed: {:?}", result.err());

    let updated = db.get_server("srv-003").unwrap().unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.status, "online");
    assert_eq!(updated.host, "192.168.1.100"); // Unchanged
}

#[test]
fn test_delete_server() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "srv-004",
        name: "To Be Deleted",
        host: "192.168.1.100",
        port: 22,
        username: "admin",
        auth_type: "password",
        identity_file: None,
        password_encrypted: None,
        group_id: None,
        status: "unknown",
    };

    db.create_server(&new_server).expect("Create server should succeed");

    // Verify it exists
    assert!(db.get_server("srv-004").unwrap().is_some());

    // Delete it
    let result = db.delete_server("srv-004");
    assert!(result.is_ok(), "Delete server should succeed: {:?}", result.err());

    // Verify it's gone
    assert!(db.get_server("srv-004").unwrap().is_none());
}

#[test]
fn test_list_servers() {
    let (db, _temp) = create_test_db();

    // Create multiple servers
    for i in 0..5 {
        let new_server = NewServer {
            id: &format!("srv-{}", i),
            name: &format!("Server {}", i),
            host: &format!("192.168.1.{}", i),
            port: 22,
            username: "admin",
            auth_type: "password",
            identity_file: None,
            password_encrypted: None,
            group_id: None,
            status: "unknown",
        };
        db.create_server(&new_server).expect("Create server should succeed");
    }

    let servers = db.list_servers();
    assert!(servers.is_ok(), "List servers should succeed: {:?}", servers.err());

    let servers = servers.unwrap();
    assert_eq!(servers.len(), 5, "Should have 5 servers");
}

#[test]
fn test_create_group() {
    let (db, _temp) = create_test_db();

    let new_group = NewGroup {
        id: "grp-001",
        name: "Production",
        color: Some("#ff0000"),
    };

    let result = db.create_group(&new_group);
    assert!(result.is_ok(), "Create group should succeed: {:?}", result.err());

    let group = db.get_group("grp-001").unwrap();
    assert!(group.is_some());
    let group = group.unwrap();
    assert_eq!(group.name, "Production");
    assert_eq!(group.color, Some("#ff0000".to_string()));
}

#[test]
fn test_foreign_key_constraint() {
    let (db, _temp) = create_test_db();

    // Create a group first
    let group = NewGroup {
        id: "grp-fk",
        name: "Test Group",
        color: None,
    };
    db.create_group(&group).expect("Create group should succeed");

    // Create a server with that group
    let server = NewServer {
        id: "srv-fk",
        name: "Test",
        host: "host",
        port: 22,
        username: "user",
        auth_type: "agent",
        identity_file: None,
        password_encrypted: None,
        group_id: Some("grp-fk"),
        status: "unknown",
    };
    db.create_server(&server).expect("Create server with valid group should succeed");

    // Try to delete the group (should succeed with ON DELETE SET NULL)
    let result = db.delete_group("grp-fk");
    assert!(result.is_ok(), "Delete group should succeed: {:?}", result.err());

    // Server should still exist but with null group_id
    let srv = db.get_server("srv-fk").unwrap().unwrap();
    assert!(srv.group_id.is_none());
}

#[test]
fn test_concurrent_database_access() {
    let (db_arc, _temp) = create_test_db_arc();

    // Spawn multiple threads that all access the database
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let db_clone = Arc::clone(&db_arc);
            thread::spawn(move || {
                let db = db_clone.lock().unwrap();
                let new_server = NewServer {
                    id: &format!("concurrent-srv-{}", i),
                    name: &format!("Server {}", i),
                    host: &format!("192.168.1.{}", i),
                    port: 22,
                    username: "admin",
                    auth_type: "password",
                    identity_file: None,
                    password_encrypted: None,
                    group_id: None,
                    status: "unknown",
                };
                db.create_server(&new_server)
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        let result = handle.join();
        assert!(result.is_ok(), "Thread should complete without panic");
        assert!(result.unwrap().is_ok(), "Create server should succeed");
    }

    // Verify all servers were created
    let db = db_arc.lock().unwrap();
    let servers = db.list_servers().unwrap();
    assert_eq!(servers.len(), 10, "All 10 servers should exist");
}

#[test]
fn test_host_crud_operations() {
    let (db, _temp) = create_test_db();

    // Create a group
    let group = NewGroup {
        id: "grp-host",
        name: "Host Group",
        color: None,
    };
    db.create_group(&group).expect("Create group should succeed");

    // Create a host
    let new_host = NewHost {
        id: "host-001",
        name: "Test Host",
        host: "example.com",
        port: 22,
        username: "admin",
        auth_type: "password",
        identity_file: None,
        identity_id: None,
        group_id: Some("grp-host"),
        notes: Some("Test notes"),
        color: Some("#00ff00"),
        environment: Some("production"),
        region: Some("us-east"),
        purpose: Some("web-server"),
        status: "unknown",
    };

    let result = db.create_host(&new_host);
    assert!(result.is_ok(), "Create host should succeed: {:?}", result.err());

    // Get the host
    let host = db.get_host("host-001").unwrap();
    assert!(host.is_some());
    let host = host.unwrap();
    assert_eq!(host.name, "Test Host");
    assert_eq!(host.notes, Some("Test notes".to_string()));
}

#[test]
fn test_search_hosts() {
    let (db, _temp) = create_test_db();

    // Create hosts
    let hosts = [
        ("host-web", "Web Server", "web.example.com"),
        ("host-db", "Database", "db.example.com"),
        ("host-cache", "Cache Server", "cache.example.com"),
    ];

    for (id, name, host) in &hosts {
        let new_host = NewHost {
            id,
            name,
            host: *host,
            port: 22,
            username: "admin",
            auth_type: "agent",
            identity_file: None,
            identity_id: None,
            group_id: None,
            notes: None,
            color: None,
            environment: None,
            region: None,
            purpose: None,
            status: "unknown",
        };
        db.create_host(&new_host).expect("Create host should succeed");
    }

    // Search by name
    let filter = HostFilter {
        search: Some("Web"),
        ..Default::default()
    };
    let results = db.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Web Server");

    // Search by hostname
    let filter = HostFilter {
        search: Some("example.com"),
        ..Default::default()
    };
    let results = db.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_transaction_rollback() {
    let (db, _temp) = create_test_db();

    // Start a transaction implicitly by using execute_batch
    let result = db.execute_batch(r#"
        BEGIN;
        INSERT INTO groups (id, name, created_at, updated_at)
        VALUES ('tx-test', 'Transaction Test', '2024-01-01', '2024-01-01');
        ROLLBACK;
    "#);

    assert!(result.is_ok(), "Transaction rollback should succeed: {:?}", result.err());

    // Verify the group was not created
    let group = db.get_group("tx-test").unwrap();
    assert!(group.is_none(), "Rolled back transaction should not persist");
}

#[test]
fn test_unique_id_constraint() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "unique-test",
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

    db.create_server(&new_server).expect("First create should succeed");

    // Try to create another server with the same ID
    let duplicate = NewServer {
        id: "unique-test", // Same ID
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

#[test]
fn test_list_servers_with_pagination() {
    let (db, _temp) = create_test_db();

    // Create 20 servers
    for i in 0..20 {
        let new_server = NewServer {
            id: &format!("page-srv-{}", i),
            name: &format!("Server {}", i),
            host: &format!("192.168.1.{}", i),
            port: 22,
            username: "admin",
            auth_type: "password",
            identity_file: None,
            password_encrypted: None,
            group_id: None,
            status: "unknown",
        };
        db.create_server(&new_server).expect("Create server should succeed");
    }

    // Get first page (10 items)
    let page1 = db.list_servers_paginated(0, 10).unwrap();
    assert_eq!(page1.len(), 10);

    // Get second page
    let page2 = db.list_servers_paginated(10, 10).unwrap();
    assert_eq!(page2.len(), 10);

    // Ensure no overlap
    let page1_ids: std::collections::HashSet<_> = page1.iter().map(|s| &s.id).collect();
    let page2_ids: std::collections::HashSet<_> = page2.iter().map(|s| &s.id).collect();
    let intersection: Vec<_> = page1_ids.intersection(&page2_ids).collect();
    assert!(intersection.is_empty(), "Pages should not overlap");
}
