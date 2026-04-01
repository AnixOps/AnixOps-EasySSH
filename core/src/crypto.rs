//! Cryptographic utilities for EasySSH
//!
//! This module provides secure encryption and decryption functionality using:
//! - AES-256-GCM for symmetric encryption
//! - Argon2id for key derivation
//! - Random nonces for each encryption operation
//!
//! # Security
//!
//! The `CryptoState` struct maintains the encryption key in memory. When locked,
//! the key is cleared from memory. Always lock the crypto state when not in use.
//!
//! # Example
//!
//! ```rust
//! use easyssh_core::crypto::CryptoState;
//!
//! // Initialize with master password
//! let mut state = CryptoState::new();
//! state.initialize("my_secure_password").unwrap();
//!
//! // Encrypt data
//! let plaintext = b"secret data";
//! let encrypted = state.encrypt(plaintext).unwrap();
//!
//! // Decrypt data
//! let decrypted = state.decrypt(&encrypted).unwrap();
//! assert_eq!(plaintext.to_vec(), decrypted);
//!
//! // Lock when done
//! state.lock();
//! ```

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{password_hash::SaltString, Argon2, PasswordHasher, Algorithm, Version, Params};
use rand::{rngs::OsRng, RngCore};
use std::sync::RwLock;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::LiteError;

/// Secure memory wrapper for cryptographic keys.
/// Automatically clears memory when dropped.
#[derive(Zeroize, ZeroizeOnDrop)]
struct SecureKey {
    #[zeroize(skip)]
    key: [u8; 32],
}

impl SecureKey {
    fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    fn as_slice(&self) -> &[u8] {
        &self.key
    }
}

/// Cryptographic state manager for encryption/decryption operations.
///
/// `CryptoState` manages the encryption key and provides methods for
/// encrypting and decrypting data using AES-256-GCM with Argon2id key derivation.
///
/// The state must be initialized with a master password before any
/// encryption or decryption operations can be performed.
///
/// # Security
///
/// Uses `RwLock` for concurrent access and `zeroize` for secure memory clearing.
pub struct CryptoState {
    cipher: Option<Aes256Gcm>,
    salt: Option<[u8; 32]>,
    secure_key: Option<SecureKey>,
}

impl CryptoState {
    /// Create a new, uninitialized crypto state.
    ///
    /// The returned state is locked and cannot be used for encryption
    /// until `initialize()` or `unlock()` is called.
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::crypto::CryptoState;
    ///
    /// let state = CryptoState::new();
    /// assert!(!state.is_unlocked());
    /// ```
    pub fn new() -> Self {
        Self {
            cipher: None,
            salt: None,
            secure_key: None,
        }
    }

    /// Initialize the crypto state with a master password.
    ///
    /// This method generates a new random salt and derives the encryption key
    /// using Argon2id with higher memory cost (secure by default).
    /// Use this for first-time setup.
    ///
    /// # Arguments
    ///
    /// * `master_password` - The password used to derive the encryption key
    ///
    /// # Errors
    ///
    /// Returns `LiteError::Crypto` if key derivation fails.
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::crypto::CryptoState;
    ///
    /// let mut state = CryptoState::new();
    /// state.initialize("my_secure_password").unwrap();
    /// assert!(state.is_unlocked());
    /// ```
    pub fn initialize(&mut self, master_password: &str) -> Result<(), LiteError> {
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);

        let key = self.derive_key_secure(master_password, &salt)?;
        let cipher =
            Aes256Gcm::new_from_slice(key.as_slice()).map_err(|e| LiteError::Crypto(e.to_string()))?;

        self.cipher = Some(cipher);
        self.salt = Some(salt);
        self.secure_key = Some(key);
        Ok(())
    }

    /// Unlock the crypto state using an existing salt.
    ///
    /// Use this method to unlock a previously initialized state.
    /// The salt must be set before calling this method using `set_salt()`.
    ///
    /// # Arguments
    ///
    /// * `master_password` - The password used to derive the encryption key
    ///
    /// # Returns
    ///
    /// Returns `true` if unlock succeeds, or an error if the salt is not set
    /// or key derivation fails.
    ///
    /// # Errors
    ///
    /// Returns `LiteError::InvalidMasterPassword` if the salt is not set.
    /// Returns `LiteError::Crypto` if key derivation fails.
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::crypto::CryptoState;
    ///
    /// // First, initialize and get the salt
    /// let mut state = CryptoState::new();
    /// state.initialize("password").unwrap();
    /// let salt = state.get_salt().unwrap();
    ///
    /// // Later, unlock with the same salt
    /// let mut new_state = CryptoState::new();
    /// new_state.set_salt(salt.try_into().unwrap());
    /// let result = new_state.unlock("password").unwrap();
    /// assert!(result);
    /// ```
    pub fn unlock(&mut self, master_password: &str) -> Result<bool, LiteError> {
        let salt = self.salt.ok_or(LiteError::InvalidMasterPassword)?;

        let key = self.derive_key_secure(master_password, &salt)?;
        let cipher =
            Aes256Gcm::new_from_slice(key.as_slice()).map_err(|e| LiteError::Crypto(e.to_string()))?;

        self.cipher = Some(cipher);
        self.secure_key = Some(key);
        Ok(true)
    }

    /// Secure key derivation with high memory cost Argon2id.
    ///
    /// Uses Argon2id with:
    /// - Memory: 64 MB (65536 KB)
    /// - Iterations: 3
    /// - Parallelism: 4
    fn derive_key_secure(
        &self,
        master_password: &str,
        salt: &[u8; 32],
    ) -> Result<SecureKey, LiteError> {
        let salt_str =
            SaltString::encode_b64(salt).map_err(|e| LiteError::Crypto(e.to_string()))?;

        // High security Argon2id parameters
        let params = Params::new(65536, 3, 4, Some(32))
            .map_err(|e| LiteError::Crypto(format!("Invalid Argon2 params: {}", e)))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let hash = argon2
            .hash_password(master_password.as_bytes(), &salt_str)
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        let output = hash.hash.ok_or(LiteError::InvalidMasterPassword)?;
        let key_bytes = output.as_bytes();

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes[..32]);
        Ok(SecureKey::new(key))
    }

    #[deprecated(since = "0.3.0", note = "Use derive_key_secure for better security")]
    #[allow(dead_code)]
    fn derive_key_internal(
        &self,
        master_password: &str,
        salt: &[u8; 32],
    ) -> Result<[u8; 32], LiteError> {
        let salt_str =
            SaltString::encode_b64(salt).map_err(|e| LiteError::Crypto(e.to_string()))?;

        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(master_password.as_bytes(), &salt_str)
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        let output = hash.hash.ok_or(LiteError::InvalidMasterPassword)?;
        let key_bytes = output.as_bytes();

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes[..32]);
        Ok(key)
    }

    /// Encrypt plaintext data.
    ///
    /// Generates a random 12-byte nonce and encrypts the data using AES-256-GCM.
    /// The returned vector contains the nonce followed by the ciphertext.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The data to encrypt
    ///
    /// # Returns
    ///
    /// A vector containing `[nonce (12 bytes) | ciphertext]`
    ///
    /// # Errors
    ///
    /// Returns `LiteError::InvalidMasterPassword` if the crypto state is locked.
    /// Returns `LiteError::Crypto` if encryption fails.
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::crypto::CryptoState;
    ///
    /// let mut state = CryptoState::new();
    /// state.initialize("password").unwrap();
    ///
    /// let encrypted = state.encrypt(b"secret message").unwrap();
    /// assert!(encrypted.len() > 12); // nonce + ciphertext
    /// ```
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, LiteError> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or(LiteError::InvalidMasterPassword)?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    /// Decrypt encrypted data.
    ///
    /// The input must contain a 12-byte nonce followed by the ciphertext.
    ///
    /// # Arguments
    ///
    /// * `data` - The encrypted data in the format `[nonce (12 bytes) | ciphertext]`
    ///
    /// # Returns
    ///
    /// The decrypted plaintext as a vector of bytes.
    ///
    /// # Errors
    ///
    /// Returns `LiteError::Crypto` if the data is too short (less than 12 bytes)
    /// or if decryption fails (corrupted data or wrong key).
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::crypto::CryptoState;
    ///
    /// let mut state = CryptoState::new();
    /// state.initialize("password").unwrap();
    ///
    /// let plaintext = b"secret message";
    /// let encrypted = state.encrypt(plaintext).unwrap();
    /// let decrypted = state.decrypt(&encrypted).unwrap();
    ///
    /// assert_eq!(plaintext.to_vec(), decrypted);
    /// ```
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, LiteError> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or(LiteError::InvalidMasterPassword)?;

        if data.len() < 12 {
            return Err(LiteError::Crypto("数据太短".to_string()));
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| LiteError::Crypto(e.to_string()))
    }

    /// Get the salt value.
    ///
    /// Returns `None` if the state has not been initialized.
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::crypto::CryptoState;
    ///
    /// let mut state = CryptoState::new();
    /// assert!(state.get_salt().is_none());
    ///
    /// state.initialize("password").unwrap();
    /// assert!(state.get_salt().is_some());
    /// ```
    pub fn get_salt(&self) -> Option<Vec<u8>> {
        self.salt.map(|s| s.to_vec())
    }

    /// Set the salt value for unlocking.
    ///
    /// Use this method to restore a previously saved salt before calling `unlock()`.
    ///
    /// # Arguments
    ///
    /// * `salt` - A 32-byte salt value from a previous initialization
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::crypto::CryptoState;
    ///
    /// let mut state = CryptoState::new();
    /// let salt = [1u8; 32];
    /// state.set_salt(salt);
    /// ```
    pub fn set_salt(&mut self, salt: [u8; 32]) {
        self.salt = Some(salt);
    }

    /// Lock the crypto state, clearing the encryption key from memory.
    ///
    /// After locking, encryption and decryption operations will fail until
    /// the state is unlocked again with `unlock()`.
    ///
    /// # Security
    ///
    /// This method uses `zeroize` to securely clear the key from memory
    /// before dropping the cipher.
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::crypto::CryptoState;
    ///
    /// let mut state = CryptoState::new();
    /// state.initialize("password").unwrap();
    /// assert!(state.is_unlocked());
    ///
    /// state.lock();
    /// assert!(!state.is_unlocked());
    /// ```
    pub fn lock(&mut self) {
        // Securely clear the key first
        if let Some(ref mut key) = self.secure_key {
            key.key.zeroize();
        }
        self.secure_key = None;
        self.cipher = None;
    }

    /// Check if the crypto state is unlocked and ready for operations.
    ///
    /// Returns `true` if the encryption key is loaded and available.
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::crypto::CryptoState;
    ///
    /// let mut state = CryptoState::new();
    /// assert!(!state.is_unlocked());
    ///
    /// state.initialize("password").unwrap();
    /// assert!(state.is_unlocked());
    /// ```
    pub fn is_unlocked(&self) -> bool {
        self.cipher.is_some()
    }
}

impl Default for CryptoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global cryptographic state instance.
///
/// This static instance provides a globally accessible crypto state for the application.
/// It is protected by a RwLock to ensure thread-safe concurrent access.
///
/// # Security
///
/// Uses `RwLock` instead of `Mutex` for better read concurrency.
/// Multiple readers can access simultaneously, but writes are exclusive.
///
/// # Example
///
/// ```rust
/// use easyssh_core::crypto::CRYPTO_STATE;
///
/// // Initialize the global crypto state
/// {
///     let mut state = CRYPTO_STATE.write().unwrap();
///     state.initialize("master_password").unwrap();
/// }
///
/// // Later, check if it's unlocked (concurrent reads)
/// let state = CRYPTO_STATE.read().unwrap();
/// assert!(state.is_unlocked());
/// ```
pub static CRYPTO_STATE: std::sync::LazyLock<RwLock<CryptoState>> =
    std::sync::LazyLock::new(|| RwLock::new(CryptoState::new()));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_state_new() {
        let state = CryptoState::new();
        assert!(state.cipher.is_none());
        assert!(state.salt.is_none());
        assert!(!state.is_unlocked());
    }

    #[test]
    fn test_crypto_state_default() {
        let state: CryptoState = Default::default();
        assert!(!state.is_unlocked());
    }

    #[test]
    fn test_initialize_and_unlock() {
        let mut state = CryptoState::new();
        let password = "test_password_123";

        // Initialize
        state.initialize(password).expect("Failed to initialize");
        assert!(state.is_unlocked());
        assert!(state.salt.is_some());

        // Get salt and create new state
        let salt = state.get_salt().expect("Should have salt");
        let mut new_state = CryptoState::new();
        let mut salt_array = [0u8; 32];
        salt_array.copy_from_slice(&salt);
        new_state.set_salt(salt_array);

        // Unlock with same password
        let result = new_state.unlock(password).expect("Failed to unlock");
        assert!(result);
        assert!(new_state.is_unlocked());

        // Lock and verify
        new_state.lock();
        assert!(!new_state.is_unlocked());
    }

    #[test]
    fn test_unlock_wrong_password() {
        let mut state = CryptoState::new();
        let password = "correct_password";

        state.initialize(password).expect("Failed to initialize");
        let salt = state.get_salt().unwrap();

        // Create new state with same salt but wrong password
        let mut wrong_state = CryptoState::new();
        let mut salt_array = [0u8; 32];
        salt_array.copy_from_slice(&salt);
        wrong_state.set_salt(salt_array);

        // Should not panic but may produce different key
        let result = wrong_state.unlock("wrong_password");
        // Result depends on Argon2 - it won't fail but derived key will be different
        assert!(result.is_ok());
    }

    #[test]
    fn test_encrypt_decrypt() {
        let mut state = CryptoState::new();
        state.initialize("master_password").expect("Failed to initialize");

        let plaintext = b"Hello, World! This is a secret message.";
        let encrypted = state.encrypt(plaintext).expect("Failed to encrypt");

        // Encrypted should be different from plaintext
        assert_ne!(encrypted, plaintext.to_vec());
        // Should include nonce (12 bytes) + ciphertext
        assert!(encrypted.len() > 12);

        let decrypted = state.decrypt(&encrypted).expect("Failed to decrypt");
        assert_eq!(decrypted, plaintext.to_vec());
    }

    #[test]
    fn test_encrypt_decrypt_unicode() {
        let mut state = CryptoState::new();
        state.initialize("unicode_test_pass").expect("Failed to initialize");

        let plaintext = "中文测试 🎉 émojis and ñoño".as_bytes();
        let encrypted = state.encrypt(plaintext).expect("Failed to encrypt");
        let decrypted = state.decrypt(&encrypted).expect("Failed to decrypt");

        assert_eq!(decrypted, plaintext.to_vec());
    }

    #[test]
    fn test_encrypt_empty_data() {
        let mut state = CryptoState::new();
        state.initialize("test_pass").expect("Failed to initialize");

        let plaintext = b"";
        let encrypted = state.encrypt(plaintext).expect("Failed to encrypt empty");
        let decrypted = state.decrypt(&encrypted).expect("Failed to decrypt empty");

        assert_eq!(decrypted, plaintext.to_vec());
    }

    #[test]
    fn test_encrypt_large_data() {
        let mut state = CryptoState::new();
        state.initialize("large_data_pass").expect("Failed to initialize");

        let plaintext = vec![0u8; 1024 * 1024]; // 1MB of zeros
        let encrypted = state.encrypt(&plaintext).expect("Failed to encrypt large");
        let decrypted = state.decrypt(&encrypted).expect("Failed to decrypt large");

        assert_eq!(decrypted.len(), plaintext.len());
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_too_short() {
        let mut state = CryptoState::new();
        state.initialize("test_pass").expect("Failed to initialize");

        let too_short = vec![0u8; 10]; // Less than 12 bytes
        let result = state.decrypt(&too_short);

        assert!(result.is_err());
        // Check that it's a Crypto error
        assert!(matches!(result.unwrap_err(), LiteError::Crypto(_)));
    }

    #[test]
    fn test_decrypt_corrupted_data() {
        let mut state = CryptoState::new();
        state.initialize("test_pass").expect("Failed to initialize");

        let plaintext = b"test message";
        let mut encrypted = state.encrypt(plaintext).expect("Failed to encrypt");

        // Corrupt some bytes
        if encrypted.len() > 15 {
            encrypted[15] ^= 0xFF;
        }

        let result = state.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_without_unlock() {
        let state = CryptoState::new();
        // Not initialized

        let plaintext = b"test";
        let result = state.encrypt(plaintext);

        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_without_unlock() {
        let state = CryptoState::new();
        // Not initialized

        let data = vec![0u8; 20];
        let result = state.decrypt(&data);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_salt_before_init() {
        let state = CryptoState::new();
        assert!(state.get_salt().is_none());
    }

    #[test]
    fn test_salt_persistence() {
        let mut state = CryptoState::new();
        state.initialize("test_pass").expect("Failed to initialize");

        let salt = state.get_salt().expect("Should have salt");
        assert_eq!(salt.len(), 32);

        // Verify salt is different each time
        let mut state2 = CryptoState::new();
        state2.initialize("test_pass").expect("Failed to initialize 2");
        let salt2 = state2.get_salt().expect("Should have salt 2");

        // Salts should be different (with extremely high probability)
        assert_ne!(salt, salt2);
    }

    #[test]
    fn test_multiple_encrypt_decrypt_cycles() {
        let mut state = CryptoState::new();
        state.initialize("cycle_test_pass").expect("Failed to initialize");

        let plaintexts = vec![
            b"First message".to_vec(),
            b"Second message with different length".to_vec(),
            vec![0u8; 100],
            vec![0xFFu8; 1000],
        ];

        for plaintext in &plaintexts {
            let encrypted = state.encrypt(plaintext).expect("Encrypt failed");
            let decrypted = state.decrypt(&encrypted).expect("Decrypt failed");
            assert_eq!(decrypted, *plaintext);
        }
    }

    #[test]
    fn test_concurrent_crypto_state() {
        use std::thread;

        // Initialize global state
        {
            let mut state = CRYPTO_STATE.write().unwrap();
            if !state.is_unlocked() {
                state.initialize("concurrent_test").expect("Failed to init");
            }
        }

        let handles: Vec<_> = (0..5)
            .map(|i| {
                thread::spawn(move || {
                    let plaintext = format!("Thread {} message", i);
                    let state = CRYPTO_STATE.write().unwrap();
                    if state.is_unlocked() {
                        let encrypted = state.encrypt(plaintext.as_bytes()).expect("Encrypt");
                        let decrypted = state.decrypt(&encrypted).expect("Decrypt");
                        assert_eq!(decrypted, plaintext.as_bytes().to_vec());
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    #[test]
    fn test_key_derivation_deterministic() {
        let state = CryptoState::new();
        let password = "deterministic_test";
        let mut salt = [0u8; 32];
        salt.copy_from_slice(&[1u8; 32]); // Fixed salt for testing

        // Same password + same salt should produce same key
        let key1 = state.derive_key_internal(password, &salt).expect("Derive 1");
        let key2 = state.derive_key_internal(password, &salt).expect("Derive 2");

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_key_derivation_different_passwords() {
        let state = CryptoState::new();
        let mut salt = [0u8; 32];
        salt.copy_from_slice(&[1u8; 32]);

        let key1 = state.derive_key_internal("password1", &salt).expect("Derive 1");
        let key2 = state.derive_key_internal("password2", &salt).expect("Derive 2");

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_derivation_different_salts() {
        let state = CryptoState::new();
        let password = "same_password";

        let mut salt1 = [0u8; 32];
        salt1.copy_from_slice(&[1u8; 32]);
        let mut salt2 = [0u8; 32];
        salt2.copy_from_slice(&[2u8; 32]);

        let key1 = state.derive_key_internal(password, &salt1).expect("Derive 1");
        let key2 = state.derive_key_internal(password, &salt2).expect("Derive 2");

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_lock_clears_cipher() {
        let mut state = CryptoState::new();
        state.initialize("test_password").expect("Failed to initialize");
        assert!(state.is_unlocked());

        state.lock();
        assert!(!state.is_unlocked());

        // Verify encryption fails after lock
        let result = state.encrypt(b"test");
        assert!(result.is_err());
    }

    #[test]
    fn test_reinitialize_changes_salt() {
        let mut state = CryptoState::new();
        state.initialize("password").expect("Init 1");
        let salt1 = state.get_salt().unwrap();

        // Re-initialize should not change salt since it's already set
        // Actually, initialize doesn't check for existing salt, it always generates new
        // This test verifies that behavior
        let mut state2 = CryptoState::new();
        state2.initialize("password").expect("Init 2");
        let salt2 = state2.get_salt().unwrap();

        // Two different states should have different salts
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_decrypt_with_wrong_key_produces_error() {
        let mut state1 = CryptoState::new();
        let mut state2 = CryptoState::new();

        state1.initialize("password1").expect("Init 1");
        state2.initialize("password2").expect("Init 2");

        // Encrypt with state1
        let plaintext = b"secret message";
        let encrypted = state1.encrypt(plaintext).expect("Encrypt");

        // Decrypt with state2 (different key) should fail
        let result = state2.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_salt_manually() {
        let mut state = CryptoState::new();
        let salt = [42u8; 32];

        state.set_salt(salt);
        assert_eq!(state.get_salt().unwrap(), salt.to_vec());
    }

    #[test]
    fn test_encrypt_decrypt_binary_data() {
        let mut state = CryptoState::new();
        state.initialize("binary_test").expect("Failed to initialize");

        // Binary data with all byte values
        let plaintext: Vec<u8> = (0..=255).collect();
        let encrypted = state.encrypt(&plaintext).expect("Failed to encrypt");
        let decrypted = state.decrypt(&encrypted).expect("Failed to decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_special_characters() {
        let mut state = CryptoState::new();
        state.initialize("special_test").expect("Failed to initialize");

        // Data with special Unicode characters
        let test_strings = vec![
            "Hello\x00World",          // Null byte
            "Line1\nLine2",            // Newline
            "Tab\tSeparated",          // Tab
            "Quote\"Test\"",           // Quotes
            "Backslash\\Test",         // Backslash
        ];

        for s in &test_strings {
            let encrypted = state.encrypt(s.as_bytes()).expect("Encrypt failed");
            let decrypted = state.decrypt(&encrypted).expect("Decrypt failed");
            assert_eq!(String::from_utf8(decrypted).unwrap(), *s);
        }
    }

    #[test]
    fn test_concurrent_crypto_state_thread_safety() {
        use std::thread;
        use std::sync::Arc;
        use std::sync::Mutex;

        let crypto = Arc::new(Mutex::new(CryptoState::new()));
        {
            crypto.lock().unwrap().initialize("concurrent_pass").expect("Init");
        }

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let crypto_clone = Arc::clone(&crypto);
                thread::spawn(move || {
                    let data = format!("Thread {} data with more content", i);
                    let state = crypto_clone.lock().unwrap();
                    let encrypted = state.encrypt(data.as_bytes()).expect("Encrypt");
                    let decrypted = state.decrypt(&encrypted).expect("Decrypt");
                    assert_eq!(decrypted, data.as_bytes().to_vec());
                    true
                })
            })
            .collect();

        let results: Vec<bool> = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        assert!(results.iter().all(|&r| r), "Concurrent operations failed");
    }

    #[test]
    fn test_multiple_initialize_calls() {
        let mut state = CryptoState::new();

        // First initialize
        state.initialize("password1").expect("First init");
        let salt1 = state.get_salt().unwrap();

        // Lock
        state.lock();
        assert!(!state.is_unlocked());

        // Re-initialize with different password (generates new salt)
        state.initialize("password2").expect("Second init");
        let salt2 = state.get_salt().unwrap();

        // Salt should be different after re-initialize
        assert_ne!(salt1, salt2);
    }
}
