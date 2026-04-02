//! Integration Tests for SSH Operations
//!
//! Tests that require SSH infrastructure:
//! - SSH connection establishment
//! - Authentication methods
//! - Command execution
//! - File transfer

use std::time::Duration;

mod common;

/// Test SSH configuration parsing and validation
#[test]
fn test_ssh_config_parsing() {
    use easyssh_core::config::SshConfig;
    use std::str::FromStr;

    let config_str = r#"
Host github.com
    HostName github.com
    User git
    Port 22
    IdentityFile ~/.ssh/id_rsa

Host production
    HostName 192.168.1.100
    User admin
    Port 2222
    StrictHostKeyChecking no
"#;

    let config = SshConfig::from_str(config_str).expect("Should parse config");

    // Verify parsed entries
    let hosts = config.get_hosts();
    assert!(hosts.contains("github.com"));
    assert!(hosts.contains("production"));

    // Verify specific values
    let github = config.get_host("github.com").expect("Should have github.com");
    assert_eq!(github.hostname, Some("github.com".to_string()));
    assert_eq!(github.user, Some("git".to_string()));
    assert_eq!(github.port, Some(22));
}

/// Test SSH connection configuration
#[test]
fn test_ssh_connection_config() {
    use easyssh_core::ssh::SshConnectionConfig;

    let config = SshConnectionConfig::builder()
        .host("192.168.1.100")
        .port(22)
        .username("admin")
        .password_auth("secret123")
        .build()
        .expect("Should build config");

    assert_eq!(config.host, "192.168.1.100");
    assert_eq!(config.port, 22);
    assert_eq!(config.username, "admin");
    assert!(config.auth_method.is_password());
}

/// Test SSH key authentication configuration
#[test]
fn test_ssh_key_auth_config() {
    use easyssh_core::ssh::SshConnectionConfig;

    let config = SshConnectionConfig::builder()
        .host("192.168.1.100")
        .username("admin")
        .key_auth("~/.ssh/id_rsa", Some("passphrase"))
        .build()
        .expect("Should build config");

    assert!(config.auth_method.is_key());

    match &config.auth_method {
        easyssh_core::ssh::AuthMethod::Key { identity_file, passphrase } => {
            assert_eq!(identity_file, &std::path::PathBuf::from("~/.ssh/id_rsa"));
            assert_eq!(passphrase.as_deref(), Some("passphrase"));
        }
        _ => panic!("Expected key auth"),
    }
}

/// Test SSH agent authentication
#[test]
fn test_ssh_agent_auth_config() {
    use easyssh_core::ssh::SshConnectionConfig;

    let config = SshConnectionConfig::builder()
        .host("192.168.1.100")
        .username("admin")
        .agent_auth()
        .build()
        .expect("Should build config");

    assert!(config.auth_method.is_agent());
}

/// Test connection timeout configuration
#[test]
fn test_connection_timeout_config() {
    use easyssh_core::ssh::SshConnectionConfig;

    let config = SshConnectionConfig::builder()
        .host("192.168.1.100")
        .username("admin")
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Should build config");

    assert_eq!(config.timeout, Duration::from_secs(30));
}

/// Test SSH connection health check
#[test]
fn test_connection_health_check() {
    use easyssh_core::ssh::{SshConnection, SshConnectionConfig};

    // This test validates the health check logic without actual connection
    let config = SshConnectionConfig::builder()
        .host("192.168.1.100")
        .username("admin")
        .build()
        .expect("Should build config");

    // A new connection should not be established
    let conn = SshConnection::new(config);
    assert!(!conn.is_connected());
    assert!(!conn.is_established());
}

/// Test known hosts handling
#[test]
fn test_known_hosts_handling() {
    use easyssh_core::ssh::KnownHosts;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let known_hosts_path = temp_dir.path().join("known_hosts");

    let mut known_hosts = KnownHosts::new(&known_hosts_path).expect("Should create KnownHosts");

    // Add a host key
    known_hosts.add_host(
        "github.com",
        22,
        "ssh-rsa",
        b"AAAAfakekeydata",
    ).expect("Should add host");

    // Check if host is known
    assert!(known_hosts.is_host_known("github.com", 22));
    assert!(!known_hosts.is_host_known("unknown.com", 22));

    // Verify host key
    let is_valid = known_hosts.verify_host_key(
        "github.com",
        22,
        "ssh-rsa",
        b"AAAAfakekeydata",
    ).expect("Should verify");
    assert!(is_valid);

    // Wrong key should fail
    let is_valid = known_hosts.verify_host_key(
        "github.com",
        22,
        "ssh-rsa",
        b"DifferentKey",
    ).expect("Should check");
    assert!(!is_valid);
}

/// Test port forwarding configuration
#[test]
#[cfg(feature = "port-forwarding")]
fn test_port_forwarding_config() {
    use easyssh_core::ssh::{PortForwardingConfig, ForwardingType};

    // Local forward
    let local_forward = PortForwardingConfig::new(
        ForwardingType::Local,
        "127.0.0.1",
        8080,
        "internal.server",
        80,
    );

    assert_eq!(local_forward.forwarding_type, ForwardingType::Local);
    assert_eq!(local_forward.local_host, "127.0.0.1");
    assert_eq!(local_forward.local_port, 8080);
    assert_eq!(local_forward.remote_host, "internal.server");
    assert_eq!(local_forward.remote_port, 80);

    // Remote forward
    let remote_forward = PortForwardingConfig::new(
        ForwardingType::Remote,
        "0.0.0.0",
        9090,
        "localhost",
        3000,
    );

    assert_eq!(remote_forward.forwarding_type, ForwardingType::Remote);
}

/// Test connection pooling
#[test]
fn test_connection_pool_management() {
    use easyssh_core::connection_pool::{ConnectionPool, PoolConfig};

    let config = PoolConfig {
        max_connections: 10,
        idle_timeout: Duration::from_secs(300),
        connection_timeout: Duration::from_secs(30),
    };

    let pool = ConnectionPool::new(config);

    // Initially empty
    assert_eq!(pool.active_connections(), 0);
    assert_eq!(pool.idle_connections(), 0);
    assert_eq!(pool.total_connections(), 0);
}

/// Test SFTP configuration (Standard feature)
#[test]
#[cfg(feature = "sftp")]
fn test_sftp_config() {
    use easyssh_core::sftp::SftpConfig;

    let config = SftpConfig::builder()
        .host("192.168.1.100")
        .username("admin")
        .password("secret")
        .initial_dir("/home/admin")
        .build()
        .expect("Should build SFTP config");

    assert_eq!(config.initial_dir, Some("/home/admin".to_string()));
}

/// Test SSH config import from file
#[test]
fn test_ssh_config_import() {
    use easyssh_core::config_import_export::import_ssh_config;
    use tempfile::TempDir;
    use std::io::Write;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("config");

    // Write test SSH config
    let config_content = r#"
Host server1
    HostName 192.168.1.10
    User admin
    Port 22

Host server2
    HostName 192.168.1.11
    User root
    Port 2222
    IdentityFile ~/.ssh/id_rsa
"#;

    let mut file = std::fs::File::create(&config_path).expect("Should create file");
    file.write_all(config_content.as_bytes()).expect("Should write");

    // Import
    let servers = import_ssh_config(&config_path).expect("Should import");
    assert_eq!(servers.len(), 2);

    let server1 = &servers[0];
    assert_eq!(server1.name, "server1");
    assert_eq!(server1.host, "192.168.1.10");
    assert_eq!(server1.username, "admin");
    assert_eq!(server1.port, 22);

    let server2 = &servers[1];
    assert_eq!(server2.name, "server2");
    assert_eq!(server2.port, 2222);
}

/// Test connection retry logic
#[test]
fn test_connection_retry_logic() {
    use easyssh_core::ssh::{RetryConfig, RetryPolicy};

    let config = RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
        policy: RetryPolicy::ExponentialBackoff,
    };

    assert_eq!(config.max_attempts, 3);

    // Calculate delays
    let delays = config.calculate_delays();
    assert_eq!(delays.len(), 2); // delays between 3 attempts

    // Exponential backoff
    assert_eq!(delays[0], Duration::from_secs(1));
    assert_eq!(delays[1], Duration::from_secs(2));
}

/// Test jump host / proxy configuration
#[test]
fn test_jump_host_configuration() {
    use easyssh_core::ssh::{ProxyConfig, ProxyType};

    let jump_host = ProxyConfig::jump_host(
        "bastion.example.com",
        22,
        "jumper",
    );

    assert_eq!(jump_host.proxy_type, ProxyType::JumpHost);
    assert_eq!(jump_host.host, "bastion.example.com");
    assert_eq!(jump_host.port, 22);
    assert_eq!(jump_host.username, "jumper");

    // SOCKS5 proxy
    let socks5 = ProxyConfig::socks5("127.0.0.1", 1080);
    assert_eq!(socks5.proxy_type, ProxyType::Socks5);
}
