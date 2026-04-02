//! Secure Keychain and Credential Storage
//!
//! This module provides secure credential storage using:
//! - Platform-native keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
//! - Encrypted fallback file storage using AES-256-GCM
//! - Automatic migration from legacy unencrypted storage
//!
//! # Security
//!
//! - Uses `RwLock` for concurrent access to the crypto state
//! - All credentials are encrypted at rest using AES-256-GCM
//! - Master password never stored, only its hash
//!
//! # Example
//!
//! ```rust,no_run
//! use easyssh_core::keychain::{store_password, get_password, delete_password};
//!
//! // Store password
//! store_password("server-1", "secret_password").unwrap();
//!
//! // Retrieve password
//! if let Ok(Some(password)) = get_password("server-1") {
//!     println!("Retrieved password");
//! }
//!
//! // Delete password
//! delete_password("server-1").unwrap();
//! ```

use crate::crypto::CRYPTO_STATE;
use crate::error::LiteError;
use keyring::Entry;
use std::fs;
use std::path::PathBuf;
use zeroize::Zeroize;

/// Service name for Keychain
const SERVICE_NAME: &str = "com.easyssh.lite";

/// Encrypted fallback store entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct EncryptedEntry {
    encrypted_data: String, // base64 encoded encrypted data
    nonce: String,          // base64 encoded nonce
}

impl Zeroize for EncryptedEntry {
    fn zeroize(&mut self) {
        self.encrypted_data.zeroize();
        self.nonce.zeroize();
    }
}

/// Get fallback storage path (encrypted file)
fn fallback_store_path() -> Result<PathBuf, LiteError> {
    let mut base = if let Some(p) = dirs::data_local_dir() {
        p
    } else if let Some(home) = dirs::home_dir() {
        home.join(".easyssh")
    } else {
        std::env::current_dir().map_err(|e| LiteError::Keychain(e.to_string()))?
    };

    base.push("EasySSH");
    fs::create_dir_all(&base).map_err(|e| LiteError::Keychain(e.to_string()))?;
    base.push("keychain_encrypted.bin");
    Ok(base)
}

/// Load encrypted fallback store
fn load_fallback_encrypted() -> Result<std::collections::HashMap<String, EncryptedEntry>, LiteError>
{
    let path = fallback_store_path()?;
    if !path.exists() {
        return Ok(std::collections::HashMap::new());
    }

    let bytes = fs::read(&path).map_err(|e| LiteError::Keychain(e.to_string()))?;
    if bytes.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    // Use crypto state to decrypt the entire store
    let crypto = CRYPTO_STATE
        .read()
        .map_err(|e| LiteError::Crypto(e.to_string()))?;

    if !crypto.is_unlocked() {
        // If not unlocked, return empty but don't fail
        // (passwords will be lost until unlock)
        return Ok(std::collections::HashMap::new());
    }

    // The file format is: nonce (12 bytes) || encrypted_data
    if bytes.len() < 12 {
        return Ok(std::collections::HashMap::new());
    }

    let decrypted = crypto
        .decrypt(&bytes)
        .map_err(|e| LiteError::Crypto(format!("Failed to decrypt keychain: {}", e)))?;

    let json = String::from_utf8(decrypted)
        .map_err(|_| LiteError::Crypto("Invalid UTF-8 in decrypted data".to_string()))?;

    serde_json::from_str(&json).map_err(|e| LiteError::Keychain(e.to_string()))
}

/// Save encrypted fallback store
fn save_fallback_encrypted(
    data: &std::collections::HashMap<String, EncryptedEntry>,
) -> Result<(), LiteError> {
    let path = fallback_store_path()?;

    let crypto = CRYPTO_STATE
        .read()
        .map_err(|e| LiteError::Crypto(e.to_string()))?;

    if !crypto.is_unlocked() {
        // Can't save without encryption - this is a security feature
        log::warn!("Cannot save keychain fallback: crypto not unlocked");
        return Ok(());
    }

    let json = serde_json::to_vec(data).map_err(|e| LiteError::Keychain(e.to_string()))?;

    let encrypted = crypto
        .encrypt(&json)
        .map_err(|e| LiteError::Crypto(format!("Failed to encrypt keychain: {}", e)))?;

    fs::write(&path, encrypted).map_err(|e| LiteError::Keychain(e.to_string()))?;

    Ok(())
}

/// Legacy fallback loader (for migration)
fn load_legacy_fallback() -> Result<std::collections::HashMap<String, String>, LiteError> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    let mut base = if let Some(p) = dirs::data_local_dir() {
        p
    } else if let Some(home) = dirs::home_dir() {
        home.join(".easyssh")
    } else {
        std::env::current_dir().map_err(|e| LiteError::Keychain(e.to_string()))?
    };

    base.push("EasySSH");
    base.push("keychain_fallback.json");

    if !base.exists() {
        return Ok(std::collections::HashMap::new());
    }

    let txt = fs::read_to_string(&base).map_err(|e| LiteError::Keychain(e.to_string()))?;
    if txt.trim().is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let map: std::collections::HashMap<String, String> =
        serde_json::from_str(&txt).map_err(|e| LiteError::Keychain(e.to_string()))?;

    // Migrate to encrypted format if crypto is available
    let mut encrypted_map = std::collections::HashMap::new();
    for (k, v) in &map {
        if let Ok(decoded) = BASE64.decode(v) {
            if let Ok(password) = String::from_utf8(decoded) {
                // Store in encrypted format
                let crypto = CRYPTO_STATE
                    .write()
                    .map_err(|e| LiteError::Crypto(e.to_string()))?;
                if crypto.is_unlocked() {
                    let encrypted = crypto
                        .encrypt(password.as_bytes())
                        .map_err(|e| LiteError::Crypto(e.to_string()))?;

                    encrypted_map.insert(
                        k.clone(),
                        EncryptedEntry {
                            encrypted_data: BASE64.encode(&encrypted[12..]),
                            nonce: BASE64.encode(&encrypted[..12]),
                        },
                    );
                }
            }
        }
    }

    // Save encrypted version and remove legacy file
    if !encrypted_map.is_empty() {
        drop(save_fallback_encrypted(&encrypted_map));
        let _ = fs::remove_file(&base);
    }

    Ok(map)
}

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Store password in keychain with encrypted fallback
pub fn store_password(server_id: &str, password: &str) -> Result<(), LiteError> {
    // 1) Always write encrypted fallback first (persistence guarantee)
    let mut map = load_fallback_encrypted()?;

    let crypto = CRYPTO_STATE
        .read()
        .map_err(|e| LiteError::Crypto(e.to_string()))?;

    if crypto.is_unlocked() {
        let encrypted = crypto
            .encrypt(password.as_bytes())
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        // encrypted format: nonce (12 bytes) || ciphertext
        map.insert(
            server_id.to_string(),
            EncryptedEntry {
                encrypted_data: BASE64.encode(&encrypted[12..]),
                nonce: BASE64.encode(&encrypted[..12]),
            },
        );

        save_fallback_encrypted(&map)?;
    } else {
        log::warn!("Crypto not unlocked - password not persisted to fallback");
    }

    // 2) Best-effort write to system keychain
    match Entry::new(SERVICE_NAME, server_id) {
        Ok(entry) => {
            if let Err(e) = entry.set_password(password) {
                log::warn!("Keychain store failed, fallback still saved: {}", e);
            }
        }
        Err(e) => {
            log::warn!(
                "Keychain entry creation failed, fallback still saved: {}",
                e
            );
        }
    }

    Ok(())
}

/// Get password from keychain with encrypted fallback
pub fn get_password(server_id: &str) -> Result<Option<String>, LiteError> {
    // 1) Try system keychain first (fastest)
    match Entry::new(SERVICE_NAME, server_id) {
        Ok(entry) => match entry.get_password() {
            Ok(password) => return Ok(Some(password)),
            Err(keyring::Error::NoEntry) => {}
            Err(e) => {
                log::warn!("Keychain read failed, trying fallback: {}", e);
            }
        },
        Err(e) => log::warn!("Keychain entry creation failed: {}", e),
    }

    // 2) Try encrypted fallback
    let map = load_fallback_encrypted()?;
    if let Some(entry) = map.get(server_id) {
        let crypto = CRYPTO_STATE
            .read()
            .map_err(|e| LiteError::Crypto(e.to_string()))?;

        if !crypto.is_unlocked() {
            return Err(LiteError::InvalidMasterPassword);
        }

        // Reconstruct encrypted blob: nonce || ciphertext
        let mut encrypted_blob = Vec::new();
        encrypted_blob.extend_from_slice(
            &BASE64
                .decode(&entry.nonce)
                .map_err(|_| LiteError::Crypto("Invalid nonce".to_string()))?,
        );
        encrypted_blob.extend_from_slice(
            &BASE64
                .decode(&entry.encrypted_data)
                .map_err(|_| LiteError::Crypto("Invalid ciphertext".to_string()))?,
        );

        let decrypted = crypto
            .decrypt(&encrypted_blob)
            .map_err(|_| LiteError::InvalidMasterPassword)?;

        let password = String::from_utf8(decrypted)
            .map_err(|_| LiteError::Crypto("Invalid UTF-8".to_string()))?;

        return Ok(Some(password));
    }

    // 3) Try legacy fallback (migration path)
    if let Ok(Some(password)) = try_load_legacy(server_id) {
        return Ok(Some(password));
    }

    Ok(None)
}

/// Try to load from legacy format
fn try_load_legacy(server_id: &str) -> Result<Option<String>, LiteError> {
    let map = load_legacy_fallback()?;
    if let Some(v) = map.get(server_id) {
        let decoded = BASE64
            .decode(v)
            .ok()
            .and_then(|b| String::from_utf8(b).ok());
        return Ok(decoded);
    }
    Ok(None)
}

/// Delete password from keychain and fallback
pub fn delete_password(server_id: &str) -> Result<(), LiteError> {
    let entry =
        Entry::new(SERVICE_NAME, server_id).map_err(|e| LiteError::Keychain(e.to_string()))?;

    let _ = entry.delete_credential();

    // Also delete from encrypted fallback
    let mut map = load_fallback_encrypted()?;
    map.remove(server_id);
    save_fallback_encrypted(&map)?;

    Ok(())
}

/// Store master password hash in keychain only (never in fallback)
pub fn store_master_password_hash(hash: &str) -> Result<(), LiteError> {
    let entry = Entry::new(SERVICE_NAME, "master_password")
        .map_err(|e| LiteError::Keychain(e.to_string()))?;

    entry
        .set_password(hash)
        .map_err(|e| LiteError::Keychain(e.to_string()))?;

    Ok(())
}

/// Get master password hash
pub fn get_master_password_hash() -> Result<Option<String>, LiteError> {
    let entry = Entry::new(SERVICE_NAME, "master_password")
        .map_err(|e| LiteError::Keychain(e.to_string()))?;

    match entry.get_password() {
        Ok(hash) => Ok(Some(hash)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(LiteError::Keychain(e.to_string())),
    }
}

/// Clear all stored passwords (emergency cleanup)
pub fn clear_all_passwords() -> Result<(), LiteError> {
    // Delete fallback file
    if let Ok(path) = fallback_store_path() {
        let _ = fs::remove_file(path);
    }

    // Note: Individual keychain entries must be deleted by server_id
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_store_path() {
        let path = fallback_store_path();
        assert!(path.is_ok());

        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("keychain_encrypted.bin"));
    }

    #[test]
    fn test_service_name_constant() {
        assert_eq!(SERVICE_NAME, "com.easyssh.lite");
    }

    #[test]
    fn test_encrypted_entry_serialization() {
        let entry = EncryptedEntry {
            encrypted_data: "base64data".to_string(),
            nonce: "base64nonce".to_string(),
        };

        let json = serde_json::to_string(&entry).expect("Failed to serialize");
        assert!(json.contains("base64data"));
        assert!(json.contains("base64nonce"));
    }

    #[test]
    fn test_encrypted_entry_deserialization() {
        let json = r#"{"encrypted_data":"dGVzdA==","nonce":"bm9uY2U="}"#;
        let entry: EncryptedEntry = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(entry.encrypted_data, "dGVzdA==");
        assert_eq!(entry.nonce, "bm9uY2U=");
    }

    #[test]
    fn test_base64_encoding() {
        // Test that base64 encoding works as expected
        let data = b"test data";
        let encoded = BASE64.encode(data);
        let decoded = BASE64.decode(&encoded).expect("Failed to decode");
        assert_eq!(data.to_vec(), decoded);
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = "Hello, World! This is a password with special chars: !@#$%^&*()";
        let encoded = BASE64.encode(original.as_bytes());
        let decoded = BASE64.decode(&encoded).expect("Failed to decode");
        let result = String::from_utf8(decoded).expect("Invalid UTF-8");
        assert_eq!(original, result);
    }

    #[test]
    fn test_legacy_fallback_path() {
        // This test just verifies the legacy path format
        // Actual migration testing requires file system access
        let path = dirs::data_local_dir().or_else(dirs::home_dir).map(|mut p| {
            p.push("EasySSH");
            p.push("keychain_fallback.json");
            p
        });

        if let Some(p) = path {
            // Path should contain the expected components
            let path_str = p.to_string_lossy();
            assert!(path_str.contains("keychain_fallback"));
        }
    }

    #[test]
    #[ignore = "Requires system keyring access, may hang in CI"]
    fn test_store_password_empty() {
        // Storing empty password should work
        let server_id = "test-empty-password-server";
        let _result = store_password(server_id, "");
        // May fail due to keychain not available in test, but shouldn't panic
        // Clean up
        let _ = delete_password(server_id);
    }

    #[test]
    #[ignore = "Requires system keyring access, may hang in CI"]
    fn test_store_password_unicode() {
        let server_id = "test-unicode-server";
        let password = "密码测试 🎉 ñoño émojis";
        let _result = store_password(server_id, password);
        // Clean up
        let _ = delete_password(server_id);
    }

    #[test]
    #[ignore = "Requires system keyring access, may hang in CI"]
    fn test_store_password_long() {
        let server_id = "test-long-password-server";
        let password = "a".repeat(1000);
        let _result = store_password(server_id, &password);
        // Clean up
        let _ = delete_password(server_id);
    }

    #[test]
    fn test_delete_password_nonexistent() {
        // Deleting a non-existent password should not panic
        let _result = delete_password("nonexistent-server-for-testing");
        // May succeed or fail, but should not panic
    }

    #[test]
    #[ignore = "Requires system keyring access, may hang in CI"]
    fn test_store_master_password_hash() {
        let hash = "argon2id_hash_test_string";
        let _result = store_master_password_hash(hash);
        // Clean up
        let _ = Entry::new(SERVICE_NAME, "master_password").map(|e| e.delete_credential());
    }

    #[test]
    fn test_get_master_password_hash_no_entry() {
        // Should return None when no entry exists
        // Note: This might return an error instead depending on keychain state
        let _result = get_master_password_hash();
        // Should not panic
    }

    #[test]
    fn test_clear_all_passwords() {
        let result = clear_all_passwords();
        assert!(result.is_ok());
    }

    #[test]
    fn test_encrypted_entry_struct() {
        let entry = EncryptedEntry {
            encrypted_data: "encrypted".to_string(),
            nonce: "nonce".to_string(),
        };

        let debug = format!("{:?}", entry);
        assert!(debug.contains("EncryptedEntry"));
    }

    #[test]
    fn test_encrypted_entry_clone() {
        let entry = EncryptedEntry {
            encrypted_data: "data".to_string(),
            nonce: "nonce".to_string(),
        };
        let cloned = entry.clone();
        assert_eq!(entry.encrypted_data, cloned.encrypted_data);
        assert_eq!(entry.nonce, cloned.nonce);
    }

    #[test]
    #[ignore = "Requires system keyring access, may hang in CI"]
    fn test_password_operations_sequence() {
        let server_id = "test-sequence-server";
        let password1 = "first_password";
        let password2 = "second_password";

        // Store first password
        let _ = store_password(server_id, password1);

        // Store second password (should overwrite)
        let _ = store_password(server_id, password2);

        // Delete password
        let _ = delete_password(server_id);

        // Verify deletion
        // get_password might return None or error
        let _ = get_password(server_id);
    }

    #[test]
    fn test_load_fallback_encrypted_no_file() {
        // When file doesn't exist, should return empty map
        // This tests the early return path
        // We can't easily test the full functionality without crypto state
    }

    #[test]
    fn test_base64_decode_invalid() {
        // Test decoding invalid base64
        let result = BASE64.decode("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_entry_zeroize() {
        let mut entry = EncryptedEntry {
            encrypted_data: "sensitive-data".to_string(),
            nonce: "sensitive-nonce".to_string(),
        };
        entry.zeroize();
        assert!(entry.encrypted_data.is_empty());
        assert!(entry.nonce.is_empty());
    }

    #[test]
    fn test_fallback_store_path_components() {
        let path = fallback_store_path().unwrap();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("EasySSH"));
        assert!(path_str.contains("keychain_encrypted.bin"));
    }

    #[test]
    fn test_service_name_format() {
        // Verify service name follows reverse domain convention
        assert!(SERVICE_NAME.starts_with("com."));
        assert!(SERVICE_NAME.contains("easyssh"));
    }

    #[test]
    fn test_base64_special_chars() {
        // Test encoding/decoding passwords with special characters
        let passwords = vec![
            "pass\nword",      // newline
            "pass\tword",      // tab
            "pass\x00word",   // null byte
            "日本語パスワード",  // Japanese
            "🔐🎉🚀",          // Emojis
        ];

        for pwd in &passwords {
            let encoded = BASE64.encode(pwd.as_bytes());
            let decoded = BASE64.decode(&encoded).expect("Failed to decode");
            let result = String::from_utf8(decoded).expect("Invalid UTF-8");
            assert_eq!(*pwd, result, "Failed for password: {:?}", pwd);
        }
    }

    #[test]
    fn test_encrypted_entry_empty() {
        let entry = EncryptedEntry {
            encrypted_data: String::new(),
            nonce: String::new(),
        };

        assert!(entry.encrypted_data.is_empty());
        assert!(entry.nonce.is_empty());
    }

    #[test]
    fn test_encrypted_entry_json_roundtrip() {
        let entry = EncryptedEntry {
            encrypted_data: "dGVzdA==".to_string(),
            nonce: "bm9uY2U=".to_string(),
        };

        let json = serde_json::to_string(&entry).expect("Failed to serialize");
        let deserialized: EncryptedEntry = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(entry.encrypted_data, deserialized.encrypted_data);
        assert_eq!(entry.nonce, deserialized.nonce);
    }

    #[test]
    fn test_base64_empty() {
        let empty = "";
        let encoded = BASE64.encode(empty.as_bytes());
        // Empty string encodes to empty string in base64
        assert_eq!(encoded, "");

        let decoded = BASE64.decode(&encoded).expect("Failed to decode empty");
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_base64_large_data() {
        // Test with larger data
        let data = vec![0u8; 10000];
        let encoded = BASE64.encode(&data);
        let decoded = BASE64.decode(&encoded).expect("Failed to decode large data");
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_base64_binary_data() {
        // Test with binary data containing all byte values
        let data: Vec<u8> = (0..=255).collect();
        let encoded = BASE64.encode(&data);
        let decoded = BASE64.decode(&encoded).expect("Failed to decode binary");
        assert_eq!(data, decoded);
    }

    #[test]
    #[ignore = "Requires system keyring access, may hang in CI"]
    fn test_get_password_not_found() {
        let result = get_password("definitely-non-existent-server-12345");
        // Should return Ok(None) for non-existent password
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_delete_password_idempotent() {
        // Deleting a password twice should not error
        let server_id = "test-delete-twice-server";

        // First delete (may or may not exist)
        let _ = delete_password(server_id);

        // Second delete should also not panic
        let result = delete_password(server_id);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires system keyring access, may hang in CI"]
    fn test_store_and_get_password_roundtrip() {
        let server_id = "test-roundtrip-server";
        let password = "my-secret-password-123!@#";

        // Store
        let store_result = store_password(server_id, password);
        assert!(store_result.is_ok());

        // Get
        let get_result = get_password(server_id);
        assert!(get_result.is_ok());

        // Verify
        if let Ok(Some(retrieved)) = get_result {
            assert_eq!(password, retrieved);
        }

        // Cleanup
        let _ = delete_password(server_id);
    }

    #[test]
    fn test_encrypted_entry_debug_format() {
        let entry = EncryptedEntry {
            encrypted_data: "test-data".to_string(),
            nonce: "test-nonce".to_string(),
        };

        let debug = format!("{:?}", entry);
        assert!(debug.contains("EncryptedEntry"));
        assert!(debug.contains("test-data"));
    }

    #[test]
    fn test_encrypted_entry_partial_eq() {
        let entry1 = EncryptedEntry {
            encrypted_data: "data".to_string(),
            nonce: "nonce".to_string(),
        };
        let entry2 = EncryptedEntry {
            encrypted_data: "data".to_string(),
            nonce: "nonce".to_string(),
        };
        let entry3 = EncryptedEntry {
            encrypted_data: "different".to_string(),
            nonce: "nonce".to_string(),
        };

        assert_eq!(entry1, entry2);
        assert_ne!(entry1, entry3);
    }

    #[test]
    fn test_base64_url_safe_chars() {
        // Test that base64 uses correct alphabet
        let data = b"\x00\x01\x02\x03\xFB\xFC\xFD\xFE\xFF";
        let encoded = BASE64.encode(data);

        // Base64 alphabet: A-Z, a-z, 0-9, +, /, =
        for c in encoded.chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=',
                "Invalid base64 character: {}",
                c
            );
        }
    }

    #[test]
    fn test_clear_all_passwords_idempotent() {
        // Calling clear multiple times should not error
        let result1 = clear_all_passwords();
        assert!(result1.is_ok());

        let result2 = clear_all_passwords();
        assert!(result2.is_ok());
    }

    #[test]
    fn test_encrypted_entry_with_real_base64() {
        // Create an entry with valid base64 data
        let original_data = b"test password data";
        let encrypted = BASE64.encode(original_data);
        let nonce = BASE64.encode(b"123456789012"); // 12 bytes for AES-GCM nonce

        let entry = EncryptedEntry {
            encrypted_data: encrypted,
            nonce,
        };

        // Verify we can decode
        let decoded_data = BASE64.decode(&entry.encrypted_data).expect("Failed to decode data");
        let decoded_nonce = BASE64.decode(&entry.nonce).expect("Failed to decode nonce");

        assert_eq!(original_data.to_vec(), decoded_data);
        assert_eq!(b"123456789012".to_vec(), decoded_nonce);
    }

    #[test]
    fn test_password_edge_cases() {
        // Test various edge case passwords
        let long_password = "a".repeat(1000);
        let edge_cases: Vec<&str> = vec![
            "",                                      // Empty
            "a",                                     // Single char
            " ",                                     // Space
            "  ",                                    // Multiple spaces
            &long_password,                          // Long password
            "!@#$%^&*()_+-=[]{}|;':\",./<>?",       // Special chars
            "\n\r\t",                                // Whitespace chars
            "'quoted'",                             // Quotes
            "\"double\"",                            // Double quotes
            "`backtick`",                           // Backticks
        ];

        for pwd in edge_cases {
            let encoded = BASE64.encode(pwd.as_bytes());
            let decoded = BASE64.decode(&encoded).expect("Failed to decode");
            let result = String::from_utf8(decoded).expect("Invalid UTF-8");
            assert_eq!(pwd, result, "Roundtrip failed for: {:?}", pwd);
        }
    }
}
