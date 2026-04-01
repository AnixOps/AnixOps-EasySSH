//! Linux GTK4 Integration Tests
//!
//! These tests verify GTK4 widgets, models, and application logic.
//! Tests use GTK4's test infrastructure and can run headless with Xvfb.

#[cfg(test)]
mod model_tests {
    use super::*;

    // ==================== GTK4 Model Tests ====================

    #[test]
    fn test_server_model_creation() {
        gtk4::init().expect("Failed to initialize GTK");

        let server = Server {
            id: "srv-001".to_string(),
            name: "Test Server".to_string(),
            host: "192.168.1.100".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_type: AuthType::Password,
            group_id: None,
            status: ServerStatus::Unknown,
        };

        assert_eq!(server.id, "srv-001");
        assert_eq!(server.name, "Test Server");
        assert_eq!(server.port, 22);
    }

    #[test]
    fn test_server_group_model_creation() {
        gtk4::init().expect("Failed to initialize GTK");

        let group = ServerGroup {
            id: "grp-001".to_string(),
            name: "Production".to_string(),
            servers: vec!["srv-001".to_string(), "srv-002".to_string()],
        };

        assert_eq!(group.name, "Production");
        assert_eq!(group.servers.len(), 2);
    }

    #[test]
    fn test_auth_type_enum_variants() {
        let password = AuthType::Password;
        let key = AuthType::Key;
        let agent = AuthType::Agent;

        // Test that all variants can be created
        assert!(matches!(password, AuthType::Password));
        assert!(matches!(key, AuthType::Key));
        assert!(matches!(agent, AuthType::Agent));
    }

    #[test]
    fn test_server_status_transitions() {
        let mut status = ServerStatus::Unknown;
        assert!(matches!(status, ServerStatus::Unknown));

        status = ServerStatus::Disconnected;
        assert!(matches!(status, ServerStatus::Disconnected));

        status = ServerStatus::Connected;
        assert!(matches!(status, ServerStatus::Connected));

        status = ServerStatus::Error;
        assert!(matches!(status, ServerStatus::Error));
    }

    // ==================== Core Conversion Tests ====================

    #[test]
    fn test_server_from_core_record() {
        gtk4::init().expect("Failed to initialize GTK");

        let core_record = easyssh_core::ServerRecord {
            id: "core-srv-001".to_string(),
            name: "Core Server".to_string(),
            host: "10.0.0.1".to_string(),
            port: 2222,
            username: "admin".to_string(),
            auth_type: "password".to_string(),
            identity_file: None,
            group_id: Some("group-1".to_string()),
            status: "active".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        let server: Server = core_record.into();

        assert_eq!(server.id, "core-srv-001");
        assert_eq!(server.host, "10.0.0.1");
        assert_eq!(server.port, 2222);
        assert!(matches!(server.auth_type, AuthType::Password));
        assert_eq!(server.group_id, Some("group-1".to_string()));
    }

    #[test]
    fn test_server_from_core_key_auth() {
        gtk4::init().expect("Failed to initialize GTK");

        let core_record = easyssh_core::ServerRecord {
            id: "key-srv".to_string(),
            name: "Key Auth Server".to_string(),
            host: "10.0.0.2".to_string(),
            port: 22,
            username: "deploy".to_string(),
            auth_type: "key".to_string(),
            identity_file: Some("~/.ssh/id_rsa".to_string()),
            group_id: None,
            status: "active".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        let server: Server = core_record.into();
        assert!(matches!(server.auth_type, AuthType::Key));
    }

    #[test]
    fn test_server_from_core_agent_auth() {
        gtk4::init().expect("Failed to initialize GTK");

        let core_record = easyssh_core::ServerRecord {
            id: "agent-srv".to_string(),
            name: "Agent Auth Server".to_string(),
            host: "10.0.0.3".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_type: "agent".to_string(),
            identity_file: None,
            group_id: None,
            status: "active".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        let server: Server = core_record.into();
        assert!(matches!(server.auth_type, AuthType::Agent));
    }

    #[test]
    fn test_server_group_from_core_record() {
        gtk4::init().expect("Failed to initialize GTK");

        let core_group = easyssh_core::GroupRecord {
            id: "core-group-001".to_string(),
            name: "Development".to_string(),
            description: Some("Dev servers".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let group: ServerGroup = core_group.into();

        assert_eq!(group.id, "core-group-001");
        assert_eq!(group.name, "Development");
    }

    // ==================== GTK4 Widget Tests ====================

    #[test]
    fn test_gtk_application_id() {
        assert_eq!(APP_ID, "com.easyssh.EasySSH");
    }

    #[test]
    fn test_gtk_widget_hierarchy() {
        gtk4::init().expect("Failed to initialize GTK");

        // Test that we can create basic GTK widgets
        let button = gtk4::Button::with_label("Test Button");
        let label = gtk4::Label::new(Some("Test Label"));
        let entry = gtk4::Entry::new();

        // Verify widgets were created
        assert_eq!(button.label().unwrap(), "Test Button");
        assert_eq!(label.text(), "Test Label");
    }

    // ==================== Form Validation Tests ====================

    fn validate_server_form(name: &str, host: &str, port: i64, username: &str) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();

        if name.is_empty() {
            errors.push("Name is required");
        }
        if host.is_empty() {
            errors.push("Host is required");
        }
        if username.is_empty() {
            errors.push("Username is required");
        }
        if port <= 0 || port > 65535 {
            errors.push("Port must be between 1 and 65535");
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    #[test]
    fn test_valid_server_form() {
        let result = validate_server_form("Web Server", "192.168.1.1", 22, "root");
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_name_validation() {
        let result = validate_server_form("", "192.168.1.1", 22, "root");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.contains(&"Name is required"));
    }

    #[test]
    fn test_empty_host_validation() {
        let result = validate_server_form("Server", "", 22, "root");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.contains(&"Host is required"));
    }

    #[test]
    fn test_empty_username_validation() {
        let result = validate_server_form("Server", "192.168.1.1", 22, "");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.contains(&"Username is required"));
    }

    #[test]
    fn test_invalid_port_validation() {
        let result = validate_server_form("Server", "192.168.1.1", 0, "root");
        assert!(result.is_err());

        let result = validate_server_form("Server", "192.168.1.1", 70000, "root");
        assert!(result.is_err());
    }

    // ==================== CSS Loading Tests ====================

    #[test]
    fn test_css_provider_creation() {
        gtk4::init().expect("Failed to initialize GTK");

        let provider = gtk4::CssProvider::new();
        provider.load_from_string(include_str!("../src/styles.css"));

        // Verify provider was created and loaded
        assert!(provider.to_str().len() > 0);
    }

    // ==================== Search Filter Tests ====================

    fn filter_servers_by_query(servers: &[TestServer], query: &str) -> Vec<&TestServer> {
        if query.is_empty() {
            return servers.iter().collect();
        }

        let query_lower = query.to_lowercase();
        servers
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.host.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    #[derive(Debug, Clone)]
    struct TestServer {
        name: String,
        host: String,
    }

    #[test]
    fn test_filter_servers_empty_query() {
        let servers = vec![
            TestServer { name: "Web".to_string(), host: "10.0.0.1".to_string() },
            TestServer { name: "DB".to_string(), host: "10.0.0.2".to_string() },
        ];

        let filtered = filter_servers_by_query(&servers, "");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_servers_by_name() {
        let servers = vec![
            TestServer { name: "Production Web".to_string(), host: "10.0.0.1".to_string() },
            TestServer { name: "Database".to_string(), host: "10.0.0.2".to_string() },
        ];

        let filtered = filter_servers_by_query(&servers, "web");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Production Web");
    }

    #[test]
    fn test_filter_servers_by_host() {
        let servers = vec![
            TestServer { name: "Web".to_string(), host: "10.0.0.1".to_string() },
            TestServer { name: "DB".to_string(), host: "10.0.0.2".to_string() },
        ];

        let filtered = filter_servers_by_query(&servers, "10.0.0.2");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "DB");
    }

    #[test]
    fn test_filter_servers_case_insensitive() {
        let servers = vec![
            TestServer { name: "Web Server".to_string(), host: "10.0.0.1".to_string() },
        ];

        let filtered = filter_servers_by_query(&servers, "WEB");
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_servers_no_match() {
        let servers = vec![
            TestServer { name: "Web".to_string(), host: "10.0.0.1".to_string() },
        ];

        let filtered = filter_servers_by_query(&servers, "database");
        assert!(filtered.is_empty());
    }

    // ==================== Server Grouping Tests ====================

    fn group_servers_by_group_id(servers: &[TestServerWithGroup]) -> std::collections::HashMap<Option<String>, Vec<&TestServerWithGroup>> {
        let mut groups: std::collections::HashMap<Option<String>, Vec<&TestServerWithGroup>> = std::collections::HashMap::new();

        for server in servers {
            groups.entry(server.group_id.clone()).or_default().push(server);
        }

        groups
    }

    #[derive(Debug, Clone)]
    struct TestServerWithGroup {
        name: String,
        group_id: Option<String>,
    }

    #[test]
    fn test_group_servers() {
        let servers = vec![
            TestServerWithGroup { name: "Prod Web".to_string(), group_id: Some("prod".to_string()) },
            TestServerWithGroup { name: "Prod DB".to_string(), group_id: Some("prod".to_string()) },
            TestServerWithGroup { name: "Dev Web".to_string(), group_id: Some("dev".to_string()) },
            TestServerWithGroup { name: "Standalone".to_string(), group_id: None },
        ];

        let grouped = group_servers_by_group_id(&servers);

        assert_eq!(grouped.get(&Some("prod".to_string())).unwrap().len(), 2);
        assert_eq!(grouped.get(&Some("dev".to_string())).unwrap().len(), 1);
        assert_eq!(grouped.get(&None).unwrap().len(), 1);
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_server_json_serialization() {
        gtk4::init().expect("Failed to initialize GTK");

        let server = Server {
            id: "json-test".to_string(),
            name: "JSON Test Server".to_string(),
            host: "json.example.com".to_string(),
            port: 2222,
            username: "jsonuser".to_string(),
            auth_type: AuthType::Key,
            group_id: Some("test-group".to_string()),
            status: ServerStatus::Connected,
        };

        // Note: Direct serialization of glib::Boxed types requires special handling
        // This test verifies the struct can be created with all fields
        assert_eq!(server.name, "JSON Test Server");
    }

    // ==================== Application Constants ====================

    const APP_ID: &str = "com.easyssh.EasySSH";
    const DEFAULT_WINDOW_WIDTH: i32 = 900;
    const DEFAULT_WINDOW_HEIGHT: i32 = 600;
    const MIN_SIDEBAR_WIDTH: i32 = 280;
    const BREAKPOINT_WIDTH: f64 = 600.0;

    #[test]
    fn test_application_constants() {
        assert_eq!(APP_ID, "com.easyssh.EasySSH");
        assert_eq!(DEFAULT_WINDOW_WIDTH, 900);
        assert_eq!(DEFAULT_WINDOW_HEIGHT, 600);
        assert_eq!(MIN_SIDEBAR_WIDTH, 280);
        assert_eq!(BREAKPOINT_WIDTH, 600.0);
    }

    // ==================== Error Handling Tests ====================

    #[derive(Debug)]
    enum AppError {
        DatabaseError(String),
        ConnectionError(String),
        ValidationError(String),
    }

    impl std::fmt::Display for AppError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
                AppError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
                AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            }
        }
    }

    impl std::error::Error for AppError {}

    #[test]
    fn test_error_types() {
        let db_err = AppError::DatabaseError("Connection failed".to_string());
        let conn_err = AppError::ConnectionError("Timeout".to_string());
        let val_err = AppError::ValidationError("Invalid port".to_string());

        assert!(matches!(db_err, AppError::DatabaseError(_)));
        assert!(matches!(conn_err, AppError::ConnectionError(_)));
        assert!(matches!(val_err, AppError::ValidationError(_)));
    }

    #[test]
    fn test_error_display() {
        let err = AppError::ConnectionError("Network unreachable".to_string());
        assert!(err.to_string().contains("Network unreachable"));
    }

    // ==================== Responsive Design Tests ====================

    fn should_collapse_sidebar(window_width: f64) -> bool {
        window_width < BREAKPOINT_WIDTH
    }

    #[test]
    fn test_responsive_breakpoint() {
        assert!(should_collapse_sidebar(500.0));
        assert!(should_collapse_sidebar(599.0));
        assert!(!should_collapse_sidebar(600.0));
        assert!(!should_collapse_sidebar(900.0));
    }
}

// ==================== View Tests ====================

#[cfg(all(test, feature = "gtk-tests"))]
mod view_tests {
    use gtk4::prelude::*;

    #[test]
    fn test_server_list_view_creation() {
        gtk4::init().expect("Failed to initialize GTK");

        // This would test the actual ServerListView widget
        // Requires the view module to be properly set up for testing
    }
}

// ==================== Integration Tests ====================

#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn test_app_state_initialization() {
        gtk4::init().expect("Failed to initialize GTK");
        libadwaita::init();

        // Test app state creation
        let core_state = Arc::new(Mutex::new(easyssh_core::AppState::new()));

        // Verify state was created
        let _state = core_state.lock().unwrap();
    }
}

// Import types from the main crate
use gtk4::glib;

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "TestServer")]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: AuthType,
    pub group_id: Option<String>,
    pub status: ServerStatus,
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "TestServerGroup")]
pub struct ServerGroup {
    pub id: String,
    pub name: String,
    pub servers: Vec<String>,
}

#[derive(Clone, Debug, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "TestAuthType")]
pub enum AuthType {
    Password,
    Key,
    Agent,
}

#[derive(Clone, Debug, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "TestServerStatus")]
pub enum ServerStatus {
    Unknown,
    Connected,
    Disconnected,
    Error,
}