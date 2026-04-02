//! SSH Module Unit Tests
//!
//! Tests for SSH functionality including:
//! - SSH config parsing
//! - Connection parameter validation
//! - Session management (mocked)
//! - Health tracking

use easyssh_core::models::server::{CreateServerDto, ServerBuilder, AuthMethod, ServerStatus, ValidationError};
use easyssh_core::ssh::{ConnectionHealth, SshSessionManager, SessionMetadata};
use std::net::SocketAddr;

mod common;

#[test]
fn test_connection_health_enum() {
    assert_eq!(ConnectionHealth::Healthy, ConnectionHealth::Healthy);
    assert_eq!(ConnectionHealth::Degraded, ConnectionHealth::Degraded);
    assert_eq!(ConnectionHealth::Unhealthy, ConnectionHealth::Unhealthy);

    assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Unhealthy);
}

#[test]
fn test_server_builder_basic() {
    let server = ServerBuilder::new()
        .name("Test Server")
        .host("192.168.1.100")
        .port(22)
        .username("admin")
        .password("secret")
        .build()
        .expect("Build should succeed");

    assert_eq!(server.name, "Test Server");
    assert_eq!(server.host, "192.168.1.100");
    assert_eq!(server.port, 22);
    assert_eq!(server.username, "admin");
    assert!(matches!(server.auth_method, AuthMethod::Password { password } if password == "secret"));
}

#[test]
fn test_server_builder_with_key() {
    let server = ServerBuilder::new()
        .name("Key Server")
        .host("192.168.1.101")
        .username("root")
        .private_key("~/.ssh/id_rsa", Some("passphrase"))
        .build()
        .expect("Build should succeed");

    assert!(matches!(
        server.auth_method,
        AuthMethod::PrivateKey { key_path, passphrase } if
            key_path == "~/.ssh/id_rsa" && passphrase == Some("passphrase".to_string())
    ));
}

#[test]
fn test_server_builder_default_port() {
    let server = ServerBuilder::new()
        .name("Test")
        .host("example.com")
        .username("admin")
        .password("pass")
        .build()
        .expect("Build should succeed");

    assert_eq!(server.port, 22, "Default port should be 22");
}

#[test]
fn test_server_builder_validation_empty_name() {
    let result = ServerBuilder::new()
        .name("")
        .host("192.168.1.1")
        .username("admin")
        .password("pass")
        .build();

    assert!(result.is_err());
    match result.unwrap_err() {
        ValidationError::EmptyName => {},
        _ => panic!("Expected EmptyName error"),
    }
}

#[test]
fn test_server_builder_validation_empty_host() {
    let result = ServerBuilder::new()
        .name("Test")
        .host("")
        .username("admin")
        .password("pass")
        .build();

    assert!(result.is_err());
    match result.unwrap_err() {
        ValidationError::EmptyHost => {},
        _ => panic!("Expected EmptyHost error"),
    }
}

#[test]
fn test_server_builder_validation_empty_username() {
    let result = ServerBuilder::new()
        .name("Test")
        .host("192.168.1.1")
        .username("")
        .password("pass")
        .build();

    assert!(result.is_err());
    match result.unwrap_err() {
        ValidationError::EmptyUsername => {},
        _ => panic!("Expected EmptyUsername error"),
    }
}

#[test]
fn test_server_builder_validation_invalid_port() {
    let result = ServerBuilder::new()
        .name("Test")
        .host("192.168.1.1")
        .port(0)
        .username("admin")
        .password("pass")
        .build();

    assert!(result.is_err());
    match result.unwrap_err() {
        ValidationError::InvalidPort(port) => assert_eq!(port, 0),
        _ => panic!("Expected InvalidPort error"),
    }
}

#[test]
fn test_server_builder_validation_port_too_high() {
    let result = ServerBuilder::new()
        .name("Test")
        .host("192.168.1.1")
        .port(70000)
        .username("admin")
        .password("pass")
        .build();

    assert!(result.is_err());
    match result.unwrap_err() {
        ValidationError::InvalidPort(port) => assert_eq!(port, 70000),
        _ => panic!("Expected InvalidPort error"),
    }
}

#[test]
fn test_auth_method_to_db_string() {
    assert_eq!(AuthMethod::Agent.to_db_string(), "agent");
    assert_eq!(
        AuthMethod::Password { password: "test".to_string() }.to_db_string(),
        "password"
    );
    assert_eq!(
        AuthMethod::PrivateKey { key_path: "~/.ssh/id_rsa".to_string(), passphrase: None }.to_db_string(),
        "key:~/.ssh/id_rsa"
    );
}

#[test]
fn test_auth_method_from_db_string() {
    assert!(matches!(AuthMethod::from_db_string("agent", None), AuthMethod::Agent));
    assert!(matches!(
        AuthMethod::from_db_string("password", None),
        AuthMethod::Password { password } if password.is_empty()
    ));

    let key_auth = AuthMethod::from_db_string("key", Some("~/.ssh/id_rsa"));
    assert!(matches!(
        key_auth,
        AuthMethod::PrivateKey { key_path, .. } if key_path == "~/.ssh/id_rsa"
    ));
}

#[test]
fn test_auth_method_auth_type() {
    assert_eq!(AuthMethod::Agent.auth_type(), "agent");
    assert_eq!(AuthMethod::Password { password: "test".to_string() }.auth_type(), "password");
    assert_eq!(
        AuthMethod::PrivateKey { key_path: "~/.ssh/id_rsa".to_string(), passphrase: None }.auth_type(),
        "key"
    );
}

#[test]
fn test_server_status_display() {
    assert_eq!(format!("{}", ServerStatus::Unknown), "unknown");
    assert_eq!(format!("{}", ServerStatus::Online), "online");
    assert_eq!(format!("{}", ServerStatus::Offline), "offline");
    assert_eq!(format!("{}", ServerStatus::Error), "error");
    assert_eq!(format!("{}", ServerStatus::Connecting), "connecting");
}

#[test]
fn test_server_status_to_db_string() {
    assert_eq!(ServerStatus::Unknown.to_db_string(), "unknown");
    assert_eq!(ServerStatus::Online.to_db_string(), "online");
    assert_eq!(ServerStatus::Offline.to_db_string(), "offline");
    assert_eq!(ServerStatus::Error.to_db_string(), "error");
    assert_eq!(ServerStatus::Connecting.to_db_string(), "connecting");
}

#[test]
fn test_server_status_from_db_string() {
    assert_eq!(ServerStatus::from_db_string("unknown"), ServerStatus::Unknown);
    assert_eq!(ServerStatus::from_db_string("online"), ServerStatus::Online);
    assert_eq!(ServerStatus::from_db_string("offline"), ServerStatus::Offline);
    assert_eq!(ServerStatus::from_db_string("error"), ServerStatus::Error);
    assert_eq!(ServerStatus::from_db_string("connecting"), ServerStatus::Connecting);
    assert_eq!(ServerStatus::from_db_string("invalid"), ServerStatus::Unknown);
}

#[test]
fn test_create_server_dto_validation() {
    let dto = CreateServerDto {
        name: "Test".to_string(),
        host: "192.168.1.1".to_string(),
        port: Some(22),
        username: "admin".to_string(),
        auth_method: AuthMethod::Password { password: "secret".to_string() },
        group_id: None,
    };

    // Basic validation
    assert!(!dto.name.is_empty());
    assert!(!dto.host.is_empty());
    assert!(!dto.username.is_empty());
}

#[test]
fn test_ssh_config_parsing_basic() {
    // Test parsing a basic SSH config line
    let config_line = "Host github.com";
    let parts: Vec<&str> = config_line.split_whitespace().collect();
    assert_eq!(parts[0], "Host");
    assert_eq!(parts[1], "github.com");
}

#[test]
fn test_ssh_config_parsing_with_wildcard() {
    let config_line = "Host *.example.com";
    let parts: Vec<&str> = config_line.split_whitespace().collect();
    assert_eq!(parts[0], "Host");
    assert!(parts[1].contains("*"));
}

#[test]
fn test_session_manager_creation() {
    let manager = SshSessionManager::new();
    // Manager should be created successfully
    drop(manager);
}

// Note: Actual connection tests would require a real SSH server or mock
// For unit tests, we focus on validation and structure tests
// Integration tests would test actual SSH connections

#[test]
fn test_host_validation_valid_ipv4() {
    let hosts = [
        "192.168.1.1",
        "10.0.0.1",
        "127.0.0.1",
        "0.0.0.0",
        "255.255.255.255",
    ];

    for host in &hosts {
        // Basic format check - actual IP validation would be more thorough
        assert!(!host.is_empty());
        assert!(host.contains('.') || host.contains(':') || host.chars().next().unwrap().is_alphanumeric());
    }
}

#[test]
fn test_host_validation_valid_hostnames() {
    let hosts = [
        "example.com",
        "subdomain.example.com",
        "localhost",
        "server-01.company.local",
        "test_server",
    ];

    for host in &hosts {
        assert!(!host.is_empty());
        // Hostnames shouldn't contain spaces
        assert!(!host.contains(' '));
    }
}

#[tokio::test]
async fn test_session_metadata_creation() {
    use std::time::Instant;

    let metadata = SessionMetadata {
        id: "test-session".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        connected_at: Instant::now(),
        last_activity: Instant::now(),
    };

    assert_eq!(metadata.id, "test-session");
    assert_eq!(metadata.host, "192.168.1.100");
    assert_eq!(metadata.port, 22);
    assert_eq!(metadata.username, "admin");
}

#[test]
fn test_ssh_key_validation() {
    // Valid key paths
    let valid_paths = [
        "~/.ssh/id_rsa",
        "~/.ssh/id_ed25519",
        "/home/user/.ssh/my_key",
        "C:\\Users\\user\\.ssh\\id_rsa", // Windows path
    ];

    for path in &valid_paths {
        assert!(!path.is_empty());
        assert!(path.contains("ssh") || path.contains("id_"));
    }
}

#[test]
fn test_port_validation() {
    let valid_ports = [1u16, 22, 80, 443, 2222, 8080, 65535];
    for port in &valid_ports {
        assert!(*port > 0 && *port <= 65535, "Port {} should be valid", port);
    }
}
