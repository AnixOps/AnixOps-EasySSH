//! Search Service Unit Tests
//!
//! Tests for server search functionality including:
//! - Full-text search
//! - Fuzzy matching
//! - Filters (auth method, status, group)
//! - Sorting
//! - Pagination

use std::sync::Arc;

use easyssh_core::db::NewHost;
use easyssh_core::services::search_service::{SearchQuery, SearchService};

#[path = "../common/mod.rs"]
mod common;
use common::create_test_db_arc_direct;

fn create_search_service() -> (
    SearchService,
    Arc<easyssh_core::db::Database>,
    tempfile::TempDir,
) {
    let (db_arc, temp) = create_test_db_arc_direct();
    let service = SearchService::new(Arc::clone(&db_arc)).expect("SearchService creation failed");
    (service, db_arc, temp)
}

#[test]
fn test_search_service_creation() {
    let (service, _, _temp) = create_search_service();
    drop(service);
}

#[test]
fn test_basic_search() {
    let (service, db, _temp) = create_search_service();

    // Create test hosts
    let hosts = vec![
        ("host-001", "Web Server", "192.168.1.10"),
        ("host-002", "Database Server", "192.168.1.11"),
        ("host-003", "Cache Server", "192.168.1.12"),
    ];

    for (id, name, host) in &hosts {
        let new_host = NewHost {
            id: id.to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "agent".to_string(),
            identity_file: None,
            identity_id: None,
            group_id: None,
            notes: None,
            color: None,
            environment: None,
            region: None,
            purpose: None,
            status: "unknown".to_string(),
        };
        db.add_host(&new_host).expect("Create host should succeed");
    }

    // Build index
    service.rebuild_index().expect("Build index should succeed");

    // Search for "web"
    let query = SearchQuery {
        keyword: Some("web".to_string()),
        ..Default::default()
    };

    let results = service.search(&query).expect("Search should succeed");
    assert!(results.iter().any(|r| r.host.name.contains("Web")));
}

#[test]
fn test_search_by_ip() {
    let (service, db, _temp) = create_search_service();

    // Create hosts
    let hosts = vec![
        ("host-001", "Server 1", "10.0.0.1"),
        ("host-002", "Server 2", "10.0.0.2"),
        ("host-003", "Server 3", "192.168.1.1"),
    ];

    for (id, name, host) in &hosts {
        let new_host = NewHost {
            id: id.to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "agent".to_string(),
            identity_file: None,
            identity_id: None,
            group_id: None,
            notes: None,
            color: None,
            environment: None,
            region: None,
            purpose: None,
            status: "unknown".to_string(),
        };
        db.add_host(&new_host).expect("Create host should succeed");
    }

    // Build index
    service.rebuild_index().expect("Build index should succeed");

    // Search by IP prefix
    let query = SearchQuery {
        keyword: Some("10.0.0".to_string()),
        ..Default::default()
    };

    let results = service.search(&query).expect("Search should succeed");
    assert_eq!(results.len(), 2);
}

#[test]
fn test_search_empty_query() {
    let (service, db, _temp) = create_search_service();

    // Create hosts
    let hosts = vec![
        ("host-001", "Server 1", "192.168.1.1"),
        ("host-002", "Server 2", "192.168.1.2"),
    ];

    for (id, name, host) in &hosts {
        let new_host = NewHost {
            id: id.to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "agent".to_string(),
            identity_file: None,
            identity_id: None,
            group_id: None,
            notes: None,
            color: None,
            environment: None,
            region: None,
            purpose: None,
            status: "unknown".to_string(),
        };
        db.add_host(&new_host).expect("Create host should succeed");
    }

    // Build index
    service.rebuild_index().expect("Build index should succeed");

    // Empty query should return all hosts
    let query = SearchQuery::default();

    let results = service.search(&query).expect("Search should succeed");
    assert!(results.len() >= 2);
}

#[test]
fn test_index_rebuild() {
    let (service, db, _temp) = create_search_service();

    // Create initial host
    let new_host = NewHost {
        id: "host-001".to_string(),
        name: "Initial Server".to_string(),
        host: "192.168.1.1".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "agent".to_string(),
        identity_file: None,
        identity_id: None,
        group_id: None,
        notes: None,
        color: None,
        environment: None,
        region: None,
        purpose: None,
        status: "unknown".to_string(),
    };
    db.add_host(&new_host).expect("Create host should succeed");

    // Build initial index
    service.rebuild_index().expect("Build index should succeed");

    // Add more hosts
    let new_host = NewHost {
        id: "host-002".to_string(),
        name: "Second Server".to_string(),
        host: "192.168.1.2".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "agent".to_string(),
        identity_file: None,
        identity_id: None,
        group_id: None,
        notes: None,
        color: None,
        environment: None,
        region: None,
        purpose: None,
        status: "unknown".to_string(),
    };
    db.add_host(&new_host).expect("Create host should succeed");

    // Rebuild index
    service
        .rebuild_index()
        .expect("Rebuild index should succeed");

    // Search should find both hosts
    let query = SearchQuery {
        keyword: Some("Server".to_string()),
        ..Default::default()
    };

    let results = service.search(&query).expect("Search should succeed");
    assert!(results.len() >= 2);
}
