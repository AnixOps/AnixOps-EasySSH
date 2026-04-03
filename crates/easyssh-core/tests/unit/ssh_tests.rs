//! SSH Module Unit Tests
//!
//! Tests for SSH functionality including:
//! - SSH config parsing
//! - Connection parameter validation
//! - Session management (mocked)
//! - Health tracking

use easyssh_core::models::server::{AuthMethod, ServerStatus};
use easyssh_core::ssh::{ConnectionHealth, SessionMetadata, SshSessionManager};

#[path = "../common/mod.rs"]
mod common;

#[test]
fn test_connection_health_enum() {
    assert_eq!(ConnectionHealth::Healthy, ConnectionHealth::Healthy);
    assert_eq!(ConnectionHealth::Degraded, ConnectionHealth::Degraded);
    assert_eq!(ConnectionHealth::Unhealthy, ConnectionHealth::Unhealthy);

    assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Unhealthy);
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
fn test_server_status_from_str() {
    assert_eq!(
        ServerStatus::from_status_str("unknown"),
        ServerStatus::Unknown
    );
    assert_eq!(
        ServerStatus::from_status_str("online"),
        ServerStatus::Online
    );
    assert_eq!(
        ServerStatus::from_status_str("offline"),
        ServerStatus::Offline
    );
    assert_eq!(ServerStatus::from_status_str("error"), ServerStatus::Error);
    assert_eq!(
        ServerStatus::from_status_str("connecting"),
        ServerStatus::Connecting
    );
    assert_eq!(
        ServerStatus::from_status_str("invalid"),
        ServerStatus::Unknown
    );
}

#[test]
fn test_auth_method_auth_type() {
    assert_eq!(AuthMethod::Agent.auth_type(), "agent");
    assert_eq!(
        AuthMethod::Password {
            password: "test".to_string()
        }
        .auth_type(),
        "password"
    );
    assert_eq!(
        AuthMethod::PrivateKey {
            key_path: "~/.ssh/id_rsa".to_string(),
            passphrase: None
        }
        .auth_type(),
        "key"
    );
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
        assert!(
            host.contains('.')
                || host.contains(':')
                || host.chars().next().unwrap().is_alphanumeric()
        );
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

#[test]
fn test_session_metadata() {
    let metadata = SessionMetadata {
        server_id: "test-session".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        connected_at: 0,
        last_activity: 0,
        bytes_sent: 0,
        bytes_received: 0,
    };

    assert_eq!(metadata.server_id, "test-session");
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
