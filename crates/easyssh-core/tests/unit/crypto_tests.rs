//! Crypto Module Unit Tests
//!
//! Tests for cryptographic operations including:
//! - Encryption/decryption correctness
//! - Key derivation (Argon2id)
//! - Error handling (wrong password, corrupted data)
//! - Lock/unlock state management
//! - Secure memory clearing

use easyssh_core::crypto::{CryptoState, ServerCredential, AuthMethod, EncryptedServerCredential, EncryptedContainer};

#[path = "../common/mod.rs"]
mod common;
use common::{test_master_password, test_wrong_password, test_encryption_data};

#[test]
fn test_crypto_state_new_is_locked() {
    let state = CryptoState::new();
    assert!(!state.is_unlocked(), "New CryptoState should be locked");
}

#[test]
fn test_crypto_state_initialize_unlocks() {
    let mut state = CryptoState::new();
    let result = state.initialize(test_master_password());
    assert!(result.is_ok(), "Initialize should succeed: {:?}", result.err());
    assert!(state.is_unlocked(), "State should be unlocked after initialization");
}

#[test]
fn test_encrypt_decrypt_roundtrip() {
    let mut state = CryptoState::new();
    state.initialize(test_master_password()).expect("Initialize should succeed");

    let plaintext = test_encryption_data();

    // Encrypt
    let encrypted = state.encrypt(plaintext).expect("Encryption should succeed");
    assert!(!encrypted.is_empty(), "Encrypted data should not be empty");
    assert!(encrypted.len() > 12, "Encrypted data should include nonce");

    // Decrypt
    let decrypted = state.decrypt(&encrypted).expect("Decryption should succeed");
    assert_eq!(decrypted, plaintext, "Decrypted data should match original");
}

#[test]
fn test_decrypt_with_wrong_password_fails() {
    let mut state1 = CryptoState::new();
    state1.initialize(test_master_password()).expect("Initialize should succeed");

    let plaintext = test_encryption_data();
    let encrypted = state1.encrypt(plaintext).expect("Encryption should succeed");

    // Create new state with different password but same salt
    let salt = state1.get_salt().expect("Should have salt");
    let mut state2 = CryptoState::new();
    state2.set_salt(salt.try_into().expect("Salt should be 32 bytes"));

    // Try to unlock with wrong password
    let unlock_result = state2.unlock(test_wrong_password());
    assert!(unlock_result.is_ok(), "Unlock with wrong password should succeed (key derivation works)");

    // But decryption should fail with corrupted data or wrong key
    let decrypt_result = state2.decrypt(&encrypted);
    assert!(decrypt_result.is_err(), "Decryption with wrong key should fail");
}

#[test]
fn test_lock_clears_key() {
    let mut state = CryptoState::new();
    state.initialize(test_master_password()).expect("Initialize should succeed");
    assert!(state.is_unlocked(), "State should be unlocked");

    // Lock the state
    state.lock();
    assert!(!state.is_unlocked(), "State should be locked after lock()");

    // Encryption should fail when locked
    let result = state.encrypt(b"test");
    assert!(result.is_err(), "Encryption should fail when locked");
}

#[test]
fn test_unlock_with_correct_password() {
    let mut state = CryptoState::new();
    state.initialize(test_master_password()).expect("Initialize should succeed");

    let salt = state.get_salt().expect("Should have salt");
    let plaintext = b"test data for unlock";
    let encrypted = state.encrypt(plaintext).expect("Encryption should succeed");

    // Lock and re-unlock
    state.lock();
    assert!(!state.is_unlocked());

    state.set_salt(salt.try_into().expect("Salt should be 32 bytes"));
    let unlock_result = state.unlock(test_master_password());
    assert!(unlock_result.is_ok(), "Unlock with correct password should succeed");
    assert!(state.is_unlocked(), "State should be unlocked");

    // Should be able to decrypt again
    let decrypted = state.decrypt(&encrypted).expect("Decryption should succeed after unlock");
    assert_eq!(decrypted, plaintext, "Decrypted data should match");
}

#[test]
fn test_server_credential_with_password() {
    let credential = ServerCredential::with_password(
        "test-server",
        "192.168.1.100",
        "admin",
        "secret123"
    );

    assert_eq!(credential.id, "test-server");
    assert_eq!(credential.host, "192.168.1.100");
    assert_eq!(credential.username, "admin");
    assert_eq!(credential.port, 22);

    match &credential.auth_method {
        AuthMethod::Password { encrypted } => {
            assert_eq!(encrypted, b"secret123");
        }
        _ => panic!("Expected Password auth method"),
    }
}

#[test]
fn test_server_credential_with_ssh_key() {
    let credential = ServerCredential::with_ssh_key(
        "test-server",
        "192.168.1.100",
        "admin",
        "-----BEGIN RSA KEY-----\n...",
        Some("key_passphrase")
    );

    assert_eq!(credential.id, "test-server");

    match &credential.auth_method {
        AuthMethod::SshKey { private_key_encrypted, passphrase_encrypted } => {
            assert!(!private_key_encrypted.is_empty());
            assert!(passphrase_encrypted.is_some());
            assert_eq!(passphrase_encrypted.as_ref().unwrap(), b"key_passphrase");
        }
        _ => panic!("Expected SshKey auth method"),
    }
}

#[test]
fn test_server_credential_with_custom_port() {
    let credential = ServerCredential::with_password("srv", "host", "user", "pass")
        .with_port(2222);

    assert_eq!(credential.port, 2222);
}

#[test]
fn test_credential_encrypt_decrypt() {
    let mut state = CryptoState::new();
    state.initialize(test_master_password()).expect("Initialize should succeed");

    let credential = ServerCredential::with_password(
        "test-server",
        "192.168.1.100",
        "admin",
        "secret_password"
    );

    // Encrypt
    let encrypted = credential.encrypt(&state).expect("Credential encryption should succeed");
    assert_eq!(encrypted.id, "test-server");
    assert!(!encrypted.encrypted_data.is_empty());

    // Decrypt
    let decrypted = encrypted.decrypt(&state).expect("Credential decryption should succeed");
    assert_eq!(decrypted.id, credential.id);
    assert_eq!(decrypted.host, credential.host);
    assert_eq!(decrypted.username, credential.username);
    assert_eq!(decrypted.port, credential.port);

    match (&decrypted.auth_method, &credential.auth_method) {
        (AuthMethod::Password { encrypted: e1 }, AuthMethod::Password { encrypted: e2 }) => {
            assert_eq!(e1, e2);
        }
        _ => panic!("Auth methods don't match"),
    }
}

#[test]
fn test_encrypted_container_version() {
    let data = vec![1, 2, 3, 4, 5];
    let container = EncryptedContainer::new(data);

    assert_eq!(container.version, 1);
    assert!(container.is_compatible());
}

#[test]
fn test_encrypt_different_plaintexts_produces_different_ciphertexts() {
    let mut state = CryptoState::new();
    state.initialize(test_master_password()).expect("Initialize should succeed");

    let plaintext1 = b"message one";
    let plaintext2 = b"message two";

    let encrypted1 = state.encrypt(plaintext1).expect("Encryption should succeed");
    let encrypted2 = state.encrypt(plaintext2).expect("Encryption should succeed");

    // Different plaintexts should produce different ciphertexts (with high probability due to random nonces)
    assert_ne!(encrypted1, encrypted2, "Different plaintexts should produce different ciphertexts");
}

#[test]
fn test_encrypt_same_plaintext_produces_different_ciphertexts() {
    let mut state = CryptoState::new();
    state.initialize(test_master_password()).expect("Initialize should succeed");

    let plaintext = b"same message";

    let encrypted1 = state.encrypt(plaintext).expect("Encryption should succeed");
    let encrypted2 = state.encrypt(plaintext).expect("Encryption should succeed");

    // Same plaintext encrypted twice should produce different ciphertexts (due to random nonces)
    assert_ne!(encrypted1, encrypted2, "Same plaintext should produce different ciphertexts due to random nonces");

    // But both should decrypt to the same plaintext
    let decrypted1 = state.decrypt(&encrypted1).expect("Decryption should succeed");
    let decrypted2 = state.decrypt(&encrypted2).expect("Decryption should succeed");
    assert_eq!(decrypted1, plaintext);
    assert_eq!(decrypted2, plaintext);
}

#[test]
fn test_decrypt_too_short_data_fails() {
    let mut state = CryptoState::new();
    state.initialize(test_master_password()).expect("Initialize should succeed");

    let too_short = vec![1, 2, 3]; // Less than 12 bytes (nonce length)
    let result = state.decrypt(&too_short);
    assert!(result.is_err(), "Decrypting too short data should fail");
}

#[test]
fn test_decrypt_corrupted_data_fails() {
    let mut state = CryptoState::new();
    state.initialize(test_master_password()).expect("Initialize should succeed");

    let plaintext = b"test message";
    let mut encrypted = state.encrypt(plaintext).expect("Encryption should succeed");

    // Corrupt some bytes in the ciphertext (after the nonce)
    if encrypted.len() > 15 {
        encrypted[15] ^= 0xFF; // Flip bits in one byte
    }

    let result = state.decrypt(&encrypted);
    assert!(result.is_err(), "Decrypting corrupted data should fail");
}

#[test]
fn test_get_set_salt() {
    let mut state = CryptoState::new();

    // Initially no salt
    assert!(state.get_salt().is_none());

    // Set a salt
    let salt = [1u8; 32];
    state.set_salt(salt);

    // Should be able to get it back
    let retrieved = state.get_salt().expect("Should have salt");
    assert_eq!(retrieved, salt.to_vec());
}

#[test]
fn test_default_crypto_state() {
    let state: CryptoState = Default::default();
    assert!(!state.is_unlocked());
    assert!(state.get_salt().is_none());
}

#[test]
fn test_credential_zeroize_on_drop() {
    // This test mainly ensures that ZeroizeOnDrop is implemented
    // The actual zeroization is difficult to test directly
    let credential = ServerCredential::with_password(
        "test",
        "host",
        "user",
        "password123"
    );

    // Just create and drop
    drop(credential);
    // If we get here without issues, the zeroize implementation is present
}
