//! Configuration Encryption
//!
//! Provides encryption and decryption for sensitive configuration data
//! using AES-256-GCM with Argon2id key derivation.
//!
//! # Features
//! - Master password-based encryption
//! - Transparent field-level encryption
//! - Secure key derivation with Argon2id
//! - Encrypted configuration backups
//!
//! # Example
//! ```rust,no_run
//! use easyssh_core::config::encryption::{ConfigEncryption, EncryptionOptions};
//! use easyssh_core::config::FullConfig;
//!
//! let encryption = ConfigEncryption::new("master_password").unwrap();
//! let config = FullConfig::default();
//! let encrypted = encryption.encrypt_config(&config).unwrap();
//! let decrypted = encryption.decrypt_config(&encrypted).unwrap();
//! ```

use crate::config::types::{FullConfig, UserPreferences};
use crate::crypto::CryptoState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Encryption error types
#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionError {
    /// Serialization failed
    Serialization(String),
    /// Deserialization failed
    Deserialization(String),
    /// Encryption operation failed
    Encryption(String),
    /// Decryption operation failed
    Decryption(String),
    /// Invalid password
    InvalidPassword,
    /// Key derivation failed
    KeyDerivation(String),
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionError::Serialization(s) => write!(f, "Serialization error: {}", s),
            EncryptionError::Deserialization(s) => write!(f, "Deserialization error: {}", s),
            EncryptionError::Encryption(s) => write!(f, "Encryption error: {}", s),
            EncryptionError::Decryption(s) => write!(f, "Decryption error: {}", s),
            EncryptionError::InvalidPassword => write!(f, "Invalid password"),
            EncryptionError::KeyDerivation(s) => write!(f, "Key derivation error: {}", s),
        }
    }
}

impl std::error::Error for EncryptionError {}

impl From<crate::error::LiteError> for EncryptionError {
    fn from(e: crate::error::LiteError) -> Self {
        EncryptionError::Encryption(e.to_string())
    }
}

/// Configuration encryption manager
pub struct ConfigEncryption {
    crypto: CryptoState,
    options: EncryptionOptions,
}

/// Encryption options and settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionOptions {
    /// Encrypt all configuration (not just sensitive fields)
    pub full_encryption: bool,
    /// Encrypt specific sections
    pub encrypt_app_config: bool,
    pub encrypt_user_preferences: bool,
    pub encrypt_security_settings: bool,
    /// Custom fields to encrypt (field paths)
    pub encrypted_fields: Vec<String>,
    /// Key derivation iterations (higher = more secure but slower)
    pub argon2_iterations: u32,
    /// Memory cost for Argon2id
    pub argon2_memory_kb: u32,
    /// Parallelism for Argon2id
    pub argon2_parallelism: u32,
}

impl Default for EncryptionOptions {
    fn default() -> Self {
        Self {
            full_encryption: false,
            encrypt_app_config: false,
            encrypt_user_preferences: true,
            encrypt_security_settings: true,
            encrypted_fields: vec![
                "user_preferences.default_key_path".to_string(),
                "security_settings.custom".to_string(),
            ],
            argon2_iterations: 3,
            argon2_memory_kb: 65536, // 64 MB
            argon2_parallelism: 4,
        }
    }
}

impl EncryptionOptions {
    /// Create options for full configuration encryption
    pub fn full_encryption() -> Self {
        Self {
            full_encryption: true,
            encrypt_app_config: true,
            encrypt_user_preferences: true,
            encrypt_security_settings: true,
            encrypted_fields: vec![],
            argon2_iterations: 3,
            argon2_memory_kb: 65536,
            argon2_parallelism: 4,
        }
    }

    /// Create options for sensitive fields only
    pub fn sensitive_only() -> Self {
        Self::default()
    }

    /// Create options with custom security level
    pub fn with_security_level(level: SecurityLevel) -> Self {
        let (iterations, memory, parallelism) = match level {
            SecurityLevel::Standard => (3, 65536, 4),
            SecurityLevel::High => (4, 262144, 4), // 256 MB
            SecurityLevel::Maximum => (5, 1048576, 8), // 1 GB, 8 threads
        };

        Self {
            full_encryption: false,
            encrypt_app_config: false,
            encrypt_user_preferences: true,
            encrypt_security_settings: true,
            encrypted_fields: vec!["user_preferences.default_key_path".to_string()],
            argon2_iterations: iterations,
            argon2_memory_kb: memory,
            argon2_parallelism: parallelism,
        }
    }
}

/// Security level for encryption
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Standard security (fast, suitable for most users)
    Standard,
    /// High security (slower, suitable for sensitive environments)
    High,
    /// Maximum security (very slow, suitable for high-security environments)
    Maximum,
}

/// Encrypted configuration container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedConfig {
    /// Encryption format version
    pub version: String,
    /// Salt for key derivation (hex encoded)
    pub salt: String,
    /// Encryption parameters
    pub params: EncryptionParams,
    /// Encrypted data sections
    pub sections: HashMap<String, EncryptedSection>,
    /// Unencrypted metadata (if partial encryption)
    pub metadata: Option<ConfigMetadata>,
}

/// Encryption parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionParams {
    pub argon2_iterations: u32,
    pub argon2_memory_kb: u32,
    pub argon2_parallelism: u32,
    pub algorithm: String,
}

impl Default for EncryptionParams {
    fn default() -> Self {
        Self {
            argon2_iterations: 3,
            argon2_memory_kb: 65536,
            argon2_parallelism: 4,
            algorithm: "AES-256-GCM".to_string(),
        }
    }
}

/// Encrypted configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSection {
    /// Encrypted data (base64 encoded)
    pub data: String,
    /// Nonce/IV for AES-GCM (hex encoded)
    pub nonce: String,
}

/// Configuration metadata (for partial encryption)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub version: u32,
    pub theme: String,
    pub language: String,
    pub encrypted_sections: Vec<String>,
}

/// Encryption result
#[derive(Debug, Clone)]
pub struct EncryptionResult {
    pub success: bool,
    pub encrypted_sections: Vec<String>,
    pub errors: Vec<String>,
}

/// Decryption result
#[derive(Debug, Clone)]
pub struct DecryptionResult {
    pub success: bool,
    pub config: Option<FullConfig>,
    pub errors: Vec<String>,
}

/// Field-level encryption for sensitive data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedField {
    pub path: String,
    pub data: String,
    pub nonce: String,
}

/// Password strength assessment
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl PasswordStrength {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            PasswordStrength::VeryWeak => "Very Weak",
            PasswordStrength::Weak => "Weak",
            PasswordStrength::Medium => "Medium",
            PasswordStrength::Strong => "Strong",
            PasswordStrength::VeryStrong => "Very Strong",
        }
    }

    /// Check if password is acceptable
    pub fn is_acceptable(&self) -> bool {
        matches!(
            self,
            PasswordStrength::Medium | PasswordStrength::Strong | PasswordStrength::VeryStrong
        )
    }

    /// Check if password is strong
    pub fn is_strong(&self) -> bool {
        matches!(
            self,
            PasswordStrength::Strong | PasswordStrength::VeryStrong
        )
    }
}

/// Password requirements
#[derive(Debug, Clone)]
pub struct PasswordRequirements {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digits: bool,
    pub require_special: bool,
}

impl Default for PasswordRequirements {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_digits: true,
            require_special: false,
        }
    }
}

impl ConfigEncryption {
    /// Create new encryption instance with master password
    pub fn new(master_password: &str) -> Result<Self, EncryptionError> {
        let mut crypto = CryptoState::new();
        crypto.initialize(master_password)?;

        Ok(Self {
            crypto,
            options: EncryptionOptions::default(),
        })
    }

    /// Create with custom options
    pub fn with_options(
        master_password: &str,
        options: EncryptionOptions,
    ) -> Result<Self, EncryptionError> {
        let mut crypto = CryptoState::new();
        crypto.initialize(master_password)?;

        Ok(Self { crypto, options })
    }

    /// Create from existing crypto state
    pub fn from_crypto(crypto: CryptoState) -> Self {
        Self {
            crypto,
            options: EncryptionOptions::default(),
        }
    }

    /// Encrypt full configuration
    pub fn encrypt_config(&self, config: &FullConfig) -> Result<EncryptedConfig, EncryptionError> {
        let mut sections = HashMap::new();
        let mut metadata = None;

        if self.options.full_encryption {
            // Encrypt entire config as single section
            let config_json = serde_json::to_string(config)
                .map_err(|e| EncryptionError::Serialization(e.to_string()))?;
            let encrypted = self.encrypt_section(&config_json)?;
            sections.insert("full".to_string(), encrypted);
        } else {
            // Encrypt specific sections
            if self.options.encrypt_app_config {
                let app_config_json = serde_json::to_string(&config.app_config)
                    .map_err(|e| EncryptionError::Serialization(e.to_string()))?;
                let encrypted = self.encrypt_section(&app_config_json)?;
                sections.insert("app_config".to_string(), encrypted);
            }

            if self.options.encrypt_user_preferences {
                let mut prefs = config.user_preferences.clone();
                // Encrypt sensitive fields within preferences
                self.encrypt_sensitive_fields(&mut prefs)?;

                let prefs_json = serde_json::to_string(&prefs)
                    .map_err(|e| EncryptionError::Serialization(e.to_string()))?;
                let encrypted = self.encrypt_section(&prefs_json)?;
                sections.insert("user_preferences".to_string(), encrypted);
            }

            if self.options.encrypt_security_settings {
                let sec_json = serde_json::to_string(&config.security_settings)
                    .map_err(|e| EncryptionError::Serialization(e.to_string()))?;
                let encrypted = self.encrypt_section(&sec_json)?;
                sections.insert("security_settings".to_string(), encrypted);
            }

            // Create metadata for partial encryption
            metadata = Some(ConfigMetadata {
                version: config.version,
                theme: format!("{:?}", config.app_config.theme),
                language: format!("{:?}", config.app_config.language),
                encrypted_sections: sections.keys().cloned().collect(),
            });
        }

        let salt = self
            .crypto
            .get_salt()
            .map(|s| hex_encode(&s))
            .unwrap_or_default();

        Ok(EncryptedConfig {
            version: "1.0".to_string(),
            salt,
            params: EncryptionParams {
                argon2_iterations: self.options.argon2_iterations,
                argon2_memory_kb: self.options.argon2_memory_kb,
                argon2_parallelism: self.options.argon2_parallelism,
                algorithm: "AES-256-GCM".to_string(),
            },
            sections,
            metadata,
        })
    }

    /// Decrypt configuration
    pub fn decrypt_config(
        &self,
        encrypted: &EncryptedConfig,
    ) -> Result<FullConfig, EncryptionError> {
        if encrypted.sections.contains_key("full") {
            // Full encryption mode
            let section = encrypted
                .sections
                .get("full")
                .ok_or(EncryptionError::Decryption(
                    "Missing full section".to_string(),
                ))?;
            let decrypted = self.decrypt_section(section)?;
            let config: FullConfig = serde_json::from_str(&decrypted)
                .map_err(|e| EncryptionError::Deserialization(e.to_string()))?;
            Ok(config)
        } else {
            // Partial encryption - reconstruct config
            let mut config = FullConfig::default();

            if let Some(section) = encrypted.sections.get("app_config") {
                let decrypted = self.decrypt_section(section)?;
                config.app_config = serde_json::from_str(&decrypted)
                    .map_err(|e| EncryptionError::Deserialization(e.to_string()))?;
            }

            if let Some(section) = encrypted.sections.get("user_preferences") {
                let decrypted = self.decrypt_section(section)?;
                let mut prefs: UserPreferences = serde_json::from_str(&decrypted)
                    .map_err(|e| EncryptionError::Deserialization(e.to_string()))?;
                // Decrypt sensitive fields
                self.decrypt_sensitive_fields(&mut prefs)?;
                config.user_preferences = prefs;
            }

            if let Some(section) = encrypted.sections.get("security_settings") {
                let decrypted = self.decrypt_section(section)?;
                config.security_settings = serde_json::from_str(&decrypted)
                    .map_err(|e| EncryptionError::Deserialization(e.to_string()))?;
            }

            // Restore metadata
            if let Some(ref metadata) = encrypted.metadata {
                config.version = metadata.version;
            }

            Ok(config)
        }
    }

    /// Verify master password
    pub fn verify_password(&self, password: &str) -> Result<bool, EncryptionError> {
        let mut test_crypto = CryptoState::new();
        test_crypto.initialize(password)?;

        // Create a test encryption and try to decrypt
        let test_data = b"test";
        let encrypted = self.crypto.encrypt(test_data)?;
        let decrypted = test_crypto.decrypt(&encrypted);

        Ok(decrypted.is_ok())
    }

    /// Change master password
    pub fn change_password(
        &mut self,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), EncryptionError> {
        if !self.verify_password(old_password)? {
            return Err(EncryptionError::InvalidPassword);
        }

        self.crypto = CryptoState::new();
        self.crypto.initialize(new_password)?;

        Ok(())
    }

    /// Encrypt a section of data
    fn encrypt_section(&self, data: &str) -> Result<EncryptedSection, EncryptionError> {
        let encrypted = self.crypto.encrypt(data.as_bytes())?;
        // Nonce is first 12 bytes of encrypted data
        let nonce = if encrypted.len() >= 12 {
            hex_encode(&encrypted[..12])
        } else {
            String::new()
        };

        Ok(EncryptedSection {
            data: base64_encode(&encrypted),
            nonce,
        })
    }

    /// Decrypt a section of data
    fn decrypt_section(&self, section: &EncryptedSection) -> Result<String, EncryptionError> {
        let encrypted_data = base64_decode(&section.data)
            .map_err(|e| EncryptionError::Decryption(format!("Base64 decode failed: {}", e)))?;

        let decrypted = self.crypto.decrypt(&encrypted_data)?;
        String::from_utf8(decrypted)
            .map_err(|e| EncryptionError::Decryption(format!("UTF-8 decode failed: {}", e)))
    }

    /// Encrypt sensitive fields within UserPreferences
    fn encrypt_sensitive_fields(&self, prefs: &mut UserPreferences) -> Result<(), EncryptionError> {
        // Encrypt default_key_path if present
        if let Some(ref key_path) = prefs.default_key_path {
            if !key_path.starts_with("ENC:") {
                let encrypted = self.crypto.encrypt(key_path.as_bytes())?;
                prefs.default_key_path = Some(format!("ENC:{}", base64_encode(&encrypted)));
            }
        }

        Ok(())
    }

    /// Decrypt sensitive fields within UserPreferences
    fn decrypt_sensitive_fields(&self, prefs: &mut UserPreferences) -> Result<(), EncryptionError> {
        // Decrypt default_key_path if encrypted
        if let Some(ref key_path) = prefs.default_key_path {
            if let Some(encrypted) = key_path.strip_prefix("ENC:") {
                let encrypted_data = base64_decode(encrypted).map_err(|e| {
                    EncryptionError::Decryption(format!("Base64 decode failed: {}", e))
                })?;
                let decrypted = self.crypto.decrypt(&encrypted_data)?;
                prefs.default_key_path = Some(String::from_utf8_lossy(&decrypted).to_string());
            }
        }

        Ok(())
    }

    /// Serialize encrypted config to JSON
    pub fn to_json(&self, encrypted: &EncryptedConfig) -> Result<String, EncryptionError> {
        serde_json::to_string_pretty(encrypted)
            .map_err(|e| EncryptionError::Serialization(e.to_string()))
    }

    /// Deserialize encrypted config from JSON
    pub fn from_json(json: &str) -> Result<EncryptedConfig, EncryptionError> {
        serde_json::from_str(json).map_err(|e| EncryptionError::Deserialization(e.to_string()))
    }
}

/// Utility functions for password handling
pub mod password {
    use super::*;

    /// Assess password strength
    pub fn assess_strength(password: &str) -> PasswordStrength {
        let length = password.len();
        let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        let mut score = 0;

        // Length scoring
        if length >= 16 {
            score += 4;
        } else if length >= 12 {
            score += 3;
        } else if length >= 8 {
            score += 2;
        } else if length >= 6 {
            score += 1;
        }

        // Character variety scoring
        if has_upper {
            score += 1;
        }
        if has_lower {
            score += 1;
        }
        if has_digit {
            score += 1;
        }
        if has_special {
            score += 2;
        }

        match score {
            0..=2 => PasswordStrength::VeryWeak,
            3..=4 => PasswordStrength::Weak,
            5..=6 => PasswordStrength::Medium,
            7..=8 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        }
    }

    /// Validate password against requirements
    pub fn validate(
        password: &str,
        requirements: &PasswordRequirements,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if password.len() < requirements.min_length {
            errors.push(format!(
                "Password must be at least {} characters",
                requirements.min_length
            ));
        }

        if requirements.require_uppercase && !password.chars().any(|c| c.is_ascii_uppercase()) {
            errors.push("Password must contain uppercase letters".to_string());
        }

        if requirements.require_lowercase && !password.chars().any(|c| c.is_ascii_lowercase()) {
            errors.push("Password must contain lowercase letters".to_string());
        }

        if requirements.require_digits && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("Password must contain digits".to_string());
        }

        if requirements.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
            errors.push("Password must contain special characters".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Generate a random password
    pub fn generate(length: usize, include_special: bool) -> String {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let lowercase = "abcdefghijklmnopqrstuvwxyz";
        let uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let digits = "0123456789";
        let special = "!@#$%^&*()_+-=[]{}|;:,.<>?";

        let mut chars: Vec<char> = Vec::new();
        chars.extend(lowercase.chars());
        chars.extend(uppercase.chars());
        chars.extend(digits.chars());
        if include_special {
            chars.extend(special.chars());
        }

        let mut rng = thread_rng();
        let password: String = (0..length)
            .map(|_| *chars.choose(&mut rng).unwrap())
            .collect();

        password
    }
}

/// Encoding helpers
fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD
        .decode(s)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_strength() {
        assert_eq!(password::assess_strength("123"), PasswordStrength::VeryWeak);
        assert_eq!(
            password::assess_strength("password"),
            PasswordStrength::Weak
        );
        assert_eq!(
            password::assess_strength("Password1"),
            PasswordStrength::Medium
        );
        assert_eq!(
            password::assess_strength("MyStr0ng!Pass"),
            PasswordStrength::Strong
        );
        assert_eq!(
            password::assess_strength("My$up3r$tr0ng!P@ssw0rd"),
            PasswordStrength::VeryStrong
        );
    }

    #[test]
    fn test_password_validation() {
        let req = PasswordRequirements::default();
        assert!(password::validate("Password1", &req).is_ok());
        assert!(password::validate("pass", &req).is_err());
    }

    #[test]
    fn test_password_generation() {
        let pwd1 = password::generate(16, true);
        assert_eq!(pwd1.len(), 16);

        let pwd2 = password::generate(12, false);
        assert_eq!(pwd2.len(), 12);
        assert!(!pwd2.chars().any(|c| !c.is_alphanumeric()));
    }

    #[test]
    fn test_encryption_options() {
        let opts = EncryptionOptions::default();
        assert!(!opts.full_encryption);
        assert!(opts.encrypt_user_preferences);

        let full = EncryptionOptions::full_encryption();
        assert!(full.full_encryption);
    }

    #[test]
    fn test_security_level_options() {
        let standard = EncryptionOptions::with_security_level(SecurityLevel::Standard);
        assert_eq!(standard.argon2_iterations, 3);
        assert_eq!(standard.argon2_memory_kb, 65536);

        let high = EncryptionOptions::with_security_level(SecurityLevel::High);
        assert_eq!(high.argon2_memory_kb, 262144);

        let max = EncryptionOptions::with_security_level(SecurityLevel::Maximum);
        assert_eq!(max.argon2_memory_kb, 1048576);
        assert_eq!(max.argon2_parallelism, 8);
    }
}
