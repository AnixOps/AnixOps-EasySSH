//! Integration Tests for SSH Operations
//!
//! Tests that require SSH infrastructure:
//! - SSH connection establishment
//! - Authentication methods
//! - Command execution
//! - File transfer

use std::time::Duration;

#[path = "../common/mod.rs"]
mod common;

/// Test SSH configuration creation
#[test]
fn test_ssh_config_creation() {
    use easyssh_core::ssh::SshConfig;

    let config = SshConfig::with_password("192.168.1.100", 22, "admin", "secret123");

    assert_eq!(config.host, "192.168.1.100");
    assert_eq!(config.port, 22);
    assert_eq!(config.username, "admin");
    assert!(config.is_password());
}

/// Test SSH key authentication configuration
#[test]
fn test_ssh_key_auth_config() {
    use easyssh_core::ssh::SshConfig;
    use std::path::PathBuf;

    let config = SshConfig::with_key(
        "192.168.1.100",
        22,
        "admin",
        PathBuf::from("~/.ssh/id_rsa"),
        Some("passphrase".to_string()),
    );

    assert!(config.is_public_key());
}

/// Test SSH agent authentication
#[test]
fn test_ssh_agent_auth_config() {
    use easyssh_core::ssh::SshConfig;

    let config = SshConfig::with_agent("192.168.1.100", 22, "admin");

    assert!(config.is_agent());
}

/// Test connection timeout configuration
#[test]
fn test_connection_timeout_config() {
    use easyssh_core::ssh::{ConnectionTimeout, SshConfig};

    let timeout = ConnectionTimeout::new(30, 10, 60, 300);
    let config = SshConfig::new("192.168.1.100", 22, "admin").with_timeout(timeout);

    assert_eq!(config.timeout.connect_secs, 30);
    assert_eq!(config.timeout.auth_secs, 10);
    assert_eq!(config.timeout.keepalive_secs, 60);
}

/// Test SSH connection health check
#[test]
fn test_connection_health_enum() {
    use easyssh_core::ssh::ConnectionHealth;

    // Test enum values
    assert_eq!(ConnectionHealth::Healthy, ConnectionHealth::Healthy);
    assert_eq!(ConnectionHealth::Degraded, ConnectionHealth::Degraded);
    assert_eq!(ConnectionHealth::Unhealthy, ConnectionHealth::Unhealthy);

    assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Unhealthy);
}

/// Test session manager creation
#[test]
fn test_session_manager_creation() {
    use easyssh_core::ssh::SshSessionManager;

    let manager = SshSessionManager::new();

    // Initially no sessions
    let stats = manager.get_pool_stats();
    assert_eq!(stats.total_sessions, 0);
}

/// Test session manager with pool config
#[test]
fn test_session_manager_with_pool_config() {
    use easyssh_core::ssh::SshSessionManager;

    let manager = SshSessionManager::new().with_pool_config(10, 300, 3600);

    let stats = manager.get_pool_stats();
    assert_eq!(stats.total_sessions, 0);
}

/// Test jump host configuration
#[test]
fn test_jump_host_configuration() {
    use easyssh_core::ssh::JumpHost;

    let jump_host = JumpHost::with_password("bastion.example.com", 22, "jumper", "secret");

    assert_eq!(jump_host.host, "bastion.example.com");
    assert_eq!(jump_host.port, 22);
    assert_eq!(jump_host.username, "jumper");
    assert!(jump_host.is_password());
}

/// Test jump host with key
#[test]
fn test_jump_host_with_key() {
    use easyssh_core::ssh::JumpHost;
    use std::path::PathBuf;

    let jump_host = JumpHost::with_key(
        "bastion.example.com",
        22,
        "jumper",
        PathBuf::from("~/.ssh/bastion_key"),
        None,
    );

    assert!(jump_host.is_public_key());
}

/// Test pool stats
#[test]
fn test_pool_stats() {
    use easyssh_core::ssh::PoolStats;

    let stats = PoolStats {
        total_pools: 2,
        total_sessions: 8,
        pools: vec![],
    };

    assert_eq!(stats.total_pools, 2);
    assert_eq!(stats.total_sessions, 8);
}

/// Test session metadata
#[test]
fn test_session_metadata() {
    use easyssh_core::ssh::SessionMetadata;
    use std::time::Instant;

    let metadata = SessionMetadata {
        id: "session-123".to_string(),
        server_id: "srv-001".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        connected_at: Instant::now(),
    };

    assert_eq!(metadata.server_id, "srv-001");
    assert_eq!(metadata.host, "192.168.1.100");
    assert_eq!(metadata.port, 22);
}

/// Test AuthMethod enum
#[test]
fn test_auth_method_enum() {
    use easyssh_core::ssh::AuthMethod;
    use std::path::PathBuf;

    // Password auth
    let password = AuthMethod::Password("secret".to_string());
    assert!(password.is_password());
    assert!(!password.is_public_key());
    assert!(!password.is_agent());

    // Key auth
    let key = AuthMethod::PublicKey {
        path: PathBuf::from("~/.ssh/id_rsa"),
        passphrase: None,
    };
    assert!(key.is_public_key());
    assert!(!key.is_password());
    assert!(!key.is_agent());

    // Agent auth
    let agent = AuthMethod::Agent;
    assert!(agent.is_agent());
    assert!(!agent.is_password());
    assert!(!agent.is_public_key());
}

/// Test ConnectionTimeout
#[test]
fn test_connection_timeout() {
    use easyssh_core::ssh::ConnectionTimeout;

    let timeout = ConnectionTimeout::new(30, 10, 60, 300);

    assert_eq!(timeout.connect_duration(), Duration::from_secs(30));
    assert_eq!(timeout.auth_duration(), Duration::from_secs(10));
    assert_eq!(timeout.keepalive_duration(), Duration::from_secs(60));
    assert_eq!(timeout.command_duration(), Some(Duration::from_secs(300)));
}

/// Test strip_ansi_codes function
#[test]
fn test_strip_ansi_codes() {
    use easyssh_core::ssh::strip_ansi_codes;

    let input = "\x1b[32mGreen text\x1b[0m";
    let stripped = strip_ansi_codes(input);
    assert_eq!(stripped, "Green text");

    let complex = "\x1b[1;31;42mBold Red on Green\x1b[0m Normal";
    let stripped = strip_ansi_codes(complex);
    assert_eq!(stripped, "Bold Red on Green Normal");
}

/// Test config validation
#[test]
fn test_ssh_config_validation() {
    use easyssh_core::ssh::SshConfig;

    // Valid config
    let valid = SshConfig::new("192.168.1.100", 22, "admin");
    assert!(valid.is_valid());

    // Empty host should still be valid struct (validation happens elsewhere)
    let empty_host = SshConfig::new("", 22, "admin");
    assert!(!empty_host.is_valid());
}
