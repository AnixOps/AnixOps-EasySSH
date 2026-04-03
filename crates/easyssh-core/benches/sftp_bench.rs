//! SFTP File Transfer Performance Benchmarks
//!
//! Tests SFTP file transfer operations, progress tracking, and queue management.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use easyssh_core::sftp::{
    SftpEntry, TransferDirection, TransferTask, TransferOptions, TransferStats, TransferQueue,
    TransferStatus,
};
use std::time::Duration;

fn bench_transfer_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("sftp_transfer_stats");

    group.bench_function("new", |b| {
        b.iter(|| {
            let _ = black_box(TransferStats::default());
        });
    });

    group.bench_function("with_values", |b| {
        b.iter(|| {
            let stats = TransferStats {
                total_tasks: 10,
                completed: 5,
                failed: 1,
                active: 2,
                pending: 2,
                total_bytes: 1024 * 1024,
                current_speed: 1024.0 * 1024.0,
                eta_seconds: Some(1.0),
            };
            black_box(stats);
        });
    });

    group.finish();
}

fn bench_transfer_options(c: &mut Criterion) {
    let mut group = c.benchmark_group("sftp_transfer_options");

    group.bench_function("default", |b| {
        b.iter(|| {
            let _ = black_box(TransferOptions::default());
        });
    });

    group.bench_function("builder", |b| {
        b.iter(|| {
            let _ = black_box(TransferOptions {
                chunk_size: 128 * 1024,
                resume: true,
                max_concurrent: 5,
                speed_limit: 1024 * 1024,
                overwrite: false,
                preserve_time: true,
                preserve_permissions: true,
                file_mode: 0o644,
                timeout: std::time::Duration::from_secs(30),
                retry_count: 3,
                verify_checksum: true,
            });
        });
    });

    group.finish();
}

fn bench_sftp_entry(c: &mut Criterion) {
    let mut group = c.benchmark_group("sftp_entry");

    group.bench_function("size_display_file", |b| {
        let entry = SftpEntry {
            name: "test.txt".to_string(),
            path: "/home/test.txt".to_string(),
            file_type: "file".to_string(),
            size: 1024 * 1024 * 5,
            mtime: 1234567890,
            permissions: Some(0o644),
        };

        b.iter(|| {
            let _ = black_box(entry.size_display());
        });
    });

    group.bench_function("serialize", |b| {
        let entry = SftpEntry {
            name: "test.txt".to_string(),
            path: "/home/test.txt".to_string(),
            file_type: "file".to_string(),
            size: 1024,
            mtime: 1234567890,
            permissions: Some(0o644),
        };

        b.iter(|| {
            let _ = black_box(serde_json::to_string(&entry).unwrap());
        });
    });

    group.finish();
}

fn bench_transfer_item_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sftp_transfer_item");

    group.bench_function("new_download", |b| {
        b.iter(|| {
            let _ = black_box(TransferTask::new(
                "/remote/file.txt",
                "/local/file.txt",
                TransferDirection::Download,
                "session-1",
            ));
        });
    });

    group.bench_function("with_options", |b| {
        let options = TransferOptions::default();

        b.iter(|| {
            let task = TransferTask::new(
                "/remote/file.txt",
                "/local/file.txt",
                TransferDirection::Download,
                "session-1",
            ).with_options(options.clone());
            black_box(task);
        });
    });

    group.finish();
}

fn bench_transfer_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("sftp_transfer_queue");

    let rt = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("new", |b| {
        b.iter(|| {
            let _ = black_box(TransferQueue::new());
        });
    });

    for item_count in [10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::new("add_items", item_count),
            &item_count,
            |b, &item_count| {
                b.iter(|| {
                    let queue = TransferQueue::new();

                    rt.block_on(async {
                        for i in 0..item_count {
                            let task = TransferTask::new(
                                format!("/remote/file{}.txt", i),
                                format!("/local/file{}.txt", i),
                                TransferDirection::Download,
                                "session-1",
                            );
                            queue.add(task).await;
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size >= TB {
        format!("{:.2} TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

fn bench_format_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("sftp_format");

    for size in [0u64, 512, 1024, 10240, 1048576, 1073741824] {
        group.bench_with_input(BenchmarkId::new("format_size", size), &size, |b, &size| {
            b.iter(|| {
                let _ = black_box(format_size(size));
            });
        });
    }

    group.finish();
}

criterion_group!(
    sftp_benches,
    bench_transfer_stats,
    bench_transfer_options,
    bench_sftp_entry,
    bench_transfer_item_creation,
    bench_transfer_queue,
    bench_format_functions
);
criterion_main!(sftp_benches);
