//! Search Service Unit Tests
//!
//! Tests for server search functionality including:
//! - Full-text search
//! - Fuzzy matching
//! - Filters (auth method, status, group)
//! - Sorting
//! - Pagination

use std::sync::{Arc, Mutex};

use easyssh_core::services::search_service::{SearchService, SearchFilter, SortBy, SortOrder, AuthMethod as SearchAuthMethod, ConnectionStatus};
use easyssh_core::models::server::{CreateServerDto, AuthMethod};
use easyssh_core::db::{Database, NewServer, NewGroup, NewHost};

mod common;
use common::{create_test_db, create_test_db_arc};

fn create_search_service() -> (SearchService, Arc<Mutex<Database>>, tempfile::TempDir) {
    let (db_arc, temp) = create_test_db_arc();
    let service = SearchService::new(db_arc.clone());
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
        ("host-001", "Web Server", "192.168.1.10", "web.example.com"),
        ("host-002", "Database Server", "192.168.1.11", "db.example.com"),
        ("host-003", "Cache Server", "192.168.1.12", "cache.example.com"),
    ];

    {
        let db = db.lock().unwrap();
        for (id, name, ip, hostname) in &hosts {
            let host = NewHost {
                id,
                name,
                host: *hostname,
                port: 22,
                username: "admin",
                auth_type: "password",
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
            db.create_host(&host).expect("Create host should succeed");
        }
    }

    // Search for "web"
    let filter = SearchFilter {
        query: Some("web".to_string()),
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Web Server");
}

#[test]
fn test_fuzzy_search() {
    let (service, db, _temp) = create_search_service();

    // Create hosts
    {
        let db = db.lock().unwrap();
        let hosts = vec![
            ("host-001", "Production Web Server", "192.168.1.10"),
            ("host-002", "Development Server", "192.168.1.11"),
            ("host-003", "Staging Environment", "192.168.1.12"),
        ];

        for (id, name, host) in &hosts {
            let new_host = NewHost {
                id,
                name,
                host: *host,
                port: 22,
                username: "admin",
                auth_type: "password",
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
    }

    // Fuzzy search should find partial matches
    let filter = SearchFilter {
        query: Some("prod".to_string()),
        fuzzy: true,
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert!(results.iter().any(|h| h.name.contains("Production")));
}

#[test]
fn test_search_by_ip() {
    let (service, db, _temp) = create_search_service();

    // Create hosts
    {
        let db = db.lock().unwrap();
        let hosts = vec![
            ("host-001", "Server 1", "10.0.0.1"),
            ("host-002", "Server 2", "10.0.0.2"),
            ("host-003", "Server 3", "192.168.1.1"),
        ];

        for (id, name, host) in &hosts {
            let new_host = NewHost {
                id,
                name,
                host: *host,
                port: 22,
                username: "admin",
                auth_type: "password",
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
    }

    // Search by IP prefix
    let filter = SearchFilter {
        query: Some("10.0.0".to_string()),
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_search_by_auth_method() {
    let (service, db, _temp) = create_search_service();

    // Create hosts with different auth methods
    {
        let db = db.lock().unwrap();
        let hosts = vec![
            ("host-001", "Password Auth", "password"),
            ("host-002", "Key Auth", "key"),
            ("host-003", "Agent Auth", "agent"),
            ("host-004", "Another Password", "password"),
        ];

        for (id, name, auth_type) in &hosts {
            let new_host = NewHost {
                id,
                name,
                host: "192.168.1.1",
                port: 22,
                username: "admin",
                auth_type,
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
    }

    // Filter by password auth
    let filter = SearchFilter {
        auth_methods: Some(vec![SearchAuthMethod::Password]),
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|h| h.auth_type == "password"));
}

#[test]
fn test_search_sorting() {
    let (service, db, _temp) = create_search_service();

    // Create hosts
    {
        let db = db.lock().unwrap();
        let hosts = vec![
            ("host-003", "Zebra Server"),
            ("host-001", "Alpha Server"),
            ("host-002", "Beta Server"),
        ];

        for (id, name) in &hosts {
            let new_host = NewHost {
                id,
                name,
                host: "192.168.1.1",
                port: 22,
                username: "admin",
                auth_type: "password",
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
    }

    // Sort by name ascending
    let filter = SearchFilter {
        sort_by: SortBy::Name,
        sort_order: SortOrder::Asc,
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results[0].name, "Alpha Server");
    assert_eq!(results[1].name, "Beta Server");
    assert_eq!(results[2].name, "Zebra Server");

    // Sort by name descending
    let filter = SearchFilter {
        sort_by: SortBy::Name,
        sort_order: SortOrder::Desc,
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results[0].name, "Zebra Server");
    assert_eq!(results[1].name, "Beta Server");
    assert_eq!(results[2].name, "Alpha Server");
}

#[test]
fn test_search_pagination() {
    let (service, db, _temp) = create_search_service();

    // Create 10 hosts
    {
        let db = db.lock().unwrap();
        for i in 0..10 {
            let new_host = NewHost {
                id: &format!("host-{:03}", i),
                name: &format!("Server {}", i),
                host: &format!("192.168.1.{}", i),
                port: 22,
                username: "admin",
                auth_type: "password",
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
    }

    // Get first page (5 items)
    let filter = SearchFilter {
        limit: Some(5),
        offset: Some(0),
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 5);

    // Get second page
    let filter = SearchFilter {
        limit: Some(5),
        offset: Some(5),
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 5);
}

#[test]
fn test_search_by_group() {
    let (service, db, _temp) = create_search_service();

    // Create group
    {
        let db = db.lock().unwrap();
        let group = NewGroup {
            id: "prod-group",
            name: "Production",
            color: Some("#ff0000"),
        };
        db.create_group(&group).expect("Create group should succeed");
    }

    // Create hosts
    {
        let db = db.lock().unwrap();
        let hosts = vec![
            ("host-001", "Prod Server 1", Some("prod-group")),
            ("host-002", "Prod Server 2", Some("prod-group")),
            ("host-003", "Dev Server", None),
        ];

        for (id, name, group_id) in &hosts {
            let new_host = NewHost {
                id,
                name,
                host: "192.168.1.1",
                port: 22,
                username: "admin",
                auth_type: "password",
                identity_file: None,
                identity_id: None,
                group_id: *group_id,
                notes: None,
                color: None,
                environment: None,
                region: None,
                purpose: None,
                status: "unknown",
            };
            db.create_host(&new_host).expect("Create host should succeed");
        }
    }

    // Filter by group
    let filter = SearchFilter {
        group_ids: Some(vec!["prod-group".to_string()]),
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|h| h.name.contains("Prod")));
}

#[test]
fn test_search_by_status() {
    let (service, db, _temp) = create_search_service();

    // Create hosts with different statuses
    {
        let db = db.lock().unwrap();
        let hosts = vec![
            ("host-001", "Online Server", "online"),
            ("host-002", "Offline Server", "offline"),
            ("host-003", "Another Online", "online"),
        ];

        for (id, name, status) in &hosts {
            let new_host = NewHost {
                id,
                name,
                host: "192.168.1.1",
                port: 22,
                username: "admin",
                auth_type: "password",
                identity_file: None,
                identity_id: None,
                group_id: None,
                notes: None,
                color: None,
                environment: None,
                region: None,
                purpose: None,
                status,
            };
            db.create_host(&new_host).expect("Create host should succeed");
        }
    }

    // Filter by online status
    let filter = SearchFilter {
        connection_status: Some(vec![ConnectionStatus::Online]),
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|h| h.status == "online"));
}

#[test]
fn test_search_combined_filters() {
    let (service, db, _temp) = create_search_service();

    // Create group and hosts
    {
        let db = db.lock().unwrap();

        let group = NewGroup {
            id: "web-group",
            name: "Web Servers",
            color: None,
        };
        db.create_group(&group).expect("Create group should succeed");

        let hosts = vec![
            ("host-001", "Web Server 1", "password", "web-group", "online"),
            ("host-002", "Web Server 2", "key", "web-group", "online"),
            ("host-003", "Other Server", "password", None, "offline"),
        ];

        for (id, name, auth_type, group_id, status) in &hosts {
            let new_host = NewHost {
                id,
                name,
                host: "192.168.1.1",
                port: 22,
                username: "admin",
                auth_type,
                identity_file: None,
                identity_id: None,
                group_id: *group_id,
                notes: None,
                color: None,
                environment: None,
                region: None,
                purpose: None,
                status,
            };
            db.create_host(&new_host).expect("Create host should succeed");
        }
    }

    // Combined filter: group + auth method + status
    let filter = SearchFilter {
        group_ids: Some(vec!["web-group".to_string()]),
        auth_methods: Some(vec![SearchAuthMethod::Password]),
        connection_status: Some(vec![ConnectionStatus::Online]),
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Web Server 1");
}

#[test]
fn test_search_history() {
    let (service, _, _temp) = create_search_service();

    // Record some searches
    service.record_search("web servers").unwrap();
    service.record_search("database").unwrap();
    service.record_search("web servers").unwrap(); // Duplicate

    // Get search history
    let history = service.get_search_history(10).unwrap();
    assert_eq!(history.len(), 2); // Should deduplicate
    assert_eq!(history[0], "web servers"); // Most recent first
    assert_eq!(history[1], "database");
}

#[test]
fn test_clear_search_history() {
    let (service, _, _temp) = create_search_service();

    // Record searches
    service.record_search("search1").unwrap();
    service.record_search("search2").unwrap();

    // Clear history
    service.clear_search_history().unwrap();

    // Verify cleared
    let history = service.get_search_history(10).unwrap();
    assert!(history.is_empty());
}

#[test]
fn test_search_empty_query_returns_all() {
    let (service, db, _temp) = create_search_service();

    // Create hosts
    {
        let db = db.lock().unwrap();
        for i in 0..5 {
            let new_host = NewHost {
                id: &format!("host-{:03}", i),
                name: &format!("Server {}", i),
                host: "192.168.1.1",
                port: 22,
                username: "admin",
                auth_type: "password",
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
    }

    // Empty query should return all
    let filter = SearchFilter {
        query: None,
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert_eq!(results.len(), 5);
}

#[test]
fn test_search_no_results() {
    let (service, db, _temp) = create_search_service();

    // Create a host
    {
        let db = db.lock().unwrap();
        let new_host = NewHost {
            id: "host-001",
            name: "Server",
            host: "192.168.1.1",
            port: 22,
            username: "admin",
            auth_type: "password",
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

    // Search for non-existent term
    let filter = SearchFilter {
        query: Some("nonexistentterm12345".to_string()),
        ..Default::default()
    };

    let results = service.search_hosts(&filter).unwrap();
    assert!(results.is_empty());
}
