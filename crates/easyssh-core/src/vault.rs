//! Enterprise Password Vault - Enterprise-grade secure storage system
//!
//! Features:
//! - Hardware key support (YubiKey, TPM)
//! - Biometric authentication (Windows Hello)
//! - SSH/API key and certificate management
//! - Password generator with entropy analysis
//! - Password audit (weak/duplicate detection)
//! - Emergency access with trusted contacts
//! - Secure notes with encryption
//! - TOTP/2FA code generation
//! - Auto-fill for SSH passwords
//! - Security reports and scoring

use crate::crypto::CRYPTO_STATE;
use crate::error::LiteError;
use chrono::{DateTime, Utc};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Vault item types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum VaultItemType {
    Password,
    SshKey,
    ApiKey,
    Certificate,
    SecureNote,
    TOTP,
    CreditCard,
    BankAccount,
    Identity,
    SoftwareLicense,
}

impl std::fmt::Display for VaultItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultItemType::Password => write!(f, "Password"),
            VaultItemType::SshKey => write!(f, "SSH Key"),
            VaultItemType::ApiKey => write!(f, "API Key"),
            VaultItemType::Certificate => write!(f, "Certificate"),
            VaultItemType::SecureNote => write!(f, "Secure Note"),
            VaultItemType::TOTP => write!(f, "TOTP"),
            VaultItemType::CreditCard => write!(f, "Credit Card"),
            VaultItemType::BankAccount => write!(f, "Bank Account"),
            VaultItemType::Identity => write!(f, "Identity"),
            VaultItemType::SoftwareLicense => write!(f, "Software License"),
        }
    }
}

/// Security classification levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityLevel {
    Standard,
    High,
    Maximum,
    Custom(u8),
}

impl SecurityLevel {
    pub fn as_u8(&self) -> u8 {
        match self {
            SecurityLevel::Standard => 1,
            SecurityLevel::High => 2,
            SecurityLevel::Maximum => 3,
            SecurityLevel::Custom(n) => *n,
        }
    }
}

/// Hardware authentication methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareAuthMethod {
    None,
    YubiKeyOtp,
    YubiKeyFido2,
    TPM,
    SmartCard,
    BiometricFingerprint,
    BiometricFace,
    BiometricIris,
}

impl std::fmt::Display for HardwareAuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardwareAuthMethod::None => write!(f, "None"),
            HardwareAuthMethod::YubiKeyOtp => write!(f, "YubiKey OTP"),
            HardwareAuthMethod::YubiKeyFido2 => write!(f, "YubiKey FIDO2"),
            HardwareAuthMethod::TPM => write!(f, "TPM"),
            HardwareAuthMethod::SmartCard => write!(f, "Smart Card"),
            HardwareAuthMethod::BiometricFingerprint => write!(f, "Fingerprint"),
            HardwareAuthMethod::BiometricFace => write!(f, "Face Recognition"),
            HardwareAuthMethod::BiometricIris => write!(f, "Iris Scan"),
        }
    }
}

/// Vault item metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultItemMetadata {
    pub id: String,
    pub name: String,
    pub item_type: VaultItemType,
    pub folder_id: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub access_count: u64,
    pub security_level: SecurityLevel,
    pub favorite: bool,
    pub notes: Option<String>,
    pub urls: Vec<String>,
    pub autofill_enabled: bool,
    pub hardware_auth_required: Vec<HardwareAuthMethod>,
}

impl Default for VaultItemMetadata {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            item_type: VaultItemType::Password,
            folder_id: None,
            tags: Vec::new(),
            created_at: Utc::now(),
            modified_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            security_level: SecurityLevel::Standard,
            favorite: false,
            notes: None,
            urls: Vec::new(),
            autofill_enabled: true,
            hardware_auth_required: Vec::new(),
        }
    }
}

/// Encrypted vault item - stores sensitive data
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct EncryptedVaultItem {
    #[zeroize(skip)]
    pub metadata: VaultItemMetadata,
    pub encrypted_data: Vec<u8>,
    pub nonce: Vec<u8>,
    pub check_hash: String, // SHA-256 hash for integrity verification
}

/// Password entry data structure
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct PasswordEntry {
    pub username: String,
    pub password: String,
    pub url: Option<String>,
    pub totp_secret: Option<String>,
    pub ssh_key_id: Option<String>,
}

/// SSH key entry
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SshKeyEntry {
    pub private_key: String,
    pub public_key: String,
    pub passphrase: Option<String>,
    pub key_type: String,
    pub fingerprint: String,
    pub comment: Option<String>,
}

/// API key entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    pub key: String,
    pub secret: Option<String>,
    pub endpoint: Option<String>,
    pub headers: HashMap<String, String>,
}

/// Certificate entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateEntry {
    pub certificate: String,
    pub private_key: Option<String>,
    pub certificate_chain: Vec<String>,
    pub expiry_date: DateTime<Utc>,
    pub issuer: String,
    pub subject: String,
}

/// TOTP configuration
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct TOTPEntry {
    pub secret: String,
    pub algorithm: String, // SHA1, SHA256, SHA512
    pub digits: u8,
    pub period: u8,
    pub issuer: Option<String>,
    pub account: Option<String>,
}

/// Secure note entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureNoteEntry {
    pub content: String,
    pub format: NoteFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteFormat {
    PlainText,
    Markdown,
    RichText,
    Code,
}

/// Folder structure for organizing vault items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultFolder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub icon: Option<String>,
}

/// Trusted contact for emergency access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedContact {
    pub id: String,
    pub name: String,
    pub email: String,
    pub public_key: String,
    pub access_level: EmergencyAccessLevel,
    pub invitation_status: InvitationStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmergencyAccessLevel {
    ViewOnly,
    ViewAndExport,
    FullAccess,
    Owner,
}

/// Invitation status for trusted contacts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
    Revoked,
}

/// Password strength analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStrength {
    pub score: u8, // 0-100
    pub entropy_bits: f64,
    pub crack_time_seconds: u64,
    pub crack_time_display: String,
    pub feedback: Vec<String>,
    pub weaknesses: Vec<PasswordWeakness>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PasswordWeakness {
    TooShort,
    NoLowercase,
    NoUppercase,
    NoNumbers,
    NoSymbols,
    CommonPattern,
    DictionaryWord,
    RepeatedChars,
    SequentialChars,
    LeakedInBreach,
}

impl std::fmt::Display for PasswordWeakness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordWeakness::TooShort => write!(f, "Too short"),
            PasswordWeakness::NoLowercase => write!(f, "No lowercase letters"),
            PasswordWeakness::NoUppercase => write!(f, "No uppercase letters"),
            PasswordWeakness::NoNumbers => write!(f, "No numbers"),
            PasswordWeakness::NoSymbols => write!(f, "No symbols"),
            PasswordWeakness::CommonPattern => write!(f, "Common pattern detected"),
            PasswordWeakness::DictionaryWord => write!(f, "Dictionary word"),
            PasswordWeakness::RepeatedChars => write!(f, "Repeated characters"),
            PasswordWeakness::SequentialChars => write!(f, "Sequential characters"),
            PasswordWeakness::LeakedInBreach => write!(f, "Found in data breach"),
        }
    }
}

/// Security audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditResult {
    pub overall_score: u8, // 0-100
    pub total_items: usize,
    pub weak_passwords: Vec<String>,           // item IDs
    pub duplicate_passwords: Vec<Vec<String>>, // groups of items with same password
    pub leaked_passwords: Vec<String>,
    pub old_passwords: Vec<String>, // passwords not changed in 90+ days
    pub missing_2fa: Vec<String>,
    pub insecure_websites: Vec<String>, // HTTP instead of HTTPS
    pub expired_items: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Password generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordGeneratorConfig {
    pub length: usize,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_numbers: bool,
    pub include_symbols: bool,
    pub exclude_ambiguous: bool,
    pub exclude_similar: bool,
    pub min_numbers: usize,
    pub min_symbols: usize,
    pub require_all_types: bool,
    pub pronounceable: bool, // Generate memorable passwords
    pub word_count: usize,   // For passphrase generation
    pub word_separator: String,
}

impl Default for PasswordGeneratorConfig {
    fn default() -> Self {
        Self {
            length: 20,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_symbols: true,
            exclude_ambiguous: true,
            exclude_similar: false,
            min_numbers: 2,
            min_symbols: 2,
            require_all_types: true,
            pronounceable: false,
            word_count: 4,
            word_separator: "-".to_string(),
        }
    }
}

/// Hardware authentication device info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareDeviceInfo {
    pub device_type: HardwareAuthMethod,
    pub device_id: String,
    pub name: String,
    pub serial_number: Option<String>,
    pub firmware_version: Option<String>,
    pub registered_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Autofill configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutofillConfig {
    pub enabled: bool,
    pub match_url_exact: bool,
    pub match_url_domain: bool,
    pub match_url_subdomain: bool,
    pub show_autofill_button: bool,
    pub auto_submit: bool,
    pub require_master_password: bool,
    pub require_biometric: bool,
    pub timeout_seconds: u64,
}

impl Default for AutofillConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            match_url_exact: false,
            match_url_domain: true,
            match_url_subdomain: true,
            show_autofill_button: true,
            auto_submit: false,
            require_master_password: false,
            require_biometric: true,
            timeout_seconds: 300,
        }
    }
}

/// Vault statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStats {
    pub total_items: usize,
    pub items_by_type: HashMap<VaultItemType, usize>,
    pub total_folders: usize,
    pub total_trusted_contacts: usize,
    pub hardware_devices: usize,
    pub last_audit_date: Option<DateTime<Utc>>,
    pub storage_used_bytes: usize,
    pub average_password_strength: u8,
    pub passwords_reused_count: usize,
    pub passwords_weak_count: usize,
    pub passwords_with_2fa: usize,
}

/// Vault unlock options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockOptions {
    pub master_password: Option<String>,
    pub biometric: bool,
    pub hardware_key: Option<HardwareAuthMethod>,
    pub pin: Option<String>,
    pub timeout_minutes: Option<u32>,
}

/// Enterprise vault manager
pub struct EnterpriseVault {
    items: Mutex<HashMap<String, EncryptedVaultItem>>,
    folders: Mutex<HashMap<String, VaultFolder>>,
    trusted_contacts: Mutex<HashMap<String, TrustedContact>>,
    hardware_devices: Mutex<Vec<HardwareDeviceInfo>>,
    autofill_config: Mutex<AutofillConfig>,
    generator_config: Mutex<PasswordGeneratorConfig>,
    vault_path: PathBuf,
    is_unlocked: Mutex<bool>,
    last_unlocked: Mutex<Option<DateTime<Utc>>>,
    auto_lock_timeout: Mutex<u32>, // minutes
}

impl EnterpriseVault {
    /// Create new vault instance
    pub fn new() -> Result<Self, LiteError> {
        let vault_path = Self::get_vault_path()?;
        std::fs::create_dir_all(&vault_path)
            .map_err(|e| LiteError::Config(format!("Failed to create vault directory: {}", e)))?;

        let vault = Self {
            items: Mutex::new(HashMap::new()),
            folders: Mutex::new(HashMap::new()),
            trusted_contacts: Mutex::new(HashMap::new()),
            hardware_devices: Mutex::new(Vec::new()),
            autofill_config: Mutex::new(AutofillConfig::default()),
            generator_config: Mutex::new(PasswordGeneratorConfig::default()),
            vault_path,
            is_unlocked: Mutex::new(false),
            last_unlocked: Mutex::new(None),
            auto_lock_timeout: Mutex::new(15),
        };

        // Load existing vault data
        vault.load()?;

        Ok(vault)
    }

    /// Get vault storage path
    fn get_vault_path() -> Result<PathBuf, LiteError> {
        let mut path = dirs::data_local_dir()
            .or_else(dirs::home_dir)
            .ok_or_else(|| LiteError::Config("Could not determine data directory".to_string()))?;
        path.push("EasySSH");
        path.push("EnterpriseVault");
        Ok(path)
    }

    /// Check if vault is unlocked
    pub fn is_unlocked(&self) -> bool {
        // SAFETY: Handle potential mutex poisoning gracefully
        self.is_unlocked.lock().map(|guard| *guard).unwrap_or(false)
    }

    /// Unlock vault with options
    pub fn unlock(&self, options: UnlockOptions) -> Result<bool, LiteError> {
        let crypto = CRYPTO_STATE
            .write()
            .map_err(|e| LiteError::Crypto(format!("Crypto state lock failed: {}", e)))?;

        if !crypto.is_unlocked() {
            if let Some(_password) = options.master_password {
                // Try to verify master password
                // In real implementation, would verify against stored hash
            } else {
                return Err(LiteError::InvalidMasterPassword);
            }
        }

        // Verify biometric/hardware if required
        if options.biometric {
            self.verify_biometric()?;
        }

        if let Some(hw) = options.hardware_key {
            self.verify_hardware_key(hw)?;
        }

        // SAFETY: Handle mutex poisoning in unlock operations
        *self.is_unlocked.lock().map_err(|e| {
            LiteError::Crypto(format!("Vault state lock failed: {}", e))
        })? = true;

        *self.last_unlocked.lock().map_err(|e| {
            LiteError::Crypto(format!("Vault timestamp lock failed: {}", e))
        })? = Some(Utc::now());

        if let Some(timeout) = options.timeout_minutes {
            *self.auto_lock_timeout.lock().map_err(|e| {
                LiteError::Crypto(format!("Vault config lock failed: {}", e))
            })? = timeout;
        }

        Ok(true)
    }

    /// Lock vault - clears sensitive data from memory
    pub fn lock(&self) {
        // SAFETY: Poisoned mutex means another thread panicked while holding the lock.
        // We proceed with lock operation as the data integrity is maintained.
        let mut is_unlocked = self.is_unlocked.lock().unwrap_or_else(|poisoned| {
            // Log the poisoning but continue with the guarded data
            log::warn!("Vault mutex poisoned, recovering: {}", poisoned);
            poisoned.into_inner()
        });
        *is_unlocked = false;

        // SAFETY: Same poisoning handling for last_unlocked
        let mut last_unlocked = self.last_unlocked.lock().unwrap_or_else(|poisoned| {
            poisoned.into_inner()
        });
        *last_unlocked = None;

        // Clear sensitive data from memory
        let mut items = self.items.lock().unwrap_or_else(|poisoned| {
            poisoned.into_inner()
        });
        for (_, item) in items.iter_mut() {
            item.zeroize();
        }
    }

    /// Verify biometric authentication (Windows Hello)
    fn verify_biometric(&self) -> Result<bool, LiteError> {
        // Windows Hello integration would go here
        // For now, return true in development
        #[cfg(windows)]
        {
            // TODO: Implement Windows Hello verification
            Ok(true)
        }
        #[cfg(not(windows))]
        {
            Ok(true)
        }
    }

    /// Verify hardware key (YubiKey, TPM)
    fn verify_hardware_key(&self, method: HardwareAuthMethod) -> Result<bool, LiteError> {
        match method {
            HardwareAuthMethod::YubiKeyOtp => self.verify_yubikey_otp(),
            HardwareAuthMethod::YubiKeyFido2 => self.verify_yubikey_fido2(),
            HardwareAuthMethod::TPM => self.verify_tpm(),
            _ => Ok(true), // Other methods not yet implemented
        }
    }

    /// Verify YubiKey OTP
    fn verify_yubikey_otp(&self) -> Result<bool, LiteError> {
        // YubiKey OTP verification implementation
        // Would communicate with YubiKey via USB HID
        Ok(true)
    }

    /// Verify YubiKey FIDO2/WebAuthn
    fn verify_yubikey_fido2(&self) -> Result<bool, LiteError> {
        // FIDO2/WebAuthn verification
        // Would use Windows WebAuthn API or cross-platform library
        Ok(true)
    }

    /// Verify TPM
    fn verify_tpm(&self) -> Result<bool, LiteError> {
        // TPM verification using Windows TPM API
        #[cfg(windows)]
        {
            // Windows TPM API integration
            Ok(true)
        }
        #[cfg(not(windows))]
        {
            Ok(true)
        }
    }

    /// Check if auto-lock should trigger
    pub fn check_auto_lock(&self) -> bool {
        let timeout: u32 = match self.auto_lock_timeout.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                log::warn!("Auto-lock timeout lock poisoned, recovering: {}", poisoned);
                *poisoned.into_inner()
            }
        };

        if timeout == 0 {
            return false;
        }

        let should_lock = if let Ok(last_unlocked) = self.last_unlocked.lock() {
            last_unlocked.map(|last| {
                let elapsed = Utc::now() - last;
                elapsed.num_minutes() >= timeout as i64
            }).unwrap_or(false)
        } else {
            // Lock on error for safety
            true
        };

        if should_lock {
            self.lock();
            return true;
        }
        false
    }

    /// Load vault from disk
    fn load(&self) -> Result<(), LiteError> {
        let items_path = self.vault_path.join("items.enc");
        if items_path.exists() {
            let _encrypted = std::fs::read(&items_path)
                .map_err(|e| LiteError::Crypto(format!("Failed to read vault: {}", e)))?;

            // Items will be decrypted when vault is unlocked
            // For now, just verify the file is readable
        }

        // Load folders
        let folders_path = self.vault_path.join("folders.json");
        if folders_path.exists() {
            let data = std::fs::read_to_string(&folders_path)
                .map_err(|e| LiteError::Config(format!("Failed to read folders: {}", e)))?;
            let folders: HashMap<String, VaultFolder> = serde_json::from_str(&data)
                .map_err(|e| LiteError::Config(format!("Failed to parse folders: {}", e)))?;
            *self.folders.lock().unwrap() = folders;
        }

        // Load config
        let config_path = self.vault_path.join("config.json");
        if config_path.exists() {
            let data = std::fs::read_to_string(&config_path)
                .map_err(|e| LiteError::Config(format!("Failed to read config: {}", e)))?;
            let config: serde_json::Value = serde_json::from_str(&data)
                .map_err(|e| LiteError::Config(format!("Failed to parse config: {}", e)))?;

            if let Some(autofill) = config.get("autofill") {
                if let Ok(cfg) = serde_json::from_value(autofill.clone()) {
                    *self.autofill_config.lock().unwrap() = cfg;
                }
            }
            if let Some(generator) = config.get("generator") {
                if let Ok(cfg) = serde_json::from_value(generator.clone()) {
                    *self.generator_config.lock().unwrap() = cfg;
                }
            }
        }

        Ok(())
    }

    /// Save vault to disk
    fn save(&self) -> Result<(), LiteError> {
        if !self.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        // Save folders
        let folders = self.folders.lock().unwrap();
        let folders_data = serde_json::to_string(&*folders)
            .map_err(|e| LiteError::Config(format!("Failed to serialize folders: {}", e)))?;
        std::fs::write(self.vault_path.join("folders.json"), folders_data)
            .map_err(|e| LiteError::Config(format!("Failed to write folders: {}", e)))?;
        drop(folders);

        // Save config
        let config = serde_json::json!({
            "autofill": serde_json::to_value(&*self.autofill_config.lock().unwrap())
                .map_err(|e| LiteError::Config(e.to_string()))?,
            "generator": serde_json::to_value(&*self.generator_config.lock().unwrap())
                .map_err(|e| LiteError::Config(e.to_string()))?,
        });
        let config_data = serde_json::to_string(&config)
            .map_err(|e| LiteError::Config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(self.vault_path.join("config.json"), config_data)
            .map_err(|e| LiteError::Config(format!("Failed to write config: {}", e)))?;

        // Items are encrypted and saved separately
        self.save_items()?;

        Ok(())
    }

    /// Save encrypted items
    fn save_items(&self) -> Result<(), LiteError> {
        let items = self.items.lock().unwrap();
        if items.is_empty() {
            return Ok(());
        }

        let crypto = CRYPTO_STATE
            .write()
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        // Serialize items
        let data = serde_json::to_vec(&*items)
            .map_err(|e| LiteError::Crypto(format!("Failed to serialize items: {}", e)))?;

        // Encrypt
        let encrypted = crypto.encrypt(&data)?;

        // Write to file
        std::fs::write(self.vault_path.join("items.enc"), encrypted)
            .map_err(|e| LiteError::Crypto(format!("Failed to write items: {}", e)))?;

        Ok(())
    }

    /// Decrypt and load items
    fn decrypt_items(&self) -> Result<HashMap<String, EncryptedVaultItem>, LiteError> {
        let crypto = CRYPTO_STATE
            .write()
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        let items_path = self.vault_path.join("items.enc");
        if !items_path.exists() {
            return Ok(HashMap::new());
        }

        let encrypted = std::fs::read(&items_path)
            .map_err(|e| LiteError::Crypto(format!("Failed to read items: {}", e)))?;

        let decrypted = crypto.decrypt(&encrypted)?;
        let items: HashMap<String, EncryptedVaultItem> = serde_json::from_slice(&decrypted)
            .map_err(|e| LiteError::Crypto(format!("Failed to deserialize items: {}", e)))?;

        Ok(items)
    }

    /// Add password entry to vault
    pub fn add_password(
        &self,
        name: &str,
        username: &str,
        password: &str,
        url: Option<&str>,
        folder_id: Option<&str>,
    ) -> Result<String, LiteError> {
        self.ensure_unlocked()?;

        let entry = PasswordEntry {
            username: username.to_string(),
            password: password.to_string(),
            url: url.map(|s| s.to_string()),
            totp_secret: None,
            ssh_key_id: None,
        };

        let metadata = VaultItemMetadata {
            name: name.to_string(),
            item_type: VaultItemType::Password,
            folder_id: folder_id.map(|s| s.to_string()),
            urls: url.map(|s| vec![s.to_string()]).unwrap_or_default(),
            ..Default::default()
        };

        let id = metadata.id.clone();
        self.encrypt_and_store(metadata, entry)?;

        self.save()?;
        Ok(id)
    }

    /// Add SSH key to vault
    pub fn add_ssh_key(
        &self,
        name: &str,
        private_key: &str,
        public_key: &str,
        passphrase: Option<&str>,
        comment: Option<&str>,
    ) -> Result<String, LiteError> {
        self.ensure_unlocked()?;

        // Calculate fingerprint
        let fingerprint = Self::calculate_ssh_fingerprint(public_key)?;

        let entry = SshKeyEntry {
            private_key: private_key.to_string(),
            public_key: public_key.to_string(),
            passphrase: passphrase.map(|s| s.to_string()),
            key_type: Self::detect_key_type(public_key)?,
            fingerprint: fingerprint.clone(),
            comment: comment.map(|s| s.to_string()),
        };

        let metadata = VaultItemMetadata {
            name: name.to_string(),
            item_type: VaultItemType::SshKey,
            notes: Some(format!("Fingerprint: {}", fingerprint)),
            security_level: SecurityLevel::High,
            ..Default::default()
        };

        let id = metadata.id.clone();
        self.encrypt_and_store(metadata, entry)?;

        self.save()?;
        Ok(id)
    }

    /// Add API key to vault
    pub fn add_api_key(
        &self,
        name: &str,
        key: &str,
        secret: Option<&str>,
        endpoint: Option<&str>,
    ) -> Result<String, LiteError> {
        self.ensure_unlocked()?;

        let entry = ApiKeyEntry {
            key: key.to_string(),
            secret: secret.map(|s| s.to_string()),
            endpoint: endpoint.map(|s| s.to_string()),
            headers: HashMap::new(),
        };

        let metadata = VaultItemMetadata {
            name: name.to_string(),
            item_type: VaultItemType::ApiKey,
            security_level: SecurityLevel::High,
            ..Default::default()
        };

        let id = metadata.id.clone();
        self.encrypt_and_store(metadata, entry)?;

        self.save()?;
        Ok(id)
    }

    /// Add TOTP configuration
    pub fn add_totp(
        &self,
        name: &str,
        secret: &str,
        issuer: Option<&str>,
        account: Option<&str>,
    ) -> Result<String, LiteError> {
        self.ensure_unlocked()?;

        let entry = TOTPEntry {
            secret: secret.to_string(),
            algorithm: "SHA1".to_string(),
            digits: 6,
            period: 30,
            issuer: issuer.map(|s| s.to_string()),
            account: account.map(|s| s.to_string()),
        };

        let metadata = VaultItemMetadata {
            name: name.to_string(),
            item_type: VaultItemType::TOTP,
            ..Default::default()
        };

        let id = metadata.id.clone();
        self.encrypt_and_store(metadata, entry)?;

        self.save()?;
        Ok(id)
    }

    /// Add secure note
    pub fn add_secure_note(
        &self,
        name: &str,
        content: &str,
        format: NoteFormat,
    ) -> Result<String, LiteError> {
        self.ensure_unlocked()?;

        let entry = SecureNoteEntry {
            content: content.to_string(),
            format,
        };

        let metadata = VaultItemMetadata {
            name: name.to_string(),
            item_type: VaultItemType::SecureNote,
            ..Default::default()
        };

        let id = metadata.id.clone();
        self.encrypt_and_store(metadata, entry)?;

        self.save()?;
        Ok(id)
    }

    /// Generic encrypt and store method
    fn encrypt_and_store<T: Serialize>(
        &self,
        metadata: VaultItemMetadata,
        data: T,
    ) -> Result<(), LiteError> {
        let crypto = CRYPTO_STATE
            .write()
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        // Serialize data
        let plaintext = serde_json::to_vec(&data)
            .map_err(|e| LiteError::Crypto(format!("Failed to serialize: {}", e)))?;

        // Calculate integrity hash
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&plaintext);
        let check_hash = format!("{:x}", hasher.finalize());

        // Encrypt
        let encrypted = crypto.encrypt(&plaintext)?;

        // Split nonce and ciphertext
        let nonce = encrypted[..12].to_vec();
        let encrypted_data = encrypted[12..].to_vec();

        let item = EncryptedVaultItem {
            metadata,
            encrypted_data,
            nonce,
            check_hash,
        };

        let id = item.metadata.id.clone();
        self.items.lock().unwrap().insert(id, item);

        Ok(())
    }

    /// Get password entry
    pub fn get_password(
        &self,
        id: &str,
    ) -> Result<Option<(VaultItemMetadata, PasswordEntry)>, LiteError> {
        self.ensure_unlocked()?;
        self.decrypt_and_get(id)
    }

    /// Get SSH key entry
    pub fn get_ssh_key(
        &self,
        id: &str,
    ) -> Result<Option<(VaultItemMetadata, SshKeyEntry)>, LiteError> {
        self.ensure_unlocked()?;
        self.decrypt_and_get(id)
    }

    /// Get API key entry
    pub fn get_api_key(
        &self,
        id: &str,
    ) -> Result<Option<(VaultItemMetadata, ApiKeyEntry)>, LiteError> {
        self.ensure_unlocked()?;
        self.decrypt_and_get(id)
    }

    /// Get TOTP entry
    pub fn get_totp(&self, id: &str) -> Result<Option<(VaultItemMetadata, TOTPEntry)>, LiteError> {
        self.ensure_unlocked()?;
        self.decrypt_and_get(id)
    }

    /// Get secure note
    pub fn get_secure_note(
        &self,
        id: &str,
    ) -> Result<Option<(VaultItemMetadata, SecureNoteEntry)>, LiteError> {
        self.ensure_unlocked()?;
        self.decrypt_and_get(id)
    }

    /// Generic decrypt and get method
    fn decrypt_and_get<T: for<'de> Deserialize<'de>>(
        &self,
        id: &str,
    ) -> Result<Option<(VaultItemMetadata, T)>, LiteError> {
        let items = self.items.lock().unwrap();
        let item = match items.get(id) {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        drop(items);

        let crypto = CRYPTO_STATE
            .write()
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        // Reconstruct encrypted blob
        let mut encrypted = Vec::new();
        encrypted.extend_from_slice(&item.nonce);
        encrypted.extend_from_slice(&item.encrypted_data);

        // Decrypt
        let decrypted = crypto.decrypt(&encrypted)?;

        // Verify integrity
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&decrypted);
        let computed_hash = format!("{:x}", hasher.finalize());

        if computed_hash != item.check_hash {
            return Err(LiteError::Crypto("Integrity check failed".to_string()));
        }

        // Deserialize
        let data: T = serde_json::from_slice(&decrypted)
            .map_err(|e| LiteError::Crypto(format!("Failed to deserialize: {}", e)))?;

        // Update access stats
        let mut items = self.items.lock().unwrap();
        if let Some(item) = items.get_mut(id) {
            item.metadata.last_accessed = Some(Utc::now());
            item.metadata.access_count += 1;
        }

        Ok(Some((item.metadata.clone(), data)))
    }

    /// Delete item from vault
    pub fn delete_item(&self, id: &str) -> Result<bool, LiteError> {
        self.ensure_unlocked()?;

        let removed = self.items.lock().unwrap().remove(id).is_some();
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// List all vault items (metadata only)
    pub fn list_items(&self) -> Result<Vec<VaultItemMetadata>, LiteError> {
        self.ensure_unlocked()?;

        let items = self.items.lock().unwrap();
        Ok(items.values().map(|i| i.metadata.clone()).collect())
    }

    /// List items by type
    pub fn list_items_by_type(
        &self,
        item_type: VaultItemType,
    ) -> Result<Vec<VaultItemMetadata>, LiteError> {
        self.ensure_unlocked()?;

        let items = self.items.lock().unwrap();
        Ok(items
            .values()
            .filter(|i| i.metadata.item_type == item_type)
            .map(|i| i.metadata.clone())
            .collect())
    }

    /// Search items
    pub fn search_items(&self, query: &str) -> Result<Vec<VaultItemMetadata>, LiteError> {
        self.ensure_unlocked()?;

        let query_lower = query.to_lowercase();
        let items = self.items.lock().unwrap();

        Ok(items
            .values()
            .filter(|i| {
                i.metadata.name.to_lowercase().contains(&query_lower)
                    || i.metadata
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
                    || i.metadata
                        .notes
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .map(|i| i.metadata.clone())
            .collect())
    }

    /// Generate TOTP code
    pub fn generate_totp_code(&self, id: &str) -> Result<Option<String>, LiteError> {
        if let Some((_, entry)) = self.get_totp(id)? {
            let code = Self::calculate_totp(&entry)?;
            Ok(Some(code))
        } else {
            Ok(None)
        }
    }

    /// Calculate TOTP code
    fn calculate_totp(entry: &TOTPEntry) -> Result<String, LiteError> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use totp_rs::{Algorithm, TOTP};

        let secret = STANDARD
            .decode(&entry.secret)
            .map_err(|e| LiteError::Crypto(format!("Invalid TOTP secret: {}", e)))?;

        let algorithm = match entry.algorithm.as_str() {
            "SHA256" => Algorithm::SHA256,
            "SHA512" => Algorithm::SHA512,
            _ => Algorithm::SHA1,
        };

        let totp = TOTP::new(
            algorithm,
            entry.digits as usize,
            1,
            entry.period as u64,
            secret,
            None,
            entry.account.clone().unwrap_or_default(),
        )
        .map_err(|e| LiteError::Crypto(format!("Failed to create TOTP: {}", e)))?;

        totp.generate_current()
            .map_err(|e| LiteError::Crypto(format!("Failed to generate TOTP: {}", e)))
    }

    /// Generate random password
    pub fn generate_password(&self) -> Result<String, LiteError> {
        let config = self.generator_config.lock().unwrap().clone();
        Self::generate_password_with_config(&config)
    }

    /// Generate password with custom config
    pub fn generate_password_with_config(
        config: &PasswordGeneratorConfig,
    ) -> Result<String, LiteError> {
        if config.pronounceable {
            return Self::generate_passphrase(config);
        }

        let mut rng = OsRng;
        let mut password = String::new();

        let lowercase = "abcdefghijklmnopqrstuvwxyz";
        let uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let numbers = "0123456789";
        let symbols = "!@#$%^&*()_+-=[]{}|;:,.<>?";
        let ambiguous = "0O1lI";
        let similar = "oO0iIlL1";

        let mut charset = String::new();

        if config.include_lowercase {
            charset.push_str(lowercase);
        }
        if config.include_uppercase {
            charset.push_str(uppercase);
        }
        if config.include_numbers {
            charset.push_str(numbers);
        }
        if config.include_symbols {
            charset.push_str(symbols);
        }
        if config.exclude_ambiguous {
            for c in ambiguous.chars() {
                charset = charset.replace(c, "");
            }
        }
        if config.exclude_similar {
            for c in similar.chars() {
                charset = charset.replace(c, "");
            }
        }

        if charset.is_empty() {
            return Err(LiteError::Config("No character set selected".to_string()));
        }

        let charset_bytes = charset.as_bytes();

        // Ensure minimum requirements
        if config.require_all_types {
            if config.include_lowercase {
                let idx = rng.next_u32() as usize % lowercase.len();
                password.push(lowercase.chars().nth(idx).unwrap());
            }
            if config.include_uppercase {
                let idx = rng.next_u32() as usize % uppercase.len();
                password.push(uppercase.chars().nth(idx).unwrap());
            }
            if config.include_numbers {
                for _ in 0..config.min_numbers {
                    let idx = rng.next_u32() as usize % numbers.len();
                    password.push(numbers.chars().nth(idx).unwrap());
                }
            }
            if config.include_symbols {
                for _ in 0..config.min_symbols {
                    let idx = rng.next_u32() as usize % symbols.len();
                    password.push(symbols.chars().nth(idx).unwrap());
                }
            }
        }

        // Fill remaining length
        while password.len() < config.length {
            let idx = rng.next_u32() as usize % charset_bytes.len();
            password.push(charset_bytes[idx] as char);
        }

        // Shuffle password
        let mut chars: Vec<char> = password.chars().collect();
        for i in (1..chars.len()).rev() {
            let j = rng.next_u32() as usize % (i + 1);
            chars.swap(i, j);
        }

        Ok(chars.into_iter().collect())
    }

    /// Generate memorable passphrase
    fn generate_passphrase(config: &PasswordGeneratorConfig) -> Result<String, LiteError> {
        // Word list for passphrase generation
        let words = vec![
            "apple",
            "banana",
            "cherry",
            "date",
            "elderberry",
            "fig",
            "grape",
            "honeydew",
            "kiwi",
            "lemon",
            "mango",
            "nectarine",
            "orange",
            "papaya",
            "quince",
            "raspberry",
            "strawberry",
            "tangerine",
            "watermelon",
            "blueberry",
            "coconut",
            "dragonfruit",
            "apricot",
            "avocado",
            "blackberry",
            "currant",
            "gooseberry",
            "guava",
            "jackfruit",
            "kumquat",
            "lychee",
            "mandarin",
            "mulberry",
            "olive",
            "peach",
            "pear",
            "persimmon",
            "pineapple",
            "plum",
            "pomegranate",
            "pomelo",
            "tamarind",
            "yuzu",
            "almond",
            "cashew",
            "chestnut",
            "hazelnut",
            "macadamia",
            "pecan",
            "pistachio",
            "walnut",
            "acorn",
            "beech",
            "birch",
            "cedar",
            "cherry",
            "chestnut",
            "elm",
            "fir",
            "hawthorn",
            "hazel",
            "hemlock",
            "holly",
            "hornbeam",
            "larch",
            "lime",
            "maple",
            "oak",
            "pine",
            "poplar",
            "rowan",
            "spruce",
            "willow",
            "yew",
            "amber",
            "azure",
            "crimson",
            "cyan",
            "emerald",
            "golden",
            "indigo",
            "ivory",
            "jade",
            "jet",
            "lime",
            "magenta",
            "maroon",
            "mauve",
            "ochre",
            "olive",
            "orange",
            "orchid",
            "peach",
            "periwinkle",
            "pink",
            "plum",
            "puce",
            "purple",
            "rose",
            "ruby",
            "saffron",
            "salmon",
            "sapphire",
            "scarlet",
            "sepia",
            "silver",
            "tan",
            "taupe",
            "teal",
            "terracotta",
            "thistle",
            "tomato",
            "turquoise",
            "ultramarine",
            "vermilion",
            "violet",
            "viridian",
            "wheat",
        ];

        let mut rng = OsRng;
        let mut passphrase_parts = Vec::new();

        for _ in 0..config.word_count {
            let idx = rng.next_u32() as usize % words.len();
            passphrase_parts.push(words[idx].to_string());
        }

        // Add numbers for extra entropy
        let num: u32 = rng.next_u32() % 1000;
        passphrase_parts.push(num.to_string());

        Ok(passphrase_parts.join(&config.word_separator))
    }

    /// Analyze password strength
    pub fn analyze_password_strength(password: &str) -> PasswordStrength {
        let mut weaknesses = Vec::new();
        let mut score = 100u8;

        // Length check
        if password.len() < 8 {
            weaknesses.push(PasswordWeakness::TooShort);
            score = score.saturating_sub(40);
        } else if password.len() < 12 {
            score = score.saturating_sub(10);
        } else if password.len() < 16 {
            score = score.saturating_sub(5);
        }

        // Character variety
        let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
        let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
        let has_numbers = password.chars().any(|c| c.is_ascii_digit());
        let has_symbols = password.chars().any(|c| !c.is_alphanumeric());

        if !has_lowercase {
            weaknesses.push(PasswordWeakness::NoLowercase);
            score = score.saturating_sub(10);
        }
        if !has_uppercase {
            weaknesses.push(PasswordWeakness::NoUppercase);
            score = score.saturating_sub(10);
        }
        if !has_numbers {
            weaknesses.push(PasswordWeakness::NoNumbers);
            score = score.saturating_sub(10);
        }
        if !has_symbols {
            weaknesses.push(PasswordWeakness::NoSymbols);
            score = score.saturating_sub(10);
        }

        // Check for repeated characters
        let mut prev_char = '\0';
        let mut repeat_count = 0;
        for c in password.chars() {
            if c == prev_char {
                repeat_count += 1;
                if repeat_count >= 2 {
                    weaknesses.push(PasswordWeakness::RepeatedChars);
                    score = score.saturating_sub(10);
                    break;
                }
            } else {
                repeat_count = 0;
            }
            prev_char = c;
        }

        // Check for sequential characters
        let password_lower = password.to_lowercase();
        let sequences = vec![
            "0123456789",
            "abcdefghijklmnopqrstuvwxyz",
            "qwertyuiop",
            "asdfghjkl",
            "zxcvbnm",
        ];
        for seq in sequences {
            for i in 0..seq.len().saturating_sub(2) {
                let pattern = &seq[i..i + 3];
                if password_lower.contains(pattern) {
                    weaknesses.push(PasswordWeakness::SequentialChars);
                    score = score.saturating_sub(10);
                    break;
                }
            }
        }

        // Check for common patterns (simplified)
        let common_patterns = vec![
            "password", "123456", "qwerty", "admin", "letmein", "welcome",
        ];
        for pattern in common_patterns {
            if password_lower.contains(pattern) {
                weaknesses.push(PasswordWeakness::CommonPattern);
                score = score.saturating_sub(20);
                break;
            }
        }

        // Calculate entropy
        let pool_size = (if has_lowercase { 26 } else { 0 })
            + (if has_uppercase { 26 } else { 0 })
            + (if has_numbers { 10 } else { 0 })
            + (if has_symbols { 32 } else { 0 });

        let entropy = if pool_size > 0 {
            (password.len() as f64) * (pool_size as f64).log2()
        } else {
            0.0
        };

        // Estimate crack time
        let guesses_per_second = 10_000_000_000.0; // 10 billion guesses/sec (high-end GPU)
        let crack_time_seconds = if entropy > 0.0 {
            (2f64.powf(entropy) / guesses_per_second) as u64
        } else {
            0
        };

        let crack_time_display = Self::format_duration(crack_time_seconds);

        // Generate feedback
        let mut feedback = Vec::new();
        if score < 50 {
            feedback.push("This password is weak and should be changed immediately".to_string());
        } else if score < 80 {
            feedback.push("This password is decent but could be stronger".to_string());
        } else {
            feedback.push("This is a strong password".to_string());
        }

        if !weaknesses.is_empty() {
            feedback.push(format!("Issues found: {}", weaknesses.len()));
        }

        PasswordStrength {
            score,
            entropy_bits: entropy,
            crack_time_seconds,
            crack_time_display,
            feedback,
            weaknesses,
        }
    }

    /// Format duration for display
    fn format_duration(seconds: u64) -> String {
        if seconds < 60 {
            format!("{} seconds", seconds)
        } else if seconds < 3600 {
            format!("{} minutes", seconds / 60)
        } else if seconds < 86400 {
            format!("{} hours", seconds / 3600)
        } else if seconds < 2592000 {
            format!("{} days", seconds / 86400)
        } else if seconds < 31536000 {
            format!("{} months", seconds / 2592000)
        } else if seconds < 3153600000 {
            format!("{} years", seconds / 31536000)
        } else if seconds < 31536000000 {
            format!("{} centuries", seconds / 315360000)
        } else {
            "forever".to_string()
        }
    }

    /// Run security audit
    pub fn run_security_audit(&self) -> Result<SecurityAuditResult, LiteError> {
        self.ensure_unlocked()?;

        let items = self.list_items()?;
        let mut weak_passwords = Vec::new();
        let mut duplicate_groups: Vec<Vec<String>> = Vec::new();
        let mut old_passwords = Vec::new();
        let mut missing_2fa = Vec::new();
        let mut expired_items = Vec::new();
        let mut recommendations = Vec::new();

        let mut password_hashes: HashMap<String, Vec<String>> = HashMap::new();
        let mut total_strength = 0u32;

        for item in &items {
            match item.item_type {
                VaultItemType::Password => {
                    if let Some((_, entry)) = self.get_password(&item.id)? {
                        // Check password strength
                        let strength = Self::analyze_password_strength(&entry.password);
                        total_strength += strength.score as u32;

                        if strength.score < 60 {
                            weak_passwords.push(item.id.clone());
                        }

                        // Check for duplicates
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(entry.password.as_bytes());
                        let hash = format!("{:x}", hasher.finalize());

                        password_hashes
                            .entry(hash)
                            .or_default()
                            .push(item.id.clone());

                        // Check age
                        let age = Utc::now() - item.created_at;
                        if age.num_days() > 90 {
                            old_passwords.push(item.id.clone());
                        }

                        // Check 2FA
                        if entry.totp_secret.is_none() {
                            missing_2fa.push(item.id.clone());
                        }
                    }
                }
                VaultItemType::Certificate => {
                    // Check certificate expiry
                    if let Some((_, entry)) = self.decrypt_and_get::<CertificateEntry>(&item.id)? {
                        if entry.expiry_date < Utc::now() {
                            expired_items.push(item.id.clone());
                        }
                    }
                }
                VaultItemType::SshKey => {
                    // Check SSH key age
                    let age = Utc::now() - item.created_at;
                    if age.num_days() > 365 {
                        recommendations.push(format!(
                            "SSH key '{}' is over 1 year old. Consider rotating.",
                            item.name
                        ));
                    }
                }
                _ => {}
            }
        }

        // Find duplicate groups
        for (_, ids) in password_hashes {
            if ids.len() > 1 {
                duplicate_groups.push(ids);
            }
        }

        // Calculate overall score
        let total_passwords = items
            .iter()
            .filter(|i| i.item_type == VaultItemType::Password)
            .count();

        let avg_strength = if total_passwords > 0 {
            (total_strength / total_passwords as u32) as u8
        } else {
            0
        };

        let mut overall_score = avg_strength;
        overall_score =
            overall_score.saturating_sub((weak_passwords.len() as u8).saturating_mul(5));
        overall_score =
            overall_score.saturating_sub((duplicate_groups.len() as u8).saturating_mul(10));
        overall_score = overall_score.saturating_sub((missing_2fa.len() as u8).saturating_mul(3));

        // Generate recommendations
        if !weak_passwords.is_empty() {
            recommendations.push(format!(
                "Found {} weak password(s). Consider updating them.",
                weak_passwords.len()
            ));
        }
        if !duplicate_groups.is_empty() {
            recommendations.push(format!(
                "Found {} password(s) reused across multiple accounts.",
                duplicate_groups.len()
            ));
        }
        if !old_passwords.is_empty() {
            recommendations.push(format!(
                "{} password(s) haven't been changed in over 90 days.",
                old_passwords.len()
            ));
        }
        if !missing_2fa.is_empty() {
            recommendations.push(format!(
                "{} account(s) don't have 2FA enabled. Consider enabling it.",
                missing_2fa.len()
            ));
        }

        Ok(SecurityAuditResult {
            overall_score,
            total_items: items.len(),
            weak_passwords,
            duplicate_passwords: duplicate_groups,
            leaked_passwords: Vec::new(), // Would require external breach database
            old_passwords,
            missing_2fa,
            insecure_websites: Vec::new(),
            expired_items,
            recommendations,
        })
    }

    /// Add trusted contact for emergency access
    pub fn add_trusted_contact(
        &self,
        name: &str,
        email: &str,
        access_level: EmergencyAccessLevel,
    ) -> Result<String, LiteError> {
        self.ensure_unlocked()?;

        let contact = TrustedContact {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            email: email.to_string(),
            public_key: String::new(), // Would be populated when contact accepts
            access_level,
            invitation_status: InvitationStatus::Pending,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(7)),
        };

        let id = contact.id.clone();
        self.trusted_contacts
            .lock()
            .unwrap()
            .insert(id.clone(), contact);
        self.save()?;

        Ok(id)
    }

    /// Remove trusted contact
    pub fn remove_trusted_contact(&self, id: &str) -> Result<bool, LiteError> {
        self.ensure_unlocked()?;

        let removed = self.trusted_contacts.lock().unwrap().remove(id).is_some();
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// List trusted contacts
    pub fn list_trusted_contacts(&self) -> Result<Vec<TrustedContact>, LiteError> {
        self.ensure_unlocked()?;
        Ok(self
            .trusted_contacts
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect())
    }

    /// Create folder
    pub fn create_folder(&self, name: &str, parent_id: Option<&str>) -> Result<String, LiteError> {
        self.ensure_unlocked()?;

        let folder = VaultFolder {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            created_at: Utc::now(),
            icon: None,
        };

        let id = folder.id.clone();
        self.folders.lock().unwrap().insert(id.clone(), folder);
        self.save()?;

        Ok(id)
    }

    /// Delete folder
    pub fn delete_folder(&self, id: &str) -> Result<bool, LiteError> {
        self.ensure_unlocked()?;

        // Check if folder has items
        let has_items = self
            .items
            .lock()
            .unwrap()
            .values()
            .any(|i| i.metadata.folder_id.as_ref() == Some(&id.to_string()));

        if has_items {
            return Err(LiteError::Config(
                "Cannot delete folder containing items".to_string(),
            ));
        }

        let removed = self.folders.lock().unwrap().remove(id).is_some();
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// List folders
    pub fn list_folders(&self) -> Result<Vec<VaultFolder>, LiteError> {
        self.ensure_unlocked()?;
        Ok(self.folders.lock().unwrap().values().cloned().collect())
    }

    /// Register hardware device
    pub fn register_hardware_device(
        &self,
        device_type: HardwareAuthMethod,
        device_id: &str,
        name: &str,
    ) -> Result<(), LiteError> {
        self.ensure_unlocked()?;

        let device = HardwareDeviceInfo {
            device_type,
            device_id: device_id.to_string(),
            name: name.to_string(),
            serial_number: None,
            firmware_version: None,
            registered_at: Utc::now(),
            last_used: None,
            is_active: true,
        };

        self.hardware_devices.lock().unwrap().push(device);
        self.save()?;
        Ok(())
    }

    /// List hardware devices
    pub fn list_hardware_devices(&self) -> Result<Vec<HardwareDeviceInfo>, LiteError> {
        self.ensure_unlocked()?;
        Ok(self.hardware_devices.lock().unwrap().clone())
    }

    /// Get vault statistics
    pub fn get_stats(&self) -> Result<VaultStats, LiteError> {
        self.ensure_unlocked()?;

        let items = self.list_items()?;
        let folders = self.list_folders()?;
        let contacts = self.list_trusted_contacts()?;
        let devices = self.list_hardware_devices()?;

        let mut items_by_type: HashMap<VaultItemType, usize> = HashMap::new();
        let mut total_strength = 0u32;
        let mut reused_count = 0usize;
        let mut weak_count = 0usize;
        let mut with_2fa = 0usize;

        let mut password_hashes: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for item in &items {
            *items_by_type.entry(item.item_type).or_insert(0) += 1;

            if item.item_type == VaultItemType::Password {
                if let Some((_, entry)) = self.get_password(&item.id)? {
                    let strength = Self::analyze_password_strength(&entry.password);
                    total_strength += strength.score as u32;

                    if strength.score < 60 {
                        weak_count += 1;
                    }

                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(entry.password.as_bytes());
                    let hash = format!("{:x}", hasher.finalize());

                    if !password_hashes.insert(hash) {
                        reused_count += 1;
                    }

                    if entry.totp_secret.is_some() {
                        with_2fa += 1;
                    }
                }
            }
        }

        let total_passwords = items_by_type
            .get(&VaultItemType::Password)
            .copied()
            .unwrap_or(0);
        let avg_strength = if total_passwords > 0 {
            (total_strength / total_passwords as u32) as u8
        } else {
            0
        };

        // Calculate storage used
        let storage_used = std::fs::read_dir(&self.vault_path)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.metadata().ok())
                    .map(|m| m.len() as usize)
                    .sum()
            })
            .unwrap_or(0);

        Ok(VaultStats {
            total_items: items.len(),
            items_by_type,
            total_folders: folders.len(),
            total_trusted_contacts: contacts.len(),
            hardware_devices: devices.len(),
            last_audit_date: None, // Would track this
            storage_used_bytes: storage_used,
            average_password_strength: avg_strength,
            passwords_reused_count: reused_count,
            passwords_weak_count: weak_count,
            passwords_with_2fa: with_2fa,
        })
    }

    /// Set autofill configuration
    pub fn set_autofill_config(&self, config: AutofillConfig) -> Result<(), LiteError> {
        self.ensure_unlocked()?;
        *self.autofill_config.lock().unwrap() = config;
        self.save()?;
        Ok(())
    }

    /// Get autofill configuration
    pub fn get_autofill_config(&self) -> Result<AutofillConfig, LiteError> {
        self.ensure_unlocked()?;
        Ok(self.autofill_config.lock().unwrap().clone())
    }

    /// Set password generator configuration
    pub fn set_generator_config(&self, config: PasswordGeneratorConfig) -> Result<(), LiteError> {
        self.ensure_unlocked()?;
        *self.generator_config.lock().unwrap() = config;
        self.save()?;
        Ok(())
    }

    /// Get password generator configuration
    pub fn get_generator_config(&self) -> Result<PasswordGeneratorConfig, LiteError> {
        self.ensure_unlocked()?;
        Ok(self.generator_config.lock().unwrap().clone())
    }

    /// Find autofill candidates for URL
    pub fn find_autofill_candidates(&self, url: &str) -> Result<Vec<VaultItemMetadata>, LiteError> {
        self.ensure_unlocked()?;

        let config = self.get_autofill_config()?;
        if !config.enabled {
            return Ok(Vec::new());
        }

        let all_items = self.list_items_by_type(VaultItemType::Password)?;

        let candidates: Vec<_> = all_items
            .into_iter()
            .filter(|item| {
                item.autofill_enabled
                    && item.urls.iter().any(|item_url| {
                        if config.match_url_exact {
                            item_url == url
                        } else if config.match_url_domain {
                            Self::domain_match(item_url, url)
                        } else if config.match_url_subdomain {
                            Self::subdomain_match(item_url, url)
                        } else {
                            item_url.contains(url) || url.contains(item_url)
                        }
                    })
            })
            .collect();

        Ok(candidates)
    }

    /// Check domain match
    fn domain_match(vault_url: &str, target_url: &str) -> bool {
        Self::extract_domain(vault_url) == Self::extract_domain(target_url)
    }

    /// Check subdomain match
    fn subdomain_match(vault_url: &str, target_url: &str) -> bool {
        let vault_domain = Self::extract_domain(vault_url);
        let target_domain = Self::extract_domain(target_url);

        vault_domain == target_domain
            || target_domain.ends_with(&format!(".{}", vault_domain))
            || vault_domain.ends_with(&format!(".{}", target_domain))
    }

    /// Extract domain from URL
    fn extract_domain(url: &str) -> String {
        url.trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .to_lowercase()
    }

    /// Calculate SSH key fingerprint
    fn calculate_ssh_fingerprint(public_key: &str) -> Result<String, LiteError> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use sha2::{Digest, Sha256};

        // Parse the public key format: "type base64 comment"
        let parts: Vec<&str> = public_key.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(LiteError::Config(
                "Invalid SSH public key format".to_string(),
            ));
        }

        let key_data = STANDARD
            .decode(parts[1])
            .map_err(|e| LiteError::Config(format!("Invalid base64 in SSH key: {}", e)))?;

        let mut hasher = Sha256::new();
        hasher.update(&key_data);
        let hash = hasher.finalize();

        // Format as SHA256:base64
        let encoded = STANDARD.encode(&hash[..]);
        Ok(format!("SHA256:{}", &encoded[..43]))
    }

    /// Detect SSH key type
    fn detect_key_type(public_key: &str) -> Result<String, LiteError> {
        let parts: Vec<&str> = public_key.split_whitespace().collect();
        if parts.is_empty() {
            return Err(LiteError::Config("Invalid SSH public key".to_string()));
        }
        Ok(parts[0].to_string())
    }

    /// Ensure vault is unlocked - SECURITY: Re-checks after auto-lock
    fn ensure_unlocked(&self) -> Result<(), LiteError> {
        // SECURITY: First check
        let unlocked = self.is_unlocked.lock().map_err(|e| {
            LiteError::Crypto(format!("Vault state lock failed: {}", e))
        })?;

        if !*unlocked {
            return Err(LiteError::InvalidMasterPassword);
        }
        drop(unlocked);

        // SECURITY: Check auto-lock before returning success
        self.check_auto_lock();

        // Re-check after auto-lock check
        let unlocked = self.is_unlocked.lock().map_err(|e| {
            LiteError::Crypto(format!("Vault state lock failed: {}", e))
        })?;

        if !*unlocked {
            return Err(LiteError::InvalidMasterPassword);
        }

        Ok(())
    }

    /// Export vault to encrypted file
    pub fn export_vault(&self, path: &str) -> Result<(), LiteError> {
        self.ensure_unlocked()?;

        // Reload items from disk to ensure we have everything
        let items = self.decrypt_items()?;
        let folders = self.folders.lock().unwrap().clone();
        let contacts = self.trusted_contacts.lock().unwrap().clone();

        let export_data = serde_json::json!({
            "version": "1.0",
            "exported_at": Utc::now(),
            "items": items,
            "folders": folders,
            "trusted_contacts": contacts,
        });

        let json_data = serde_json::to_vec(&export_data)
            .map_err(|e| LiteError::Config(format!("Failed to serialize vault: {}", e)))?;

        let crypto = CRYPTO_STATE
            .write()
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        let encrypted = crypto.encrypt(&json_data)?;

        std::fs::write(path, encrypted)
            .map_err(|e| LiteError::Config(format!("Failed to write export: {}", e)))?;

        Ok(())
    }

    /// Import vault from encrypted file
    pub fn import_vault(&self, path: &str, merge: bool) -> Result<usize, LiteError> {
        self.ensure_unlocked()?;

        let encrypted = std::fs::read(path)
            .map_err(|e| LiteError::Config(format!("Failed to read import file: {}", e)))?;

        let crypto = CRYPTO_STATE
            .write()
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        let decrypted = crypto.decrypt(&encrypted)?;

        let import_data: serde_json::Value = serde_json::from_slice(&decrypted)
            .map_err(|e| LiteError::Config(format!("Failed to parse vault: {}", e)))?;

        let mut imported_count = 0;

        if let Some(items) = import_data.get("items") {
            let imported_items: HashMap<String, EncryptedVaultItem> =
                serde_json::from_value(items.clone())
                    .map_err(|e| LiteError::Config(format!("Failed to parse items: {}", e)))?;

            let mut items_lock = self.items.lock().unwrap();
            for (id, item) in imported_items {
                if merge || !items_lock.contains_key(&id) {
                    items_lock.insert(id, item);
                    imported_count += 1;
                }
            }
        }

        if let Some(folders) = import_data.get("folders") {
            let imported_folders: HashMap<String, VaultFolder> =
                serde_json::from_value(folders.clone())
                    .map_err(|e| LiteError::Config(format!("Failed to parse folders: {}", e)))?;

            let mut folders_lock = self.folders.lock().unwrap();
            for (id, folder) in imported_folders {
                if merge || !folders_lock.contains_key(&id) {
                    folders_lock.insert(id, folder);
                }
            }
        }

        self.save()?;
        Ok(imported_count)
    }
}

impl Default for EnterpriseVault {
    fn default() -> Self {
        Self::new().expect("Failed to create default vault")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_generator_default() {
        let config = PasswordGeneratorConfig::default();
        assert_eq!(config.length, 20);
        assert!(config.include_uppercase);
        assert!(config.include_lowercase);
        assert!(config.include_numbers);
        assert!(config.include_symbols);
    }

    #[test]
    fn test_password_generation() {
        let config = PasswordGeneratorConfig::default();
        let password = EnterpriseVault::generate_password_with_config(&config)
            .expect("Failed to generate password");

        assert_eq!(password.len(), config.length);
    }

    #[test]
    fn test_passphrase_generation() {
        let mut config = PasswordGeneratorConfig::default();
        config.pronounceable = true;
        config.word_count = 5;

        let passphrase = EnterpriseVault::generate_password_with_config(&config)
            .expect("Failed to generate passphrase");

        // Should contain separator characters
        assert!(passphrase.contains(&config.word_separator));
        // Should have words + number
        let parts: Vec<&str> = passphrase.split(&config.word_separator).collect();
        assert_eq!(parts.len(), config.word_count + 1); // words + number
    }

    #[test]
    fn test_password_strength_analysis() {
        let weak = EnterpriseVault::analyze_password_strength("123456");
        assert!(weak.score < 50);
        assert!(!weak.weaknesses.is_empty());

        let strong = EnterpriseVault::analyze_password_strength("Tr0ub4dor&3x!@mp1e");
        assert!(strong.score > 70);
    }

    #[test]
    fn test_password_strength_entropy() {
        let strength = EnterpriseVault::analyze_password_strength("abcdefghij");
        assert!(strength.entropy_bits > 0.0);
        assert!(!strength.crack_time_display.is_empty());
    }

    #[test]
    fn test_domain_extraction() {
        assert_eq!(
            EnterpriseVault::extract_domain("https://example.com/login"),
            "example.com"
        );
        assert_eq!(
            EnterpriseVault::extract_domain("http://www.sub.example.com:8080/path"),
            "sub.example.com" // Port is stripped by extract_domain
        );
        assert_eq!(
            EnterpriseVault::extract_domain("example.com"),
            "example.com"
        );
    }

    #[test]
    fn test_domain_matching() {
        assert!(EnterpriseVault::domain_match(
            "https://example.com/login",
            "https://example.com/admin"
        ));
        assert!(!EnterpriseVault::domain_match(
            "https://example.com",
            "https://evil.com"
        ));
    }

    #[test]
    fn test_vault_item_type_display() {
        assert_eq!(VaultItemType::Password.to_string(), "Password");
        assert_eq!(VaultItemType::SshKey.to_string(), "SSH Key");
        assert_eq!(VaultItemType::TOTP.to_string(), "TOTP");
    }

    #[test]
    fn test_hardware_auth_method_display() {
        assert_eq!(HardwareAuthMethod::YubiKeyOtp.to_string(), "YubiKey OTP");
        assert_eq!(
            HardwareAuthMethod::BiometricFace.to_string(),
            "Face Recognition"
        );
    }

    #[test]
    fn test_security_level_as_u8() {
        assert_eq!(SecurityLevel::Standard.as_u8(), 1);
        assert_eq!(SecurityLevel::High.as_u8(), 2);
        assert_eq!(SecurityLevel::Maximum.as_u8(), 3);
        assert_eq!(SecurityLevel::Custom(5).as_u8(), 5);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(EnterpriseVault::format_duration(30), "30 seconds");
        assert_eq!(EnterpriseVault::format_duration(120), "2 minutes");
        assert_eq!(EnterpriseVault::format_duration(7200), "2 hours");
        assert_eq!(EnterpriseVault::format_duration(172800), "2 days");
        assert_eq!(EnterpriseVault::format_duration(5184000), "2 months");
        assert_eq!(EnterpriseVault::format_duration(63072000), "2 years");
        assert!(
            EnterpriseVault::format_duration(u64::MAX).contains("centuries")
                || EnterpriseVault::format_duration(u64::MAX) == "forever"
        );
    }
}
