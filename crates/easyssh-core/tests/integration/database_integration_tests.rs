//! Database Integration Tests
//!
//! Comprehensive database testing including:
//! - CRUD operations
//! - Database initialization
//! - Concurrent access

use std::sync::Arc;
use std::thread;

#[path = "../common/mod.rs"]
mod common;
use common::{create_test_db, create_in_memory_db, create_test_db_arc};

use easyssh_core::db::{Database, NewServer, NewGroup, UpdateServer, UpdateGroup};
use tempfile::TempDir;

/// Test database initialization
#[test]
fn test_database_initialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(db_path).expect("Should create database");
    db.init().expect("Should initialize");

    // Verify database is initialized
    assert!(db.is_initialized().expect("Should check initialization"));
}

/// Test in-memory database creation
#[test]
fn test_in_memory_database() {
    let db = create_in_memory_db();

    // Should be able to get servers (empty list)
    let servers = db.get_servers().expect("Should get servers");
    assert!(servers.is_empty());
}

/// Test server CRUD operations
#[test]
fn test_server_crud() {
    let (db, _temp) = create_test_db();

    // Create server with NewServer struct
    let server = NewServer {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        group_id: None,
        status: "active".to_string(),
    };

    // Add server
    db.add_server(&server).expect("Should add server");

    // Read - get all servers
    let servers = db.get_servers().expect("Should get servers");
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "Test Server");
    assert_eq!(servers[0].host, "192.168.1.100");
    assert_eq!(servers[0].port, 22);

    // Read - get specific server
    let retrieved = db.get_server(&server.id).expect("Should get server");
    assert_eq!(retrieved.name, "Test Server");
    assert_eq!(retrieved.host, "192.168.1.100");

    // Update server
    let update = UpdateServer {
        id: server.id.clone(),
        name: Some("Updated Server".to_string()),
        host: Some("192.168.1.200".to_string()),
        port: Some(2222),
        username: None,
        auth_type: None,
        identity_file: None,
        group_id: None,
        status: None,
    };
    db.update_server(&update).expect("Should update server");

    let updated = db.get_server(&server.id).expect("Should get updated server");
    assert_eq!(updated.name, "Updated Server");
    assert_eq!(updated.host, "192.168.1.200");
    assert_eq!(updated.port, 2222);

    // Delete server
    db.delete_server(&server.id).expect("Should delete server");
    assert!(db.get_server(&server.id).is_err());
}

/// Test group CRUD operations
#[test]
fn test_group_crud() {
    let (db, _temp) = create_test_db();

    // Create group
    let group = NewGroup {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Production".to_string(),
        color: "#ff0000".to_string(),
    };

    // Add group
    db.add_group(&group).expect("Should add group");

    // Read - get all groups
    let groups = db.get_groups().expect("Should get groups");
    // Note: there may be default groups, so check that our group is present
    assert!(groups.iter().any(|g| g.name == "Production"));

    // Read - get specific group
    let retrieved = db.get_group(&group.id).expect("Should get group");
    assert_eq!(retrieved.name, "Production");
    assert_eq!(retrieved.color, "#ff0000");

    // Update group
    let update = UpdateGroup {
        id: group.id.clone(),
        name: Some("Prod".to_string()),
        color: Some("#cc0000".to_string()),
    };
    db.update_group(&update).expect("Should update group");

    let updated = db.get_group(&group.id).expect("Should get updated group");
    assert_eq!(updated.name, "Prod");
    assert_eq!(updated.color, "#cc0000");

    // Delete group
    db.delete_group(&group.id).expect("Should delete group");
    assert!(db.get_group(&group.id).is_err());
}

/// Test server-group relationship
#[test]
fn test_server_group_relationship() {
    let (db, _temp) = create_test_db();

    // Create a group first
    let group = NewGroup {
        id: uuid::Uuid::new_v4().to_string(),
        name: "TestGroup".to_string(),
        color: "#00ff00".to_string(),
    };
    db.add_group(&group).expect("Should add group");

    // Create server with group
    let server = NewServer {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Server in Group".to_string(),
        host: "192.168.1.10".to_string(),
        port: 22,
        username: "user".to_string(),
        auth_type: "password".to_string(),
        identity_file: None,
        group_id: Some(group.id.clone()),
        status: "active".to_string(),
    };
    db.add_server(&server).expect("Should add server");

    // Verify server has group
    let retrieved = db.get_server(&server.id).expect("Should get server");
    assert_eq!(retrieved.group_id, Some(group.id.clone()));
}

/// Test concurrent database access
#[test]
fn test_concurrent_database_access() {
    let (db_arc, _temp) = create_test_db_arc();

    let mut handles = vec![];

    // Spawn threads that perform database operations
    for i in 0..5 {
        let db_clone = Arc::clone(&db_arc);
        let handle = thread::spawn(move || {
            for j in 0..5 {
                let server = NewServer {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("Thread{}-Server{}", i, j),
                    host: "192.168.1.1".to_string(),
                    port: 22,
                    username: "user".to_string(),
                    auth_type: "password".to_string(),
                    identity_file: None,
                    group_id: None,
                    status: "active".to_string(),
                };
                db_clone.lock().unwrap().add_server(&server).expect("Should add");
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    // Verify all servers were added
    let servers = db_arc.lock().unwrap().get_servers().expect("Should get servers");
    assert_eq!(servers.len(), 25);
}

/// Test config operations
#[test]
fn test_config_operations() {
    let (db, _temp) = create_test_db();

    // Set config value
    db.set_config("test_key", "test_value").expect("Should set config");

    // Get config value
    let value = db.get_config("test_key").expect("Should get config");
    assert_eq!(value, Some("test_value".to_string()));

    // Get non-existent config
    let value = db.get_config("nonexistent").expect("Should get config");
    assert!(value.is_none());

    // Update config
    db.set_config("test_key", "updated_value").expect("Should set config");
    let value = db.get_config("test_key").expect("Should get config");
    assert_eq!(value, Some("updated_value".to_string()));
}