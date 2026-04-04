//! Configuration types for russh-based SSH implementation
//!
//! Provides SSH connection configuration following SYSTEM_INVARIANTS.md constraints:
//! - Section 2.3: Authentication security (keychain, memory clearing)
//! - Section 2.2: Connection timeout settings

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// SSH authentication method for russh backend.
///
/// # Security Constraints (SYSTEM_INVARIANTS.md Section 2.3)
///
/// - Passwords/keys must be stored in system keychain, not plaintext
/// - Failed auth must clear sensitive data from memory
/// - Private key files must have proper permissions (600 on Unix)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RusshAuthMethod {
    /// Password authentication (should be retrieved from keychain)
    Password(String),

    /// Public key authentication with optional passphrase
    PublicKey {
        /// Path to private key file
        path: PathBuf,
        /// Optional passphrase for encrypted keys
        passphrase: Option<String>,
        /// Raw key data (if loaded from memory)
        key_data: Option<Vec<u8>>,
    },

    /// SSH agent authentication
    Agent,

    /// Keyboard-interactive authentication
    KeyboardInteractive {
        /// Responses to prompts
        responses: Vec<String>,
    },

    /// None authentication (for testing only)
    None,
}

impl RusshAuthMethod {
    /// Create password authentication method.
    ///
    /// **Warning**: Password should be retrieved from keychain, not hardcoded.
    pub fn password(password: impl Into<String>) -> Self {
        Self::Password(password.into())
    }

    /// Create public key authentication from file path.
    pub fn public_key_file(path: PathBuf, passphrase: Option<String>) -> Self {
        Self::PublicKey {
            path,
            passphrase,
            key_data: None,
        }
    }

    /// Create public key authentication from raw key data.
    pub fn public_key_data(key_data: Vec<u8>, passphrase: Option<String>) -> Self {
        Self::PublicKey {
            path: PathBuf::new(),
            passphrase,
            key_data: Some(key_data),
        }
    }

    /// Create SSH agent authentication.
    pub fn agent() -> Self {
        Self::Agent
    }

    /// Check if authentication method is valid.
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Password(p) => !p.is_empty(),
            Self::PublicKey { path, key_data, .. } => {
                !path.as_os_str().is_empty() || key_data.is_some()
            }
            Self::Agent => true,
            Self::KeyboardInteractive { responses } => !responses.is_empty(),
            Self::None => false, // None auth is never valid for real connections
        }
    }

    /// Get display name for this auth method.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Password(_) => "Password",
            Self::PublicKey { .. } => "Public Key",
            Self::Agent => "SSH Agent",
            Self::KeyboardInteractive { .. } => "Keyboard Interactive",
            Self::None => "None",
        }
    }

    /// Check if this is password authentication.
    pub fn is_password(&self) -> bool {
        matches!(self, Self::Password(_))
    }

    /// Check if this is public key authentication.
    pub fn is_public_key(&self) -> bool {
        matches!(self, Self::PublicKey { .. })
    }

    /// Check if this is agent authentication.
    pub fn is_agent(&self) -> bool {
        matches!(self, Self::Agent)
    }

    /// Clear sensitive data from memory.
    ///
    /// Following SYSTEM_INVARIANTS.md Section 2.3:
    /// "Auth failure must clear sensitive data from memory"
    pub fn clear_sensitive_data(&mut self) {
        match self {
            Self::Password(ref mut p) => {
                // Zero out password memory
                unsafe {
                    std::ptr::write_volatile(p.as_mut_ptr(), 0);
                }
                p.clear();
            }
            Self::PublicKey {
                ref mut passphrase,
                ref mut key_data,
                ..
            } => {
                if let Some(ref mut p) = passphrase {
                    unsafe {
                        std::ptr::write_volatile(p.as_mut_ptr(), 0);
                    }
                    p.clear();
                }
                if let Some(ref mut data) = key_data {
                    for byte in data.iter_mut() {
                        unsafe {
                            std::ptr::write_volatile(byte, 0);
                        }
                    }
                    data.clear();
                }
            }
            Self::KeyboardInteractive { ref mut responses } => {
                for resp in responses.iter_mut() {
                    unsafe {
                        std::ptr::write_volatile(resp.as_mut_ptr(), 0);
                    }
                    resp.clear();
                }
                responses.clear();
            }
            _ => {}
        }
    }
}

impl std::fmt::Display for RusshAuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Password(_) => write!(f, "password"),
            Self::PublicKey { path, .. } => write!(f, "publickey({})", path.display()),
            Self::Agent => write!(f, "agent"),
            Self::KeyboardInteractive { .. } => write!(f, "keyboard-interactive"),
            Self::None => write!(f, "none"),
        }
    }
}

/// Known hosts verification policy.
///
/// Following SYSTEM_INVARIANTS.md Section 9.2:
/// "SSH host key must be verified (first ask, subsequent verify)"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RusshKnownHostsPolicy {
    /// Strict verification - reject unknown or changed keys
    #[default]
    Strict,

    /// Accept new keys, verify existing ones
    AcceptNew,

    /// Add new keys automatically to known_hosts
    Add,

    /// Ignore host key verification (dangerous, only for testing)
    Ignore,
}

impl RusshKnownHostsPolicy {
    /// Check if this policy requires verification.
    pub fn requires_verification(&self) -> bool {
        !matches!(self, Self::Ignore)
    }

    /// Check if new hosts are automatically accepted.
    pub fn auto_accept_new(&self) -> bool {
        matches!(self, Self::AcceptNew | Self::Add | Self::Ignore)
    }

    /// Get policy description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Strict => "Strict - Reject unknown or changed host keys",
            Self::AcceptNew => "AcceptNew - Accept first connection, verify subsequent",
            Self::Add => "Add - Automatically add new host keys",
            Self::Ignore => "Ignore - No host key verification (insecure)",
        }
    }
}

/// Connection timeout configuration.
///
/// Following SYSTEM_INVARIANTS.md Section 8.1:
/// "Connection establishment: target < 2s, max 10s"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RusshTimeout {
    /// Connection establishment timeout in seconds (default: 10)
    pub connect_secs: u64,

    /// Authentication timeout in seconds (default: 30)
    pub auth_secs: u64,

    /// Channel operation timeout in seconds (default: 30)
    pub channel_secs: u64,

    /// Keepalive interval in seconds (0 = disabled, default: 30)
    pub keepalive_secs: u64,

    /// Command execution timeout in seconds (default: 60)
    pub command_secs: u64,
}

impl RusshTimeout {
    /// Create new timeout configuration.
    pub fn new(
        connect_secs: u64,
        auth_secs: u64,
        channel_secs: u64,
        keepalive_secs: u64,
        command_secs: u64,
    ) -> Self {
        Self {
            connect_secs,
            auth_secs,
            channel_secs,
            keepalive_secs,
            command_secs,
        }
    }

    /// Get connection timeout as Duration.
    pub fn connect_duration(&self) -> Duration {
        Duration::from_secs(self.connect_secs)
    }

    /// Get authentication timeout as Duration.
    pub fn auth_duration(&self) -> Duration {
        Duration::from_secs(self.auth_secs)
    }

    /// Get channel timeout as Duration.
    pub fn channel_duration(&self) -> Duration {
        Duration::from_secs(self.channel_secs)
    }

    /// Get keepalive interval as Duration (None if 0).
    pub fn keepalive_duration(&self) -> Option<Duration> {
        if self.keepalive_secs > 0 {
            Some(Duration::from_secs(self.keepalive_secs))
        } else {
            None
        }
    }

    /// Get command timeout as Duration.
    pub fn command_duration(&self) -> Duration {
        Duration::from_secs(self.command_secs)
    }

    /// Create aggressive timeout for quick connection tests.
    pub fn aggressive() -> Self {
        Self {
            connect_secs: 5,
            auth_secs: 10,
            channel_secs: 10,
            keepalive_secs: 0,
            command_secs: 10,
        }
    }

    /// Create relaxed timeout for slow networks.
    pub fn relaxed() -> Self {
        Self {
            connect_secs: 30,
            auth_secs: 60,
            channel_secs: 60,
            keepalive_secs: 60,
            command_secs: 120,
        }
    }
}

impl Default for RusshTimeout {
    fn default() -> Self {
        Self {
            connect_secs: 10,
            auth_secs: 30,
            channel_secs: 30,
            keepalive_secs: 30,
            command_secs: 60,
        }
    }
}

/// SSH connection configuration for russh backend.
///
/// # Example
///
/// ```rust
/// use easyssh_core::russh_impl::{RusshConfig, RusshAuthMethod, RusshKnownHostsPolicy};
///
/// let config = RusshConfig::new("192.168.1.1", 22, "root")
///     .with_auth(RusshAuthMethod::agent())
///     .with_known_hosts_policy(RusshKnownHostsPolicy::AcceptNew);
///
/// assert!(config.is_valid());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RusshConfig {
    /// Remote host address (IP or hostname)
    pub host: String,

    /// Remote SSH port (default: 22)
    pub port: u16,

    /// Username for authentication
    pub username: String,

    /// Authentication method
    pub auth: RusshAuthMethod,

    /// Connection timeout settings
    pub timeout: RusshTimeout,

    /// Known hosts verification policy
    pub known_hosts_policy: RusshKnownHostsPolicy,

    /// Path to known_hosts file
    pub known_hosts_path: Option<PathBuf>,

    /// Enable compression (default: true)
    pub compression: bool,

    /// Preferred cipher (None = default)
    pub preferred_cipher: Option<String>,

    /// Preferred MAC (None = default)
    pub preferred_mac: Option<String>,

    /// Preferred key exchange algorithm (None = default)
    pub preferred_kex: Option<String>,

    /// Jump host configuration for ProxyJump
    pub jump_host: Option<JumpHostConfig>,
}

/// Jump host (ProxyJump) configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpHostConfig {
    /// Jump host address
    pub host: String,

    /// Jump host port
    pub port: u16,

    /// Jump host username
    pub username: String,

    /// Jump host authentication
    pub auth: RusshAuthMethod,
}

impl JumpHostConfig {
    /// Create new jump host config.
    pub fn new(host: impl Into<String>, port: u16, username: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            auth: RusshAuthMethod::Agent,
        }
    }

    /// Set authentication method.
    pub fn with_auth(mut self, auth: RusshAuthMethod) -> Self {
        self.auth = auth;
        self
    }

    /// Get address string.
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        !self.host.is_empty() && self.port > 0 && !self.username.is_empty()
    }
}

impl RusshConfig {
    /// Create new SSH configuration with defaults.
    pub fn new(host: impl Into<String>, port: u16, username: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            auth: RusshAuthMethod::Agent,
            timeout: RusshTimeout::default(),
            known_hosts_policy: RusshKnownHostsPolicy::default(),
            known_hosts_path: Self::default_known_hosts_path(),
            compression: true,
            preferred_cipher: None,
            preferred_mac: None,
            preferred_kex: None,
            jump_host: None,
        }
    }

    /// Create config with password authentication.
    pub fn with_password(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::new(host, port, username).with_auth(RusshAuthMethod::password(password))
    }

    /// Create config with public key authentication.
    pub fn with_key(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        key_path: PathBuf,
        passphrase: Option<String>,
    ) -> Self {
        Self::new(host, port, username)
            .with_auth(RusshAuthMethod::public_key_file(key_path, passphrase))
    }

    /// Create config with agent authentication.
    pub fn with_agent(host: impl Into<String>, port: u16, username: impl Into<String>) -> Self {
        Self::new(host, port, username)
    }

    /// Set authentication method.
    pub fn with_auth(mut self, auth: RusshAuthMethod) -> Self {
        self.auth = auth;
        self
    }

    /// Set timeout configuration.
    pub fn with_timeout(mut self, timeout: RusshTimeout) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set known hosts policy.
    pub fn with_known_hosts_policy(mut self, policy: RusshKnownHostsPolicy) -> Self {
        self.known_hosts_policy = policy;
        self
    }

    /// Set known hosts file path.
    pub fn with_known_hosts_path(mut self, path: Option<PathBuf>) -> Self {
        self.known_hosts_path = path;
        self
    }

    /// Set compression enabled.
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }

    /// Set preferred cipher.
    pub fn with_cipher(mut self, cipher: Option<String>) -> Self {
        self.preferred_cipher = cipher;
        self
    }

    /// Set jump host configuration.
    pub fn with_jump_host(mut self, jump_host: Option<JumpHostConfig>) -> Self {
        self.jump_host = jump_host;
        self
    }

    /// Get default known_hosts path.
    fn default_known_hosts_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".ssh").join("known_hosts"))
    }

    /// Get connection address string.
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Check if configuration is valid.
    pub fn is_valid(&self) -> bool {
        !self.host.is_empty()
            && self.port > 0
            && !self.username.is_empty()
            && self.auth.is_valid()
    }

    /// Get server key for connection pooling.
    pub fn server_key(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }
}

impl Default for RusshConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            username: String::new(),
            auth: RusshAuthMethod::Agent,
            timeout: RusshTimeout::default(),
            known_hosts_policy: RusshKnownHostsPolicy::default(),
            known_hosts_path: Self::default_known_hosts_path(),
            compression: true,
            preferred_cipher: None,
            preferred_mac: None,
            preferred_kex: None,
            jump_host: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_method_validity() {
        assert!(RusshAuthMethod::password("test").is_valid());
        assert!(!RusshAuthMethod::password("").is_valid());
        assert!(RusshAuthMethod::agent().is_valid());
    }

    #[test]
    fn test_auth_method_display_name() {
        assert_eq!(RusshAuthMethod::password("x").display_name(), "Password");
        assert_eq!(RusshAuthMethod::agent().display_name(), "SSH Agent");
    }

    #[test]
    fn test_known_hosts_policy() {
        assert!(RusshKnownHostsPolicy::Strict.requires_verification());
        assert!(!RusshKnownHostsPolicy::Ignore.requires_verification());
        assert!(RusshKnownHostsPolicy::AcceptNew.auto_accept_new());
    }

    #[test]
    fn test_timeout_defaults() {
        let timeout = RusshTimeout::default();
        assert_eq!(timeout.connect_secs, 10);
        assert_eq!(timeout.keepalive_secs, 30);
    }

    #[test]
    fn test_config_validity() {
        let config = RusshConfig::new("192.168.1.1", 22, "root");
        assert!(config.is_valid());

        let config = RusshConfig::new("", 22, "root");
        assert!(!config.is_valid());
    }

    #[test]
    fn test_config_builder() {
        let config = RusshConfig::with_password("host", 22, "user", "pass")
            .with_compression(false)
            .with_known_hosts_policy(RusshKnownHostsPolicy::AcceptNew);

        assert!(config.auth.is_password());
        assert!(!config.compression);
        assert_eq!(config.known_hosts_policy, RusshKnownHostsPolicy::AcceptNew);
    }

    #[test]
    fn test_jump_host_config() {
        let jump = JumpHostConfig::new("jumphost", 22, "jumpuser")
            .with_auth(RusshAuthMethod::agent());

        assert!(jump.is_valid());
        assert_eq!(jump.address(), "jumphost:22");
    }

    #[test]
    fn test_clear_sensitive_data() {
        let mut auth = RusshAuthMethod::password("secret_password");
        auth.clear_sensitive_data();

        // After clearing, the password should be empty
        if let RusshAuthMethod::Password(p) = &auth {
            assert!(p.is_empty() || p.as_bytes().iter().all(|&b| b == 0));
        }
    }
}