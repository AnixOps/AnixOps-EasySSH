//! Cryptographic Performance Benchmarks
//!
//! This module benchmarks the performance of cryptographic operations used in EasySSH.
//!
//! # Benchmark Scenarios
//!
//! - Key derivation (Argon2id) with different memory costs
//! - AES-256-GCM encryption/decryption for various data sizes
//! - Credential encryption/decryption round trips
//! - Master key unlock performance
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --bench crypto_bench
//! cargo bench --bench crypto_bench -- --save-baseline baseline1
//! cargo bench --bench crypto_bench -- --baseline baseline1
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use easyssh_core::crypto::{CredentialEncryption, CryptoState, MasterKey, ServerCredential};

/// Initialize crypto state with a test password
fn init_crypto_state() -> CryptoState {
    let mut state = CryptoState::new();
    state.initialize("benchmark_test_password_123").unwrap();
    state
}

/// Generate test data of specified size
fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Benchmark AES-256-GCM encryption for various data sizes
fn bench_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("aes256_gcm_encrypt");
    let state = init_crypto_state();

    // Test different data sizes
    for size in [64, 256, 1024, 4096, 16384, 65536, 262144, 1048576].iter() {
        let data = generate_test_data(*size);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_bytes", size)),
            &data,
            |b, data| {
                b.iter(|| {
                    let encrypted = state.encrypt(black_box(data)).unwrap();
                    black_box(encrypted);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark AES-256-GCM decryption for various data sizes
fn bench_decryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("aes256_gcm_decrypt");
    let state = init_crypto_state();

    // Pre-encrypt data for decryption benchmarks
    let test_sizes = [64, 256, 1024, 4096, 16384, 65536, 262144, 1048576];
    let encrypted_data: Vec<_> = test_sizes
        .iter()
        .map(|size| {
            let data = generate_test_data(*size);
            (*size, state.encrypt(&data).unwrap())
        })
        .collect();

    for (size, encrypted) in encrypted_data {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_bytes", size)),
            &encrypted,
            |b, encrypted| {
                b.iter(|| {
                    let decrypted = state.decrypt(black_box(encrypted)).unwrap();
                    black_box(decrypted);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark encrypt/decrypt round trip
fn bench_encrypt_decrypt_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("encrypt_decrypt_roundtrip");
    let state = init_crypto_state();

    for size in [256, 1024, 4096, 16384, 65536].iter() {
        let data = generate_test_data(*size);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_bytes", size)),
            &data,
            |b, data| {
                b.iter(|| {
                    let encrypted = state.encrypt(black_box(data)).unwrap();
                    let decrypted = state.decrypt(black_box(&encrypted)).unwrap();
                    black_box(decrypted);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark credential encryption
fn bench_credential_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("credential_encryption");
    let state = init_crypto_state();

    group.bench_function("password_credential", |b| {
        let credential = ServerCredential::with_password(
            "server-1",
            "192.168.1.100",
            "admin",
            "secret_password_123",
        );

        b.iter(|| {
            let encrypted = credential.encrypt(black_box(&state)).unwrap();
            black_box(encrypted);
        });
    });

    group.bench_function("ssh_key_credential", |b| {
        let private_key = "-----BEGIN OPENSSH PRIVATE KEY-----\n".to_string()
            + &"b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n"
                .repeat(20)
            + "-----END OPENSSH PRIVATE KEY-----";

        let credential = ServerCredential::with_ssh_key(
            "server-2",
            "192.168.1.101",
            "root",
            &private_key,
            Some("key_passphrase"),
        );

        b.iter(|| {
            let encrypted = credential.encrypt(black_box(&state)).unwrap();
            black_box(encrypted);
        });
    });

    group.bench_function("credential_roundtrip", |b| {
        let credential =
            ServerCredential::with_password("server-3", "192.168.1.102", "user", "password123");
        let encrypted = credential.encrypt(&state).unwrap();

        b.iter(|| {
            let decrypted = encrypted.decrypt(black_box(&state)).unwrap();
            black_box(decrypted);
        });
    });

    group.finish();
}

/// Benchmark key derivation (Argon2id)
fn bench_key_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_derivation");

    // Note: These benchmarks use the actual Argon2id parameters
    // and may take longer due to the memory-hard nature of Argon2
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(30));

    let passwords = [
        ("short_password", "123456"),
        ("medium_password", "my_secure_password_123"),
        (
            "long_password",
            "this_is_a_very_long_password_with_special_chars_!@#$%^&*()",
        ),
    ];

    for (name, password) in passwords.iter() {
        group.bench_function(*name, |b| {
            b.iter(|| {
                let mut state = CryptoState::new();
                state.initialize(black_box(password)).unwrap();
                black_box(state);
            });
        });
    }

    group.finish();
}

/// Benchmark master key operations
fn bench_master_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("master_key");

    // Note: Keychain operations may not work in CI environment
    // These benchmarks focus on the crypto operations, not keychain storage

    group.bench_function("initialization", |b| {
        b.iter(|| {
            let mut master = MasterKey::new();
            master
                .initialize(black_box("benchmark_master_password"))
                .unwrap();
            black_box(master);
        });
    });

    // Pre-initialize for unlock benchmark
    let mut master = MasterKey::new();
    master.initialize("unlock_test_password").unwrap();
    let salt = master.get_salt().unwrap();

    group.bench_function("unlock", |b| {
        b.iter(|| {
            let mut new_master = MasterKey::new();
            // Set salt and attempt unlock
            let mut salt_array = [0u8; 32];
            salt_array.copy_from_slice(&salt);
            new_master
                .unlock(black_box("unlock_test_password"))
                .unwrap();
            black_box(new_master);
        });
    });

    group.finish();
}

/// Benchmark multiple concurrent encryption operations
fn bench_concurrent_encryption(c: &mut Criterion) {
    use std::thread;

    let mut group = c.benchmark_group("concurrent_encryption");

    for num_threads in [1, 2, 4, 8].iter() {
        group.bench_function(format!("{}_threads", num_threads), |b| {
            let data = std::sync::Arc::new(generate_test_data(1024));

            b.iter(|| {
                let mut handles = Vec::new();

                for _ in 0..*num_threads {
                    let data_clone = std::sync::Arc::clone(&data);
                    let handle = thread::spawn(move || {
                        let mut state = CryptoState::new();
                        state.initialize("concurrent_pass").unwrap();
                        let encrypted = state.encrypt(&data_clone).unwrap();
                        black_box(encrypted);
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

/// Benchmark credential encryption with various payload sizes
fn bench_credential_encryption_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("credential_encryption_sizes");
    let state = init_crypto_state();

    for metadata_size in [0, 256, 1024, 4096, 16384].iter() {
        let metadata = if *metadata_size > 0 {
            Some(generate_test_data(*metadata_size))
        } else {
            None
        };

        let mut credential =
            ServerCredential::with_password("server-test", "192.168.1.1", "user", "password123");

        if let Some(meta) = metadata {
            credential.metadata_encrypted = Some(meta);
        }

        group.throughput(Throughput::Bytes(*metadata_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_bytes_metadata", metadata_size)),
            &credential,
            |b, cred| {
                b.iter(|| {
                    let encrypted = cred.encrypt(black_box(&state)).unwrap();
                    black_box(encrypted);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark password-based encryption (CredentialEncryption)
fn bench_password_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("password_encryption");
    let state = init_crypto_state();
    let encryption = CredentialEncryption::new(state);

    let passwords = [
        ("short", "123456"),
        ("medium", "secure_password_123"),
        ("long", "a".repeat(256)),
    ];

    for (name, password) in passwords.iter() {
        group.bench_function(*name, |b| {
            b.iter(|| {
                let encrypted = encryption.encrypt_password(black_box(password)).unwrap();
                black_box(encrypted);
            });
        });
    }

    group.finish();
}

// Criterion group configuration
criterion_group!(
    name = crypto_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets =
        bench_encryption,
        bench_decryption,
        bench_encrypt_decrypt_roundtrip,
        bench_credential_encryption,
        bench_key_derivation,
        bench_master_key,
        bench_concurrent_encryption,
        bench_credential_encryption_sizes,
        bench_password_encryption
);

criterion_main!(crypto_benches);
