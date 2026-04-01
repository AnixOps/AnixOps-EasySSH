//! SFTP File Transfer Performance Benchmarks
//!
//! Tests SFTP file transfer operations, progress tracking, and queue management.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use easyssh_core::sftp::{
    SftpEntry, TransferDirection, TransferItem, TransferOptions, TransferProgress,
    TransferQueue, TransferStatus,
};
use std::time::Duration;

fn bench_transfer_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("sftp_transfer_progress");

    group.bench_function("new", |b| {
        b.iter(|| {
            let _ = black_box(TransferProgress::new("test.txt", Some(1024), false));
        });
    });

    group.bench_function("update", |b| {
        let mut progress = TransferProgress::new("test.txt", Some(1048576), false);
        let mut transferred = 0u64;

        b.iter(|| {
            transferred = (transferred + 1024) % 1048576;
            progress.update(black_box(transferred));
        });
    });

    group.bench_function("percentage_calculation", |b| {
        let progress = TransferProgress {
            transferred: 524288,
            total: Some(1048576),
            start_time: chrono::Utc::now().timestamp(),
            speed_bps: 1024.0 * 1024.0,
            elapsed_secs: 0.5,
            eta_secs: Some(0.5),
            status: TransferStatus::Transferring,
            filename: "test.txt".to_string(),
            is_resume: false,
            resume_offset: 0,
        };

        b.iter(|| {
            let _ = black_box(progress.percentage());
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
            let _ = black_box(
                TransferOptions::default()
                    .with_chunk_size(128 * 1024)
                    .with_resume(true)
                    .with_max_concurrent(5)
                    .with_speed_limit(1024 * 1024)
                    .with_overwrite(false)
            );
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
        let options = TransferOptions::default();

        b.iter(|| {
            let _ = black_box(TransferItem::new(
                "session-1",
                "/remote/file.txt",
                "/local/file.txt",
                TransferDirection::Download,
                options.clone(),
                Some(1024),
            ));
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
                    let options = TransferOptions::default();

                    rt.block_on(async {
                        for i in 0..item_count {
                            let item = TransferItem::new(
                                "session-1",
                                format!("/remote/file{}.txt", i),
                                format!("/local/file{}.txt", i),
                                TransferDirection::Download,
                                options.clone(),
                                Some(1024),
                            );
                            queue.add(item).await;
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
        group.bench_with_input(
            BenchmarkId::new("format_size", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let _ = black_box(format_size(size));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    sftp_benches,
    bench_transfer_progress,
    bench_transfer_options,
    bench_sftp_entry,
    bench_transfer_item_creation,
    bench_transfer_queue,
    bench_format_functions
);
criterion_main!(sftp_benches);
