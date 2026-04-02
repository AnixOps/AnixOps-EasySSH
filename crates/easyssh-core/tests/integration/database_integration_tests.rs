//! Database Integration Tests
//!
//! Comprehensive database testing including:
//! - CRUD operations
//! - Transaction handling
//! - Migration tests
//! - Concurrent access
//! - Foreign key constraints

use std::time::Duration;

mod common;
use common::{create_test_db, create_in_memory_db, TestServerFixture};

use easyssh_core::db::Database;
use easyssh_core::models::{Server, Group, Identity, ServerUpdate, GroupUpdate};
use tempfile::TempDir;

/// Test database initialization and migrations
#[test]
fn test_database_initialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).expect("Should create database");
    db.init().expect("Should initialize");

    // Verify tables exist
    let tables = db.list_tables().expect("Should list tables");
    assert!(tables.contains(&"servers".to_string()));
    assert!(tables.contains(&"groups".to_string()));
    assert!(tables.contains(&"identities".to_string()));
    assert!(tables.contains(&"server_tags".to_string()));

    // Verify indexes exist
    let indexes = db.list_indexes().expect("Should list indexes");
    assert!(indexes.iter().any(|i| i.contains("servers_name")));
    assert!(indexes.iter().any(|i| i.contains("servers_host")));
}

/// Test server CRUD operations
#[test]
fn test_server_crud() {
    let (db, _temp) = create_test_db();

    // Create
    let server = Server::new("Test Server", "192.168.1.100", 22)
        .with_username("admin")
        .with_password("secret123")
        .with_tags(vec!["production".to_string(), "web".to_string()]);

    let id = db.add_server(&server).expect("Should add server");
    assert!(!id.is_empty());

    // Read
    let retrieved = db.get_server(&id).expect("Should get server");
    assert_eq!(retrieved.name, "Test Server");
    assert_eq!(retrieved.host, "192.168.1.100");
    assert_eq!(retrieved.port, 22);
    assert_eq!(retrieved.username, "admin");
    assert_eq!(retrieved.tags.len(), 2);
    assert!(retrieved.tags.contains(&"production".to_string()));

    // Update
    let update = ServerUpdate::new()
        .name("Updated Server")
        .host("192.168.1.200")
        .port(2222);

    db.update_server(&id, &update).expect("Should update server");

    let updated = db.get_server(&id).expect("Should get updated server");
    assert_eq!(updated.name, "Updated Server");
    assert_eq!(updated.host, "192.168.1.200");
    assert_eq!(updated.port, 2222);
    // Other fields should be preserved
    assert_eq!(updated.username, "admin");

    // Delete
    db.delete_server(&id).expect("Should delete server");
    assert!(db.get_server(&id).is_err());
}

/// Test group CRUD operations
#[test]
fn test_group_crud() {
    let (db, _temp) = create_test_db();

    // Create
    let group = Group::new("Production", "#ff0000");
    let id = db.add_group(&group).expect("Should add group");

    // Read
    let retrieved = db.get_group(&id).expect("Should get group");
    assert_eq!(retrieved.name, "Production");
    assert_eq!(retrieved.color, "#ff0000");

    // Update
    let update = GroupUpdate::new().name("Prod").color("#cc0000");
    db.update_group(&id, &update).expect("Should update group");

    let updated = db.get_group(&id).expect("Should get updated group");
    assert_eq!(updated.name, "Prod");

    // Delete
    db.delete_group(&id).expect("Should delete group");
    assert!(db.get_group(&id).is_err());
}

/// Test server-group relationships
#[test]
fn test_server_group_relationships() {
    let (db, _temp) = create_test_db();

    // Create groups
    let prod_group = Group::new("Production", "#ff0000");
    let prod_id = db.add_group(&prod_group).expect("Should add group");

    let dev_group = Group::new("Development", "#00ff00");
    let dev_id = db.add_group(&dev_group).expect("Should add group");

    // Create servers in groups
    let prod_server = Server::new("Prod Server", "192.168.1.10", 22)
        .with_group(&prod_id);
    let prod_srv_id = db.add_server(&prod_server).expect("Should add server");

    let dev_server = Server::new("Dev Server", "192.168.1.20", 22)
        .with_group(&dev_id);
    let dev_srv_id = db.add_server(&dev_server).expect("Should add server");

    // Get servers by group
    let prod_servers = db.get_servers_by_group(&prod_id).expect("Should get servers");
    assert_eq!(prod_servers.len(), 1);
    assert_eq!(prod_servers[0].id, prod_srv_id);

    let dev_servers = db.get_servers_by_group(&dev_id).expect("Should get servers");
    assert_eq!(dev_servers.len(), 1);
    assert_eq!(dev_servers[0].id, dev_srv_id);

    // Move server to different group
    db.move_server_to_group(&dev_srv_id, &prod_id).expect("Should move server");

    let prod_servers = db.get_servers_by_group(&prod_id).expect("Should get servers");
    assert_eq!(prod_servers.len(), 2);

    // Remove from group
    db.remove_server_from_group(&prod_srv_id).expect("Should remove from group");

    let prod_servers = db.get_servers_by_group(&prod_id).expect("Should get servers");
    assert_eq!(prod_servers.len(), 1);
}

/// Test identity management
#[test]
fn test_identity_management() {
    let (db, _temp) = create_test_db();

    // Create identity with password
    let password_identity = Identity::new_password("My Password", "secret123");
    let pwd_id = db.add_identity(&password_identity).expect("Should add identity");

    // Create identity with SSH key
    let key_identity = Identity::new_key(
        "My Key",
        "~/.ssh/id_rsa",
        Some("key_passphrase")
    );
    let key_id = db.add_identity(&key_identity).expect("Should add identity");

    // List identities
    let identities = db.get_all_identities().expect("Should get identities");
    assert_eq!(identities.len(), 2);

    // Get specific identity
    let retrieved = db.get_identity(&key_id).expect("Should get identity");
    assert_eq!(retrieved.name, "My Key");
    assert_eq!(retrieved.private_key_path, Some("~/.ssh/id_rsa".to_string()));

    // Update identity
    let updated = Identity::new_key("Updated Key", "~/.ssh/id_ed25519", None);
    db.update_identity(&key_id, &updated).expect("Should update");

    let retrieved = db.get_identity(&key_id).expect("Should get updated");
    assert_eq!(retrieved.name, "Updated Key");

    // Delete identity
    db.delete_identity(&pwd_id).expect("Should delete");
    assert!(db.get_identity(&pwd_id).is_err());
}

/// Test server-identity relationships
#[test]
fn test_server_identity_relationships() {
    let (db, _temp) = create_test_db();

    // Create identity
    let identity = Identity::new_key("Server Key", "~/.ssh/id_rsa", None);
    let identity_id = db.add_identity(&identity).expect("Should add identity");

    // Create server with identity
    let server = Server::new("Server with Key", "192.168.1.1", 22)
        .with_identity(&identity_id);
    let server_id = db.add_server(&server).expect("Should add server");

    // Get server and verify identity
    let retrieved = db.get_server(&server_id).expect("Should get server");
    assert_eq!(retrieved.identity_id, Some(identity_id));

    // Update server identity
    let new_identity = Identity::new_password("New Identity", "newpass");
    let new_id = db.add_identity(&new_identity).expect("Should add identity");

    db.assign_identity_to_server(&server_id, &new_id).expect("Should assign");

    let retrieved = db.get_server(&server_id).expect("Should get server");
    assert_eq!(retrieved.identity_id, Some(new_id));

    // Remove identity
    db.remove_identity_from_server(&server_id).expect("Should remove");

    let retrieved = db.get_server(&server_id).expect("Should get server");
    assert!(retrieved.identity_id.is_none());
}

/// Test search functionality in database
#[test]
fn test_database_search() {
    let (db, _temp) = create_test_db();

    // Add test servers
    let servers = vec![
        Server::new("Web Production", "192.168.1.10", 22),
        Server::new("Database Production", "192.168.1.11", 3306),
        Server::new("Web Development", "dev.web.local", 22),
        Server::new("Staging Server", "192.168.2.50", 22),
    ];

    for server in &servers {
        db.add_server(server).expect("Should add server");
    }

    // Search by name
    let results = db.search_servers("Production").expect("Should search");
    assert_eq!(results.len(), 2);

    // Search by host
    let results = db.search_servers("192.168.1").expect("Should search");
    assert_eq!(results.len(), 2);

    // Search by partial name
    let results = db.search_servers("Web").expect("Should search");
    assert_eq!(results.len(), 2);

    // Case insensitive search
    let results = db.search_servers("PRODUCTION").expect("Should search");
    assert_eq!(results.len(), 2);

    // No results
    let results = db.search_servers("nonexistent").expect("Should search");
    assert!(results.is_empty());
}

/// Test transaction handling
#[test]
fn test_transaction_handling() {
    let (db, _temp) = create_test_db();

    // Begin transaction
    let tx = db.begin_transaction().expect("Should begin transaction");

    // Add servers in transaction
    let server1 = Server::new("Server 1", "192.168.1.1", 22);
    let server2 = Server::new("Server 2", "192.168.1.2", 22);

    let id1 = tx.add_server(&server1).expect("Should add in tx");
    let id2 = tx.add_server(&server2).expect("Should add in tx");

    // Servers should not be visible outside transaction yet
    let outside_servers = db.get_all_servers().expect("Should get servers");
    assert!(outside_servers.is_empty());

    // Commit transaction
    tx.commit().expect("Should commit");

    // Now servers should be visible
    let servers = db.get_all_servers().expect("Should get servers");
    assert_eq!(servers.len(), 2);
    assert!(servers.iter().any(|s| s.id == id1));
    assert!(servers.iter().any(|s| s.id == id2));
}

/// Test transaction rollback
#[test]
fn test_transaction_rollback() {
    let (db, _temp) = create_test_db();

    // Begin transaction
    let tx = db.begin_transaction().expect("Should begin transaction");

    // Add server in transaction
    let server = Server::new("Temp Server", "192.168.1.1", 22);
    tx.add_server(&server).expect("Should add in tx");

    // Rollback
    tx.rollback().expect("Should rollback");

    // Server should not exist
    let servers = db.get_all_servers().expect("Should get servers");
    assert!(servers.is_empty());
}

/// Test pagination
#[test]
fn test_pagination() {
    let (db, _temp) = create_test_db();

    // Add many servers
    for i in 0..100 {
        let server = Server::new(&format!("Server {}", i), "192.168.1.1", 22);
        db.add_server(&server).expect("Should add server");
    }

    // Test pagination
    let page1 = db.get_servers_paginated(0, 10).expect("Should get page");
    assert_eq!(page1.len(), 10);

    let page2 = db.get_servers_paginated(10, 10).expect("Should get page");
    assert_eq!(page2.len(), 10);

    // Verify different pages
    assert_ne!(page1[0].id, page2[0].id);

    // Last page may have fewer items
    let last_page = db.get_servers_paginated(90, 20).expect("Should get page");
    assert_eq!(last_page.len(), 10); // Only 10 remaining
}

/// Test bulk operations
#[test]
fn test_bulk_operations() {
    let (db, _temp) = create_test_db();

    // Bulk insert
    let servers: Vec<_> = (0..50)
        .map(|i| Server::new(&format!("Server {}", i), "192.168.1.1", 22))
        .collect();

    let ids = db.add_servers_bulk(&servers).expect("Should bulk insert");
    assert_eq!(ids.len(), 50);

    // Bulk delete
    let ids_to_delete: Vec<_> = ids.iter().take(10).cloned().collect();
    db.delete_servers_bulk(&ids_to_delete).expect("Should bulk delete");

    let remaining = db.get_all_servers().expect("Should get servers");
    assert_eq!(remaining.len(), 40);
}

/// Test import/export operations
#[test]
fn test_import_export() {
    let (db, _temp) = create_test_db();

    // Add test data
    let group = Group::new("Test Group", "#ff0000");
    let group_id = db.add_group(&group).expect("Should add group");

    let server = Server::new("Export Test", "192.168.1.1", 22)
        .with_group(&group_id)
        .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
    db.add_server(&server).expect("Should add server");

    // Export all data
    let export = db.export_all_data().expect("Should export");
    assert!(!export.servers.is_empty());
    assert!(!export.groups.is_empty());

    // Create new database and import
    let temp_dir2 = TempDir::new().expect("Failed to create temp directory");
    let db2 = Database::new(temp_dir2.path().join("test2.db")).expect("Should create DB");
    db2.init().expect("Should init");

    db2.import_all_data(&export).expect("Should import");

    // Verify imported data
    let servers = db2.get_all_servers().expect("Should get servers");
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "Export Test");

    let groups = db2.get_all_groups().expect("Should get groups");
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "Test Group");
}

/// Test data integrity constraints
#[test]
fn test_data_integrity() {
    let (db, _temp) = create_test_db();

    // Try to create server with non-existent group
    let server = Server::new("Test", "192.168.1.1", 22)
        .with_group("non-existent-group");

    let result = db.add_server(&server);
    // Should fail due to foreign key constraint
    assert!(result.is_err());

    // Try to delete group with servers
    let group = Group::new("Parent Group", "#ff0000");
    let group_id = db.add_group(&group).expect("Should add group");

    let server = Server::new("Test", "192.168.1.1", 22)
        .with_group(&group_id);
    db.add_server(&server).expect("Should add server");

    // Should fail to delete group with servers
    let result = db.delete_group(&group_id);
    assert!(result.is_err());
}

/// Test connection pool with database
#[test]
fn test_concurrent_database_access() {
    use std::sync::Arc;
    use std::thread;

    let (db, _temp) = create_test_db();
    let db = Arc::new(std::sync::Mutex::new(db));

    let mut handles = vec![];

    // Spawn threads that perform database operations
    for i in 0..10 {
        let db_clone = Arc::clone(&db);
        let handle = thread::spawn(move || {
            for j in 0..10 {
                let server = Server::new(
                    &format!("Thread{}-Server{}", i, j),
                    "192.168.1.1",
                    22
                );
                db_clone.lock().unwrap().add_server(&server).expect("Should add");
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    // Verify all servers were added
    let servers = db.lock().unwrap().get_all_servers().expect("Should get servers");
    assert_eq!(servers.len(), 100);
}
