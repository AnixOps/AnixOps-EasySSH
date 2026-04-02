//! Server Service Unit Tests
//!
//! Tests for server management business logic including:
//! - CRUD operations
//! - Validation
//! - Import/Export functionality
//! - Search and filtering
//! - Duplicate detection

use std::sync::{Arc, Mutex};

use easyssh_core::services::server_service::{ServerService, ServerServiceError, ImportResult};
use easyssh_core::models::server::{CreateServerDto, ServerBuilder, AuthMethod, ServerStatus};
use easyssh_core::db::{Database, NewServer, NewGroup};

mod common;
use common::{create_test_db, create_test_db_arc, TestServerFixture};

fn create_server_service() -> (ServerService, Arc<Mutex<Database>>, tempfile::TempDir) {
    let (db_arc, temp) = create_test_db_arc();
    let service = ServerService::new(db_arc.clone());
    (service, db_arc, temp)
}

#[test]
fn test_server_service_creation() {
    let (service, _, _temp) = create_server_service();
    drop(service);
}

#[test]
fn test_create_server_success() {
    let (service, _, _temp) = create_server_service();

    let dto = CreateServerDto {
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    let result = service.create_server(dto);
    assert!(result.is_ok(), "Create server should succeed: {:?}", result.err());

    let server = result.unwrap();
    assert_eq!(server.name, "Test Server");
    assert_eq!(server.host, "192.168.1.100");
    assert_eq!(server.port, 22);
    assert_eq!(server.username, "admin");
}

#[test]
fn test_create_server_validation_empty_name() {
    let (service, _, _temp) = create_server_service();

    let dto = CreateServerDto {
        name: "".to_string(),
        host: "192.168.1.100".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    let result = service.create_server(dto);
    assert!(result.is_err(), "Create server with empty name should fail");

    match result.unwrap_err() {
        ServerServiceError::Validation(_) => {},
        _ => panic!("Expected Validation error"),
    }
}

#[test]
fn test_create_server_validation_empty_host() {
    let (service, _, _temp) = create_server_service();

    let dto = CreateServerDto {
        name: "Test Server".to_string(),
        host: "".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    let result = service.create_server(dto);
    assert!(result.is_err(), "Create server with empty host should fail");
}

#[test]
fn test_create_server_validation_empty_username() {
    let (service, _, _temp) = create_server_service();

    let dto = CreateServerDto {
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: Some(22),
        username: "".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    let result = service.create_server(dto);
    assert!(result.is_err(), "Create server with empty username should fail");
}

#[test]
fn test_create_server_default_port() {
    let (service, _, _temp) = create_server_service();

    let dto = CreateServerDto {
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: None, // Should default to 22
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    let result = service.create_server(dto);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().port, 22);
}

#[test]
fn test_get_server_by_id() {
    let (service, _, _temp) = create_server_service();

    // Create a server first
    let dto = CreateServerDto {
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    let created = service.create_server(dto).expect("Create should succeed");
    let id = created.id.clone();

    // Get it back
    let result = service.get_server(&id);
    assert!(result.is_ok());

    let server = result.unwrap();
    assert!(server.is_some());
    assert_eq!(server.unwrap().name, "Test Server");
}

#[test]
fn test_get_nonexistent_server() {
    let (service, _, _temp) = create_server_service();

    let result = service.get_server("nonexistent-id");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_update_server() {
    let (service, _, _temp) = create_server_service();

    // Create a server
    let dto = CreateServerDto {
        name: "Original Name".to_string(),
        host: "192.168.1.100".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    let created = service.create_server(dto).expect("Create should succeed");
    let id = created.id.clone();

    // Update it
    let update = easyssh_core::models::server::UpdateServerDto {
        id: id.clone(),
        name: Some("Updated Name".to_string()),
        host: None,
        port: None,
        username: None,
        auth_method: None,
        group_id: None,
    };

    let result = service.update_server(update);
    assert!(result.is_ok(), "Update should succeed: {:?}", result.err());

    let updated = result.unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.host, "192.168.1.100"); // Unchanged
}

#[test]
fn test_update_nonexistent_server() {
    let (service, _, _temp) = create_server_service();

    let update = easyssh_core::models::server::UpdateServerDto {
        id: "nonexistent".to_string(),
        name: Some("New Name".to_string()),
        host: None,
        port: None,
        username: None,
        auth_method: None,
        group_id: None,
    };

    let result = service.update_server(update);
    assert!(result.is_err(), "Update nonexistent server should fail");

    match result.unwrap_err() {
        ServerServiceError::NotFound(_) => {},
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_delete_server() {
    let (service, _, _temp) = create_server_service();

    // Create a server
    let dto = CreateServerDto {
        name: "To Delete".to_string(),
        host: "192.168.1.100".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    let created = service.create_server(dto).expect("Create should succeed");
    let id = created.id.clone();

    // Verify it exists
    assert!(service.get_server(&id).unwrap().is_some());

    // Delete it
    let result = service.delete_server(&id);
    assert!(result.is_ok(), "Delete should succeed: {:?}", result.err());

    // Verify it's gone
    assert!(service.get_server(&id).unwrap().is_none());
}

#[test]
fn test_delete_nonexistent_server() {
    let (service, _, _temp) = create_server_service();

    let result = service.delete_server("nonexistent");
    assert!(result.is_err(), "Delete nonexistent server should fail");

    match result.unwrap_err() {
        ServerServiceError::NotFound(_) => {},
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_list_all_servers() {
    let (service, _, _temp) = create_server_service();

    // Create multiple servers
    for i in 0..5 {
        let dto = CreateServerDto {
            name: format!("Server {}", i),
            host: format!("192.168.1.{}", i),
            port: Some(22),
            username: "admin".to_string(),
            auth_method: AuthMethod::Password { password: "secret".to_string() },
            group_id: None,
        };
        service.create_server(dto).expect("Create should succeed");
    }

    let result = service.list_all_servers();
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 5);
}

#[test]
fn test_duplicate_name_detection() {
    let (service, _, _temp) = create_server_service();

    // Create first server
    let dto1 = CreateServerDto {
        name: "Unique Name".to_string(),
        host: "192.168.1.100".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };
    service.create_server(dto1).expect("First create should succeed");

    // Try to create second with same name
    let dto2 = CreateServerDto {
        name: "Unique Name".to_string(), // Same name
        host: "192.168.1.101".to_string(),
        port: Some(22),
        username: "root".to_string(),
        auth_method: AuthMethod::Agent,
        group_id: None,
    };

    let result = service.create_server(dto2);
    assert!(result.is_err(), "Duplicate name should fail");

    match result.unwrap_err() {
        ServerServiceError::DuplicateName(name) => assert_eq!(name, "Unique Name"),
        _ => panic!("Expected DuplicateName error"),
    }
}

#[test]
fn test_import_servers() {
    let (service, _, _temp) = create_server_service();

    let servers = vec![
        CreateServerDto {
            name: "Import 1".to_string(),
            host: "192.168.1.100".to_string(),
            port: Some(22),
            username: "admin".to_string(),
            auth_method: AuthMethod::Password { password: "pass1".to_string() },
            group_id: None,
        },
        CreateServerDto {
            name: "Import 2".to_string(),
            host: "192.168.1.101".to_string(),
            port: Some(2222),
            username: "root".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        },
    ];

    let result = service.import_servers(servers);
    assert!(result.is_ok(), "Import should succeed: {:?}", result.err());

    let import_result = result.unwrap();
    assert_eq!(import_result.total, 2);
    assert_eq!(import_result.imported, 2);
    assert_eq!(import_result.skipped, 0);
    assert!(import_result.errors.is_empty());

    // Verify servers were imported
    let all_servers = service.list_all_servers().unwrap();
    assert_eq!(all_servers.len(), 2);
}

#[test]
fn test_import_servers_with_duplicates() {
    let (service, _, _temp) = create_server_service();

    // Create existing server
    let existing = CreateServerDto {
        name: "Existing".to_string(),
        host: "192.168.1.100".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "pass".to_string() },
        group_id: None,
    };
    service.create_server(existing).expect("Create should succeed");

    // Import including a duplicate
    let servers = vec![
        CreateServerDto {
            name: "Existing".to_string(), // Duplicate name
            host: "192.168.1.100".to_string(),
            port: Some(22),
            username: "admin".to_string(),
            auth_method: AuthMethod::Password { password: "pass".to_string() },
            group_id: None,
        },
        CreateServerDto {
            name: "New Server".to_string(),
            host: "192.168.1.101".to_string(),
            port: Some(22),
            username: "root".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        },
    ];

    let result = service.import_servers(servers);
    assert!(result.is_ok());

    let import_result = result.unwrap();
    assert_eq!(import_result.total, 2);
    assert_eq!(import_result.imported, 1); // Only the new one
    assert_eq!(import_result.skipped, 1); // The duplicate
}

#[test]
fn test_search_servers() {
    let (service, _, _temp) = create_server_service();

    // Create servers with different names
    let servers = vec![
        ("Web Server", "192.168.1.10"),
        ("Database Server", "192.168.1.11"),
        ("Web Cache", "192.168.1.12"),
    ];

    for (name, host) in &servers {
        let dto = CreateServerDto {
            name: name.to_string(),
            host: host.to_string(),
            port: Some(22),
            username: "admin".to_string(),
            auth_method: AuthMethod::Password { password: "pass".to_string() },
            group_id: None,
        };
        service.create_server(dto).expect("Create should succeed");
    }

    // Search for "Web"
    let results = service.search_servers("Web", None).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|s| s.name == "Web Server"));
    assert!(results.iter().any(|s| s.name == "Web Cache"));

    // Search for "Database"
    let results = service.search_servers("Database", None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Database Server");

    // Search by IP
    let results = service.search_servers("192.168.1.11", None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].host, "192.168.1.11");
}

#[test]
fn test_server_status_management() {
    let (service, _, _temp) = create_server_service();

    // Create server
    let dto = CreateServerDto {
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    let created = service.create_server(dto).expect("Create should succeed");
    let id = created.id.clone();

    // Initial status should be Unknown
    assert_eq!(created.status, ServerStatus::Unknown);

    // Update status to Online
    let result = service.update_server_status(&id, ServerStatus::Online);
    assert!(result.is_ok());

    let updated = result.unwrap();
    assert_eq!(updated.status, ServerStatus::Online);
}

#[test]
fn test_get_servers_by_group() {
    let (service, db_arc, _temp) = create_server_service();

    // Create a group
    {
        let db = db_arc.lock().unwrap();
        let group = NewGroup {
            id: "test-group",
            name: "Test Group",
            color: None,
        };
        db.create_group(&group).expect("Create group should succeed");
    }

    // Create servers in the group
    for i in 0..3 {
        let dto = CreateServerDto {
            name: format!("Grouped Server {}", i),
            host: format!("192.168.1.{}", i),
            port: Some(22),
            username: "admin".to_string(),
            auth_method: AuthMethod::Password { password: "pass".to_string() },
            group_id: Some("test-group".to_string()),
        };
        service.create_server(dto).expect("Create should succeed");
    }

    // Create server not in group
    let other = CreateServerDto {
        name: "Other Server".to_string(),
        host: "192.168.2.1".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "pass".to_string() },
        group_id: None,
    };
    service.create_server(other).expect("Create should succeed");

    // Get servers by group
    let results = service.get_servers_by_group("test-group").unwrap();
    assert_eq!(results.len(), 3);
}
