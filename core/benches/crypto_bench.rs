//! Crypto Performance Benchmarks
//!
//! Tests encryption/decryption performance, key derivation, and various data sizes.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use easyssh_core::crypto::CryptoState;

fn bench_key_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto_key_derivation");

    for password_len in [8, 16, 32, 64, 128] {
        let password = "a".repeat(password_len);
        group.bench_with_input(
            BenchmarkId::from_parameter(password_len),
            &password,
            |b, password| {
                b.iter(|| {
                    let mut state = CryptoState::new();
                    state.initialize(black_box(password)).unwrap();
                    state.lock();
                });
            },
        );
    }

    group.finish();
}

fn bench_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto_encryption");

    // Setup crypto state
    let mut state = CryptoState::new();
    state.initialize("benchmark_password_12345").unwrap();

    for size in [1024, 4096, 16384, 65536, 262144, 1048576] {
        let data = vec![0u8; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("encrypt", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let _ = state.encrypt(black_box(data)).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_decryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto_decryption");

    // Setup crypto state
    let mut state = CryptoState::new();
    state.initialize("benchmark_password_12345").unwrap();

    for size in [1024, 4096, 16384, 65536, 262144, 1048576] {
        let data = vec![0u8; size];
        let encrypted = state.encrypt(&data).unwrap();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("decrypt", size),
            &encrypted,
            |b, encrypted| {
                b.iter(|| {
                    let _ = state.decrypt(black_box(encrypted)).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto_roundtrip");

    for size in [1024, 4096, 16384, 65536, 262144] {
        let data = vec![0u8; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(BenchmarkId::from_parameter(size), |b| {
            b.iter(|| {
                let mut state = CryptoState::new();
                state.initialize("benchmark_password_12345").unwrap();
                let encrypted = state.encrypt(black_box(&data)).unwrap();
                let _ = state.decrypt(&encrypted).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_unlock(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto_unlock");

    // Create a state with known salt
    let mut init_state = CryptoState::new();
    init_state.initialize("benchmark_password_unlock").unwrap();
    let salt = init_state.get_salt().unwrap();
    let mut salt_array = [0u8; 32];
    salt_array.copy_from_slice(&salt);

    group.bench_function("unlock", |b| {
        b.iter(|| {
            let mut state = CryptoState::new();
            state.set_salt(salt_array);
            state.unlock(black_box("benchmark_password_unlock")).unwrap();
        });
    });

    group.finish();
}

fn bench_concurrent_encryption(c: &mut Criterion) {
    use std::thread;

    let mut group = c.benchmark_group("crypto_concurrent");

    for num_threads in [1, 2, 4, 8, 16] {
        let data = vec![0u8; 65536];

        group.bench_function(BenchmarkId::from_parameter(num_threads), |b| {
            b.iter(|| {
                let mut handles = vec![];
                let data = black_box(&data);

                for _ in 0..num_threads {
                    let data = data.clone();
                    handles.push(thread::spawn(move || {
                        let mut state = CryptoState::new();
                        state.initialize("concurrent_test").unwrap();
                        let _ = state.encrypt(&data).unwrap();
                    }));
                }

                for handle in handles {
                    handle.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    crypto_benches,
    bench_key_derivation,
    bench_encryption,
    bench_decryption,
    bench_roundtrip,
    bench_unlock,
    bench_concurrent_encryption
);
criterion_main!(crypto_benches);
