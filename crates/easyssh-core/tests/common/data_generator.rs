//! Test Data Generator
//!
//! Utilities for generating test data for unit and integration tests.
//! Supports generating servers, groups, identities, and related entities.

use easyssh_core::models::{Server, Group, Identity, Tag};
use std::collections::HashMap;

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
        let count = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("{}-{:06}", self.prefix, count)
    }

    /// Generate a test server
    pub fn generate_server(&self) -> Server {
        let id = self.next_id();
        let num = id.split('-').last().unwrap_or("1").parse::<u16>().unwrap_or(1);

        Server::new(
            &format!("Server {}", id),
            &format!("192.168.{}.{}", num / 256, num % 256),
            22
        )
        .with_username("admin")
        .with_description(&format!("Test server generated for {}", id))
    }

    /// Generate a server with specific properties
    pub fn generate_server_with(
        &self,
        name: Option<&str>,
        host: Option<&str>,
        port: Option<u16>,
    ) -> Server {
        let id = self.next_id();
        let num = id.split('-').last().unwrap_or("1").parse::<u16>().unwrap_or(1);

        let name = name.unwrap_or(&format!("Server {}", id));
        let host = host.unwrap_or(&format!("192.168.{}.{}", num / 256, num % 256));
        let port = port.unwrap_or(22);

        Server::new(name, host, port)
            .with_username("admin")
    }

    /// Generate multiple servers
    pub fn generate_servers(&self, count: usize) -> Vec<Server> {
        (0..count).map(|_| self.generate_server()).collect()
    }

    /// Generate a test group
    pub fn generate_group(&self) -> Group {
        let id = self.next_id();
        let colors = vec!["#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#FF00FF", "#00FFFF"];
        let num = id.split('-').last().unwrap_or("1").parse::<usize>().unwrap_or(1);
        let color = colors[num % colors.len()];

        Group::new(&format!("Group {}", id), color)
    }

    /// Generate multiple groups
    pub fn generate_groups(&self, count: usize) -> Vec<Group> {
        (0..count).map(|_| self.generate_group()).collect()
    }

    /// Generate a test identity with password
    pub fn generate_password_identity(&self) -> Identity {
        let id = self.next_id();
        Identity::new_password(&format!("Identity {}", id), &format!("password_{}", id))
    }

    /// Generate a test identity with SSH key
    pub fn generate_key_identity(&self) -> Identity {
        let id = self.next_id();
        Identity::new_key(
            &format!("Key Identity {}", id),
            &format!("~/.ssh/id_{}", id.to_lowercase()),
            None
        )
    }

    /// Generate multiple identities
    pub fn generate_identities(&self, count: usize) -> Vec<Identity> {
        (0..count)
            .map(|i| {
                if i % 2 == 0 {
                    self.generate_password_identity()
                } else {
                    self.generate_key_identity()
                }
            })
            .collect()
    }

    /// Generate a tag
    pub fn generate_tag(&self) -> Tag {
        let id = self.next_id();
        Tag::new(&format!("tag-{}", id.to_lowercase()))
    }

    /// Generate multiple tags
    pub fn generate_tags(&self, count: usize) -> Vec<Tag> {
        (0..count).map(|_| self.generate_tag()).collect()
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
    use easyssh_core::models::{Server, Group};

    /// Generate a production-like environment
    pub fn production_environment() -> (Vec<Group>, Vec<Server>) {
        let mut groups = vec![];
        let mut servers = vec![];

        // Production group
        let prod_group = Group::new("Production", "#FF4136");
        groups.push(prod_group.clone());

        for i in 1..=5 {
            let server = Server::new(
                &format!("Prod-Web-{}", i),
                &format!("10.0.1.{}", i),
                22
            )
            .with_username("deploy")
            .with_group(&prod_group.id)
            .with_tags(vec!["production".to_string(), "web".to_string()]);
            servers.push(server);
        }

        for i in 1..=3 {
            let server = Server::new(
                &format!("Prod-DB-{}", i),
                &format!("10.0.2.{}", i),
                22
            )
            .with_username("dbadmin")
            .with_group(&prod_group.id)
            .with_tags(vec!["production".to_string(), "database".to_string()]);
            servers.push(server);
        }

        // Staging group
        let staging_group = Group::new("Staging", "#FFDC00");
        groups.push(staging_group.clone());

        for i in 1..=3 {
            let server = Server::new(
                &format!("Staging-{}", i),
                &format!("10.1.1.{}", i),
                22
            )
            .with_username("deploy")
            .with_group(&staging_group.id)
            .with_tags(vec!["staging".to_string()]);
            servers.push(server);
        }

        // Development group
        let dev_group = Group::new("Development", "#2ECC40");
        groups.push(dev_group.clone());

        for i in 1..=10 {
            let server = Server::new(
                &format!("Dev-{}", i),
                &format!("192.168.10.{}", i),
                22
            )
            .with_username("developer")
            .with_group(&dev_group.id)
            .with_tags(vec!["development".to_string()]);
            servers.push(server);
        }

        (groups, servers)
    }

    /// Generate a team collaboration setup
    pub fn team_collaboration_setup() -> Vec<(String, Vec<Server>)> {
        let mut teams = vec![];

        // Backend team
        let backend_servers: Vec<_> = (1..=8)
            .map(|i| Server::new(
                &format!("Backend-{}", i),
                &format!("10.20.1.{}", i),
                22
            )
            .with_username("backend")
            .with_tags(vec!["backend".to_string(), "api".to_string()]))
            .collect();
        teams.push(("Backend".to_string(), backend_servers));

        // Frontend team
        let frontend_servers: Vec<_> = (1..=6)
            .map(|i| Server::new(
                &format!("Frontend-{}", i),
                &format!("10.20.2.{}", i),
                22
            )
            .with_username("frontend")
            .with_tags(vec!["frontend".to_string(), "web".to_string()]))
            .collect();
        teams.push(("Frontend".to_string(), frontend_servers));

        // DevOps team
        let devops_servers: Vec<_> = (1..=4)
            .map(|i| Server::new(
                &format!("Infra-{}", i),
                &format!("10.20.3.{}", i),
                22
            )
            .with_username("devops")
            .with_tags(vec!["infrastructure".to_string(), "ci-cd".to_string()]))
            .collect();
        teams.push(("DevOps".to_string(), devops_servers));

        teams
    }

    /// Generate edge case scenarios
    pub fn edge_cases() -> Vec<Server> {
        vec![
            // Empty name
            Server::new("", "192.168.1.1", 22),
            // Very long name
            Server::new(&"a".repeat(1000), "192.168.1.2", 22),
            // Unicode name
            Server::new("服务器测试", "192.168.1.3", 22),
            // Special characters
            Server::new("Test <script>alert('xss')</script>", "192.168.1.4", 22),
            // Edge port numbers
            Server::new("Port 1", "192.168.1.5", 1),
            Server::new("Port Max", "192.168.1.6", 65535),
            // IPv6 address (if supported)
            Server::new("IPv6", "::1", 22),
            // Hostname with port
            Server::new("Hostname", "example.com:2222", 22),
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
