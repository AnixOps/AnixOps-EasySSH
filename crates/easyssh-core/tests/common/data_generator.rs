//! Test Data Generator
//!
//! Utilities for generating test data for unit and integration tests.
//! Supports generating servers, groups, and related entities.

use easyssh_core::models::{AuthMethod, Group, Server};

/// Generator for creating test data with configurable properties
pub struct TestDataGenerator {
    prefix: String,
    counter: std::sync::atomic::AtomicU64,
}

impl TestDataGenerator {
    /// Create a new generator with the given prefix
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Get next unique ID
    fn next_id(&self) -> String {
        let count = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("{}-{:06}", self.prefix, count)
    }

    /// Generate a test server
    pub fn generate_server(&self) -> Server {
        let id = self.next_id();
        let num = id
            .split('-')
            .last()
            .unwrap_or("1")
            .parse::<u16>()
            .unwrap_or(1);

        Server::new(
            format!("Server {}", id),
            format!("192.168.{}.{}", num / 256, num % 256),
            22,
            "admin".to_string(),
            AuthMethod::Agent,
            None,
        )
    }

    /// Generate a server with specific properties
    pub fn generate_server_with(
        &self,
        name: Option<String>,
        host: Option<String>,
        port: Option<u16>,
        username: Option<String>,
    ) -> Server {
        let id = self.next_id();
        let num = id
            .split('-')
            .last()
            .unwrap_or("1")
            .parse::<u16>()
            .unwrap_or(1);

        let name = name.unwrap_or_else(|| format!("Server {}", id));
        let host = host.unwrap_or_else(|| format!("192.168.{}.{}", num / 256, num % 256));
        let port = port.unwrap_or(22);
        let username = username.unwrap_or_else(|| "admin".to_string());

        Server::new(name, host, port, username, AuthMethod::Agent, None)
    }

    /// Generate multiple servers
    pub fn generate_servers(&self, count: usize) -> Vec<Server> {
        (0..count).map(|_| self.generate_server()).collect()
    }

    /// Generate a test group
    pub fn generate_group(&self) -> Group {
        let id = self.next_id();
        let colors = vec![
            "#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#FF00FF", "#00FFFF",
        ];
        let num = id
            .split('-')
            .last()
            .unwrap_or("1")
            .parse::<usize>()
            .unwrap_or(1);
        let color = colors[num % colors.len()].to_string();

        Group::new(format!("Group {}", id), color)
    }

    /// Generate multiple groups
    pub fn generate_groups(&self, count: usize) -> Vec<Group> {
        (0..count).map(|_| self.generate_group()).collect()
    }
}

impl Default for TestDataGenerator {
    fn default() -> Self {
        Self::new("test")
    }
}

/// Predefined test scenarios
pub mod scenarios {
    use super::*;

    /// Generate a production-like environment
    pub fn production_environment() -> (Vec<Group>, Vec<Server>) {
        let mut groups = vec![];
        let mut servers = vec![];

        // Production group
        let prod_group = Group::new("Production".to_string(), "#FF4136".to_string());
        groups.push(prod_group.clone());

        for i in 1..=5 {
            let server = Server::new(
                format!("Prod-Web-{}", i),
                format!("10.0.1.{}", i),
                22,
                "deploy".to_string(),
                AuthMethod::Agent,
                Some(prod_group.id.clone()),
            );
            servers.push(server);
        }

        for i in 1..=3 {
            let server = Server::new(
                format!("Prod-DB-{}", i),
                format!("10.0.2.{}", i),
                22,
                "dbadmin".to_string(),
                AuthMethod::Agent,
                Some(prod_group.id.clone()),
            );
            servers.push(server);
        }

        // Staging group
        let staging_group = Group::new("Staging".to_string(), "#FFDC00".to_string());
        groups.push(staging_group.clone());

        for i in 1..=3 {
            let server = Server::new(
                format!("Staging-{}", i),
                format!("10.1.1.{}", i),
                22,
                "deploy".to_string(),
                AuthMethod::Agent,
                Some(staging_group.id.clone()),
            );
            servers.push(server);
        }

        // Development group
        let dev_group = Group::new("Development".to_string(), "#2ECC40".to_string());
        groups.push(dev_group.clone());

        for i in 1..=10 {
            let server = Server::new(
                format!("Dev-{}", i),
                format!("192.168.10.{}", i),
                22,
                "developer".to_string(),
                AuthMethod::Agent,
                Some(dev_group.id.clone()),
            );
            servers.push(server);
        }

        (groups, servers)
    }

    /// Generate edge case scenarios
    pub fn edge_cases() -> Vec<Server> {
        vec![
            // Empty name
            Server::new(
                "".to_string(),
                "192.168.1.1".to_string(),
                22,
                "admin".to_string(),
                AuthMethod::Agent,
                None,
            ),
            // Very long name
            Server::new(
                "a".repeat(1000),
                "192.168.1.2".to_string(),
                22,
                "admin".to_string(),
                AuthMethod::Agent,
                None,
            ),
            // Unicode name
            Server::new(
                "服务器测试".to_string(),
                "192.168.1.3".to_string(),
                22,
                "admin".to_string(),
                AuthMethod::Agent,
                None,
            ),
            // Special characters
            Server::new(
                "Test <script>alert('xss')</script>".to_string(),
                "192.168.1.4".to_string(),
                22,
                "admin".to_string(),
                AuthMethod::Agent,
                None,
            ),
            // Edge port numbers
            Server::new(
                "Port 1".to_string(),
                "192.168.1.5".to_string(),
                1,
                "admin".to_string(),
                AuthMethod::Agent,
                None,
            ),
            Server::new(
                "Port Max".to_string(),
                "192.168.1.6".to_string(),
                65535,
                "admin".to_string(),
                AuthMethod::Agent,
                None,
            ),
            // IPv6 address
            Server::new(
                "IPv6".to_string(),
                "::1".to_string(),
                22,
                "admin".to_string(),
                AuthMethod::Agent,
                None,
            ),
            // Hostname
            Server::new(
                "Hostname".to_string(),
                "example.com".to_string(),
                2222,
                "admin".to_string(),
                AuthMethod::Agent,
                None,
            ),
        ]
    }
}

/// Generate realistic SSH configurations for testing
pub mod ssh_configs {
    /// Sample SSH config file content
    pub fn sample_ssh_config() -> &'static str {
        r#"# Personal GitHub Account
Host github-personal
    HostName github.com
    User git
    IdentityFile ~/.ssh/id_rsa_personal
    IdentitiesOnly yes

# Work GitHub Account
Host github-work
    HostName github.com
    User git
    IdentityFile ~/.ssh/id_rsa_work
    IdentitiesOnly yes

# Production servers
Host prod-*
    HostName %h.example.com
    User admin
    Port 22
    StrictHostKeyChecking yes
    UserKnownHostsFile ~/.ssh/known_hosts_prod

Host prod-web-1
    HostName 10.0.1.10
    ProxyJump bastion

Host prod-db-1
    HostName 10.0.2.10
    ProxyJump bastion
    LocalForward 5433 localhost:5432

# Bastion/Jump host
Host bastion
    HostName bastion.example.com
    User jumper
    Port 2222
    IdentityFile ~/.ssh/bastion_key

# Development
Host dev-*
    HostName %h.local
    User developer
    Port 22
    StrictHostKeyChecking no

Host dev-local
    HostName localhost
    User dev
    Port 2222
"#
    }

    /// Malformed SSH config for error testing
    pub fn malformed_ssh_config() -> &'static str {
        r#"Host
    HostName
    Port abc
    User

Host *
    InvalidOption value
    AnotherInvalidThing

Host test
HostName 192.168.1.1
Port 22
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_creates_unique_ids() {
        let gen = TestDataGenerator::new("test");

        let server1 = gen.generate_server();
        let server2 = gen.generate_server();

        assert_ne!(server1.id, server2.id);
    }

    #[test]
    fn test_generator_creates_multiple_servers() {
        let gen = TestDataGenerator::new("test");
        let servers = gen.generate_servers(10);

        assert_eq!(servers.len(), 10);

        // All IDs should be unique
        let ids: std::collections::HashSet<_> = servers.iter().map(|s| &s.id).collect();
        assert_eq!(ids.len(), 10);
    }

    #[test]
    fn test_production_environment() {
        let (groups, servers) = scenarios::production_environment();

        assert!(!groups.is_empty());
        assert!(!servers.is_empty());

        // Verify group assignment
        for server in &servers {
            assert!(server.group_id.is_some());
        }
    }

    #[test]
    fn test_edge_cases_generation() {
        let edge_cases = scenarios::edge_cases();
        assert!(!edge_cases.is_empty());

        // Verify edge cases are preserved
        assert!(edge_cases.iter().any(|s| s.name.is_empty()));
        assert!(edge_cases.iter().any(|s| s.name.len() > 100));
    }
}
