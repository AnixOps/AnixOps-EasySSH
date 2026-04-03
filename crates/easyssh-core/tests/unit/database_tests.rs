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

use easyssh_core::db::{NewGroup, NewHost, NewServer, UpdateServer};

#[path = "../common/mod.rs"]
mod common;
use common::{create_in_memory_db, create_test_db, create_test_db_arc};

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
        "groups",
        "servers",
        "schema_migrations",
        "hosts",
        "tags",
        "host_tags",
        "identities",
        "snippets",
        "sessions",
        "layouts",
        "sync_state",
        "audit_events",
    ];

    for table in &tables {
        let result: Result<i64, _> =
            db.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
                row.get(0)
            });
        assert!(
            result.is_ok(),
            "Table {} should exist: {:?}",
            table,
            result.err()
        );
    }
}

#[test]
fn test_create_server() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "srv-001".to_string(),
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        group_id: None,
        status: "unknown".to_string(),
    };

    let result = db.add_server(&new_server);
    assert!(
        result.is_ok(),
        "Create server should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_get_server_by_id() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "srv-002".to_string(),
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        group_id: None,
        status: "unknown".to_string(),
    };

    db.add_server(&new_server)
        .expect("Create server should succeed");

    let server = db.get_server("srv-002");
    assert!(
        server.is_ok(),
        "Get server should succeed: {:?}",
        server.err()
    );

    let server = server.unwrap();
    assert_eq!(server.name, "Test Server");
    assert_eq!(server.host, "192.168.1.100");
}

#[test]
fn test_update_server() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "srv-003".to_string(),
        name: "Original Name".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        group_id: None,
        status: "unknown".to_string(),
    };

    db.add_server(&new_server)
        .expect("Create server should succeed");

    let update = UpdateServer {
        id: "srv-003".to_string(),
        name: Some("Updated Name".to_string()),
        host: None,
        port: None,
        username: None,
        auth_type: None,
        identity_file: None,
        group_id: None,
        status: Some("online".to_string()),
    };

    let result = db.update_server(&update);
    assert!(
        result.is_ok(),
        "Update server should succeed: {:?}",
        result.err()
    );

    let updated = db.get_server("srv-003").unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.status, "online");
    assert_eq!(updated.host, "192.168.1.100"); // Unchanged
}

#[test]
fn test_delete_server() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "srv-004".to_string(),
        name: "To Be Deleted".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        group_id: None,
        status: "unknown".to_string(),
    };

    db.add_server(&new_server)
        .expect("Create server should succeed");

    // Delete it
    let result = db.delete_server("srv-004");
    assert!(
        result.is_ok(),
        "Delete server should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_list_servers() {
    let (db, _temp) = create_test_db();

    // Create multiple servers
    for i in 0..5 {
        let new_server = NewServer {
            id: format!("srv-{}", i),
            name: format!("Server {}", i),
            host: format!("192.168.1.{}", i),
            port: 22,
            username: "admin".to_string(),
            auth_type: "password".to_string(),
            identity_file: None,
            group_id: None,
            status: "unknown".to_string(),
        };
        db.add_server(&new_server)
            .expect("Create server should succeed");
    }

    let servers = db.get_servers();
    assert!(
        servers.is_ok(),
        "List servers should succeed: {:?}",
        servers.err()
    );

    let servers = servers.unwrap();
    assert_eq!(servers.len(), 5, "Should have 5 servers");
}

#[test]
fn test_create_group() {
    let (db, _temp) = create_test_db();

    let new_group = NewGroup {
        id: "grp-001".to_string(),
        name: "Production".to_string(),
        color: Some("#ff0000".to_string()),
    };

    let result = db.add_group(&new_group);
    assert!(
        result.is_ok(),
        "Create group should succeed: {:?}",
        result.err()
    );

    let group = db.get_group("grp-001").unwrap();
    assert_eq!(group.name, "Production");
    assert_eq!(group.color, Some("#ff0000".to_string()));
}

#[test]
fn test_foreign_key_constraint() {
    let (db, _temp) = create_test_db();

    // Create a group first
    let group = NewGroup {
        id: "grp-fk".to_string(),
        name: "Test Group".to_string(),
        color: None,
    };
    db.add_group(&group).expect("Create group should succeed");

    // Create a server with that group
    let server = NewServer {
        id: "srv-fk".to_string(),
        name: "Test".to_string(),
        host: "host".to_string(),
        port: 22,
        username: "user".to_string(),
        auth_type: "agent".to_string(),
        identity_file: None,
        group_id: Some("grp-fk".to_string()),
        status: "unknown".to_string(),
    };
    db.add_server(&server)
        .expect("Create server with valid group should succeed");

    // Try to delete the group (should succeed with ON DELETE SET NULL)
    let result = db.delete_group("grp-fk");
    assert!(
        result.is_ok(),
        "Delete group should succeed: {:?}",
        result.err()
    );

    // Server should still exist but with null group_id
    let srv = db.get_server("srv-fk").unwrap();
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
                    id: format!("concurrent-srv-{}", i),
                    name: format!("Server {}", i),
                    host: format!("192.168.1.{}", i),
                    port: 22,
                    username: "admin".to_string(),
                    auth_type: "password".to_string(),
                    identity_file: None,
                    group_id: None,
                    status: "unknown".to_string(),
                };
                db.add_server(&new_server)
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
    let servers = db.get_servers().unwrap();
    assert_eq!(servers.len(), 10, "All 10 servers should exist");
}

#[test]
fn test_host_crud_operations() {
    let (db, _temp) = create_test_db();

    // Create a group
    let group = NewGroup {
        id: "grp-host".to_string(),
        name: "Host Group".to_string(),
        color: None,
    };
    db.add_group(&group).expect("Create group should succeed");

    // Create a host
    let new_host = NewHost {
        id: "host-001".to_string(),
        name: "Test Host".to_string(),
        host: "example.com".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        identity_id: None,
        group_id: Some("grp-host".to_string()),
        notes: Some("Test notes".to_string()),
        color: Some("#00ff00".to_string()),
        environment: Some("production".to_string()),
        region: Some("us-east".to_string()),
        purpose: Some("web-server".to_string()),
        status: "unknown".to_string(),
    };

    let result = db.add_host(&new_host);
    assert!(
        result.is_ok(),
        "Create host should succeed: {:?}",
        result.err()
    );

    // Get the host
    let host = db.get_host("host-001").unwrap();
    assert_eq!(host.name, "Test Host");
    assert_eq!(host.notes, Some("Test notes".to_string()));
}

#[test]
fn test_unique_id_constraint() {
    let (db, _temp) = create_test_db();

    let new_server = NewServer {
        id: "unique-test".to_string(),
        name: "First".to_string(),
        host: "host1".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        group_id: None,
        status: "unknown".to_string(),
    };

    db.add_server(&new_server)
        .expect("First create should succeed");

    // Try to create another server with the same ID
    let duplicate = NewServer {
        id: "unique-test".to_string(), // Same ID
        name: "Second".to_string(),
        host: "host2".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        group_id: None,
        status: "unknown".to_string(),
    };

    let result = db.add_server(&duplicate);
    assert!(result.is_err(), "Duplicate ID should fail");
}

#[test]
fn test_list_all_servers() {
    let (db, _temp) = create_test_db();

    // Create 20 servers
    for i in 0..20 {
        let new_server = NewServer {
            id: format!("page-srv-{}", i),
            name: format!("Server {}", i),
            host: format!("192.168.1.{}", i),
            port: 22,
            username: "admin".to_string(),
            auth_type: "password".to_string(),
            identity_file: None,
            group_id: None,
            status: "unknown".to_string(),
        };
        db.add_server(&new_server)
            .expect("Create server should succeed");
    }

    // Get all servers
    let servers = db.get_servers().unwrap();
    assert_eq!(servers.len(), 20);
}
