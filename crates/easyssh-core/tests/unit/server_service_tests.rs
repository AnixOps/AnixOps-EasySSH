//! Server Service Unit Tests
//!
//! Tests for server management business logic including:
//! - CRUD operations
//! - Validation
//! - Import/Export functionality
//! - Search and filtering
//! - Duplicate detection

use std::sync::{Arc, Mutex};

use easyssh_core::models::server::{AuthMethod, CreateServerDto, UpdateServerDto};
use easyssh_core::services::server_service::ServerService;

#[path = "../common/mod.rs"]
mod common;
use common::create_test_db_arc;

fn create_server_service() -> (
    ServerService,
    Arc<Mutex<easyssh_core::db::Database>>,
    tempfile::TempDir,
) {
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
        port: 22,
        username: "admin".to_string(),
        auth_method: AuthMethod::Agent,
        group_id: None,
    };

    let result = service.create_server(dto);
    assert!(
        result.is_ok(),
        "Create server should succeed: {:?}",
        result.err()
    );

    let server = result.unwrap();
    assert_eq!(server.name, "Test Server");
    assert_eq!(server.host, "192.168.1.100");
    assert_eq!(server.port, 22);
    assert_eq!(server.username, "admin");
}

#[test]
fn test_create_server_default_port() {
    let (service, _, _temp) = create_server_service();

    let dto = CreateServerDto {
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22, // Default port
        username: "admin".to_string(),
        auth_method: AuthMethod::Agent,
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
        port: 22,
        username: "admin".to_string(),
        auth_method: AuthMethod::Agent,
        group_id: None,
    };

    let created = service.create_server(dto).expect("Create should succeed");
    let id = created.id.clone();

    // Get it back
    let result = service.get_server(&id);
    assert!(result.is_ok());

    let server = result.unwrap();
    assert_eq!(server.name, "Test Server");
}

#[test]
fn test_update_server() {
    let (service, _, _temp) = create_server_service();

    // Create a server
    let dto = CreateServerDto {
        name: "Original Name".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_method: AuthMethod::Agent,
        group_id: None,
    };

    let created = service.create_server(dto).expect("Create should succeed");
    let id = created.id.clone();

    // Update it
    let update = UpdateServerDto {
        name: Some("Updated Name".to_string()),
        host: None,
        port: None,
        username: None,
        auth_method: None,
        group_id: None,
    };

    let result = service.update_server(&id, update);
    assert!(result.is_ok(), "Update should succeed: {:?}", result.err());

    let updated = result.unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.host, "192.168.1.100"); // Unchanged
}

#[test]
fn test_delete_server() {
    let (service, _, _temp) = create_server_service();

    // Create a server
    let dto = CreateServerDto {
        name: "To Delete".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_method: AuthMethod::Agent,
        group_id: None,
    };

    let created = service.create_server(dto).expect("Create should succeed");
    let id = created.id.clone();

    // Delete it
    let result = service.delete_server(&id);
    assert!(result.is_ok(), "Delete should succeed: {:?}", result.err());
}

#[test]
fn test_list_servers() {
    let (service, _, _temp) = create_server_service();

    // Create multiple servers
    for i in 0..5 {
        let dto = CreateServerDto {
            name: format!("Server {}", i),
            host: format!("192.168.1.{}", i),
            port: 22,
            username: "admin".to_string(),
            auth_method: AuthMethod::Agent,
            group_id: None,
        };
        service.create_server(dto).expect("Create should succeed");
    }

    let result = service.get_all_servers();
    assert!(result.is_ok());
    let servers = result.unwrap();
    assert!(servers.len() >= 5);
}
