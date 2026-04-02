//! Backup verification and integrity checking

use super::{BackupError, BackupResult, SnapshotId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use tracing::{error, info, warn};

/// Backup verification options
#[derive(Debug, Clone)]
pub struct VerificationOptions {
    /// Verify checksums
    pub verify_checksums: bool,
    /// Verify file structure
    pub verify_structure: bool,
    /// Test restoration
    pub test_restore: bool,
    /// Verify can read all files
    pub verify_readable: bool,
    /// Maximum time to spend verifying (seconds)
    pub max_verification_time: u64,
}

impl Default for VerificationOptions {
    fn default() -> Self {
        Self {
            verify_checksums: true,
            verify_structure: true,
            test_restore: false,
            verify_readable: true,
            max_verification_time: 3600, // 1 hour
        }
    }
}

/// Verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub success: bool,
    pub verified_files: u64,
    pub failed_files: u64,
    pub total_bytes: u64,
    pub errors: Vec<VerificationError>,
    pub warnings: Vec<String>,
    pub duration_seconds: f64,
}

/// Verification error details
#[derive(Debug, Clone)]
pub struct VerificationError {
    pub file_path: PathBuf,
    pub error_type: VerificationErrorType,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationErrorType {
    ChecksumMismatch,
    FileNotFound,
    FileCorrupted,
    PermissionDenied,
    InvalidStructure,
    CannotRead,
    MetadataMismatch,
}

/// Backup verifier
pub struct BackupVerifier {
    options: VerificationOptions,
}

impl BackupVerifier {
    /// Create a new backup verifier
    pub fn new(options: VerificationOptions) -> Self {
        Self { options }
    }

    /// Verify a backup file integrity
    pub async fn verify_file(
        &self,
        path: &Path,
        expected_checksum: Option<&str>,
    ) -> VerificationResult {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check file exists
        if !path.exists() {
            return VerificationResult {
                success: false,
                verified_files: 0,
                failed_files: 1,
                total_bytes: 0,
                errors: vec![VerificationError {
                    file_path: path.to_path_buf(),
                    error_type: VerificationErrorType::FileNotFound,
                    message: "File not found".to_string(),
                }],
                warnings,
                duration_seconds: 0.0,
            };
        }

        // Verify readable
        if self.options.verify_readable {
            match tokio::fs::File::open(path).await {
                Ok(mut file) => {
                    let mut buffer = [0u8; 4096];
                    match file.read(&mut buffer).await {
                        Ok(_) => {}
                        Err(e) => {
                            errors.push(VerificationError {
                                file_path: path.to_path_buf(),
                                error_type: VerificationErrorType::CannotRead,
                                message: format!("Cannot read file: {}", e),
                            });
                        }
                    }
                }
                Err(e) => {
                    errors.push(VerificationError {
                        file_path: path.to_path_buf(),
                        error_type: VerificationErrorType::PermissionDenied,
                        message: format!("Permission denied: {}", e),
                    });
                }
            }
        }

        // Verify checksum
        if self.options.verify_checksums && expected_checksum.is_some() {
            match self.verify_checksum(path, expected_checksum.unwrap()).await {
                Ok(true) => {}
                Ok(false) => {
                    errors.push(VerificationError {
                        file_path: path.to_path_buf(),
                        error_type: VerificationErrorType::ChecksumMismatch,
                        message: "Checksum mismatch".to_string(),
                    });
                }
                Err(e) => {
                    errors.push(VerificationError {
                        file_path: path.to_path_buf(),
                        error_type: VerificationErrorType::FileCorrupted,
                        message: format!("Failed to compute checksum: {}", e),
                    });
                }
            }
        }

        // Get file size
        let total_bytes = tokio::fs::metadata(path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let duration = start_time.elapsed().as_secs_f64();

        VerificationResult {
            success: errors.is_empty(),
            verified_files: if errors.is_empty() { 1 } else { 0 },
            failed_files: if errors.is_empty() { 0 } else { 1 },
            total_bytes,
            errors,
            warnings,
            duration_seconds: duration,
        }
    }

    /// Verify checksum of a file
    async fn verify_checksum(&self, path: &Path, expected: &str) -> Result<bool, BackupError> {
        let data = tokio::fs::read(path).await.map_err(BackupError::Io)?;
        let actual = blake3::hash(&data).to_hex().to_string();
        Ok(actual == expected)
    }

    /// Verify an entire backup directory
    pub async fn verify_directory(
        &self,
        path: &Path,
        checksum_manifest: Option<&HashMap<PathBuf, String>>,
    ) -> VerificationResult {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut verified_files = 0u64;
        let mut failed_files = 0u64;
        let mut total_bytes = 0u64;

        let entries = walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file());

        for entry in entries {
            let file_path = entry.path();
            let relative_path = file_path.strip_prefix(path).unwrap_or(file_path);

            // Check if file is in manifest
            let expected_checksum = checksum_manifest.and_then(|m| m.get(relative_path).cloned());

            let result = self
                .verify_file(file_path, expected_checksum.as_deref())
                .await;

            verified_files += result.verified_files;
            failed_files += result.failed_files;
            total_bytes += result.total_bytes;
            errors.extend(result.errors);
            warnings.extend(result.warnings);
        }

        let duration = start_time.elapsed().as_secs_f64();

        VerificationResult {
            success: errors.is_empty(),
            verified_files,
            failed_files,
            total_bytes,
            errors,
            warnings,
            duration_seconds: duration,
        }
    }

    /// Test if a backup can be restored
    pub async fn test_restore(
        &self,
        backup_path: &Path,
        temp_restore_path: &Path,
        format: super::CompressionFormat,
    ) -> VerificationResult {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        info!("Testing restore to {:?}", temp_restore_path);

        // Create temp directory
        if let Err(e) = tokio::fs::create_dir_all(temp_restore_path).await {
            return VerificationResult {
                success: false,
                verified_files: 0,
                failed_files: 0,
                total_bytes: 0,
                errors: vec![VerificationError {
                    file_path: temp_restore_path.to_path_buf(),
                    error_type: VerificationErrorType::PermissionDenied,
                    message: format!("Cannot create temp directory: {}", e),
                }],
                warnings,
                duration_seconds: 0.0,
            };
        }

        // Try to restore
        match super::decompress_backup(backup_path, temp_restore_path, format).await {
            Ok(_) => {
                // Verify restored files
                let verify_result = self.verify_directory(temp_restore_path, None).await;

                // Cleanup
                let _ = tokio::fs::remove_dir_all(temp_restore_path).await;

                let duration = start_time.elapsed().as_secs_f64();

                VerificationResult {
                    success: verify_result.success,
                    verified_files: verify_result.verified_files,
                    failed_files: verify_result.failed_files,
                    total_bytes: verify_result.total_bytes,
                    errors: verify_result.errors,
                    warnings: verify_result.warnings,
                    duration_seconds: duration,
                }
            }
            Err(e) => {
                let _ = tokio::fs::remove_dir_all(temp_restore_path).await;

                errors.push(VerificationError {
                    file_path: backup_path.to_path_buf(),
                    error_type: VerificationErrorType::FileCorrupted,
                    message: format!("Failed to restore: {}", e),
                });

                VerificationResult {
                    success: false,
                    verified_files: 0,
                    failed_files: 0,
                    total_bytes: 0,
                    errors,
                    warnings,
                    duration_seconds: start_time.elapsed().as_secs_f64(),
                }
            }
        }
    }

    /// Verify backup index integrity
    pub async fn verify_index(&self, index_path: &Path, data_dir: &Path) -> VerificationResult {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut verified_files = 0u64;
        let mut failed_files = 0u64;
        let mut total_bytes = 0u64;

        // Load index
        let index_json = match tokio::fs::read_to_string(index_path).await {
            Ok(json) => json,
            Err(e) => {
                return VerificationResult {
                    success: false,
                    verified_files: 0,
                    failed_files: 0,
                    total_bytes: 0,
                    errors: vec![VerificationError {
                        file_path: index_path.to_path_buf(),
                        error_type: VerificationErrorType::FileNotFound,
                        message: format!("Cannot read index: {}", e),
                    }],
                    warnings,
                    duration_seconds: 0.0,
                };
            }
        };

        // Parse index (assuming it's a JSON map of file paths to checksums)
        let index: HashMap<String, String> = match serde_json::from_str(&index_json) {
            Ok(idx) => idx,
            Err(e) => {
                return VerificationResult {
                    success: false,
                    verified_files: 0,
                    failed_files: 0,
                    total_bytes: 0,
                    errors: vec![VerificationError {
                        file_path: index_path.to_path_buf(),
                        error_type: VerificationErrorType::InvalidStructure,
                        message: format!("Invalid index format: {}", e),
                    }],
                    warnings,
                    duration_seconds: 0.0,
                };
            }
        };

        // Verify each file in index exists and matches checksum
        for (relative_path, expected_checksum) in &index {
            let full_path = data_dir.join(relative_path);

            if !full_path.exists() {
                errors.push(VerificationError {
                    file_path: full_path.clone(),
                    error_type: VerificationErrorType::FileNotFound,
                    message: "File in index not found on disk".to_string(),
                });
                failed_files += 1;
                continue;
            }

            let metadata = match tokio::fs::metadata(&full_path).await {
                Ok(m) => m,
                Err(e) => {
                    errors.push(VerificationError {
                        file_path: full_path.clone(),
                        error_type: VerificationErrorType::PermissionDenied,
                        message: format!("Cannot read metadata: {}", e),
                    });
                    failed_files += 1;
                    continue;
                }
            };

            total_bytes += metadata.len();

            // Verify checksum
            match self.verify_checksum(&full_path, expected_checksum).await {
                Ok(true) => {
                    verified_files += 1;
                }
                Ok(false) => {
                    errors.push(VerificationError {
                        file_path: full_path.clone(),
                        error_type: VerificationErrorType::ChecksumMismatch,
                        message: "Checksum mismatch".to_string(),
                    });
                    failed_files += 1;
                }
                Err(e) => {
                    errors.push(VerificationError {
                        file_path: full_path.clone(),
                        error_type: VerificationErrorType::FileCorrupted,
                        message: format!("Failed to verify: {}", e),
                    });
                    failed_files += 1;
                }
            }
        }

        let duration = start_time.elapsed().as_secs_f64();

        // Check for files not in index
        if self.options.verify_structure {
            let disk_files: std::collections::HashSet<_> = walkdir::WalkDir::new(data_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .collect();

            let index_files: std::collections::HashSet<_> =
                index.keys().map(|k| data_dir.join(k)).collect();

            for file in disk_files.difference(&index_files) {
                warnings.push(format!(
                    "File exists on disk but not in index: {}",
                    file.display()
                ));
            }
        }

        VerificationResult {
            success: errors.is_empty(),
            verified_files,
            failed_files,
            total_bytes,
            errors,
            warnings,
            duration_seconds: duration,
        }
    }
}

/// Convenience function to verify backup integrity
pub async fn verify_backup_integrity(
    backup_path: &Path,
    expected_checksum: Option<&str>,
) -> BackupResult<bool> {
    let verifier = BackupVerifier::new(VerificationOptions::default());
    let result = verifier.verify_file(backup_path, expected_checksum).await;
    Ok(result.success)
}

/// Convenience function to verify backup can be restored
pub async fn verify_backup_restorable(
    backup_path: &Path,
    format: super::CompressionFormat,
    temp_path: &Path,
) -> BackupResult<bool> {
    let verifier = BackupVerifier::new(VerificationOptions {
        test_restore: true,
        ..Default::default()
    });
    let result = verifier.test_restore(backup_path, temp_path, format).await;
    Ok(result.success)
}

/// Create a checksum manifest for a directory
pub async fn create_checksum_manifest(path: &Path) -> BackupResult<HashMap<PathBuf, String>> {
    let mut manifest = HashMap::new();

    let entries = walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    for entry in entries {
        let file_path = entry.path();
        let relative_path = file_path.strip_prefix(path).unwrap_or(file_path);

        let data = tokio::fs::read(file_path).await.map_err(BackupError::Io)?;
        let checksum = blake3::hash(&data).to_hex().to_string();

        manifest.insert(relative_path.to_path_buf(), checksum);
    }

    Ok(manifest)
}

/// Save checksum manifest to file
pub async fn save_checksum_manifest(
    manifest: &HashMap<PathBuf, String>,
    output_path: &Path,
) -> BackupResult<()> {
    let json =
        serde_json::to_string_pretty(manifest).map_err(|e| BackupError::Config(e.to_string()))?;
    tokio::fs::write(output_path, json)
        .await
        .map_err(BackupError::Io)?;
    Ok(())
}

/// Parity verification using Reed-Solomon
pub mod parity {
    use super::*;
    use reed_solomon_erasure::galois_8::ReedSolomon;

    /// Create parity data for a set of chunks
    pub fn create_parity(chunks: &[Vec<u8>], parity_shards: usize) -> BackupResult<Vec<Vec<u8>>> {
        let data_shards = chunks.len();
        let r = ReedSolomon::new(data_shards, parity_shards)
            .map_err(|e| BackupError::Verification(e.to_string()))?;

        // All chunks must be the same size
        let shard_size = chunks[0].len();
        let mut shards: Vec<_> = chunks
            .iter()
            .cloned()
            .chain((0..parity_shards).map(|_| vec![0u8; shard_size]))
            .collect();

        // Convert to slices
        let mut shard_slices: Vec<_> = shards.iter_mut().map(|v| v.as_mut_slice()).collect();

        r.encode(&mut shard_slices)
            .map_err(|e| BackupError::Verification(e.to_string()))?;

        // Extract parity shards
        Ok(shards[data_shards..].to_vec())
    }

    /// Verify and reconstruct missing chunks
    pub fn verify_and_reconstruct(
        _chunks: &mut [Option<Vec<u8>>],
        _parity: &[Vec<u8>],
    ) -> BackupResult<bool> {
        // Simplified implementation - returns false for now
        // Full implementation would use reed-solomon erasure coding
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_verify_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, b"Hello, World!")
            .await
            .unwrap();

        let verifier = BackupVerifier::new(VerificationOptions::default());
        let checksum = blake3::hash(b"Hello, World!").to_hex().to_string();

        let result = verifier.verify_file(&test_file, Some(&checksum)).await;
        assert!(result.success);
        assert_eq!(result.verified_files, 1);

        // Test with wrong checksum
        let result = verifier
            .verify_file(&test_file, Some("wrong_checksum"))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_verify_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        tokio::fs::write(temp_dir.path().join("file1.txt"), b"content1")
            .await
            .unwrap();
        tokio::fs::write(temp_dir.path().join("file2.txt"), b"content2")
            .await
            .unwrap();

        let verifier = BackupVerifier::new(VerificationOptions::default());
        let result = verifier.verify_directory(temp_dir.path(), None).await;

        assert!(result.success);
        assert_eq!(result.verified_files, 2);
    }

    #[tokio::test]
    async fn test_create_checksum_manifest() {
        let temp_dir = TempDir::new().unwrap();

        tokio::fs::write(temp_dir.path().join("file1.txt"), b"content1")
            .await
            .unwrap();
        tokio::fs::write(temp_dir.path().join("file2.txt"), b"content2")
            .await
            .unwrap();

        let manifest = create_checksum_manifest(temp_dir.path()).await.unwrap();

        assert_eq!(manifest.len(), 2);
        assert!(manifest.contains_key(&PathBuf::from("file1.txt")));
        assert!(manifest.contains_key(&PathBuf::from("file2.txt")));
    }

    #[test]
    fn test_parity_verification() {
        // Create 4 data chunks
        let chunks: Vec<Vec<u8>> = (0..4).map(|i| vec![i as u8; 10]).collect();

        // Create 2 parity shards
        let parity = parity::create_parity(&chunks, 2).unwrap();
        assert_eq!(parity.len(), 2);

        // Simulate losing one chunk
        let mut corrupted: Vec<Option<Vec<u8>>> = chunks
            .iter()
            .enumerate()
            .map(|(i, c)| if i == 1 { None } else { Some(c.clone()) })
            .collect();

        // Reconstruct
        let success = parity::verify_and_reconstruct(&mut corrupted, &parity).unwrap();
        assert!(success);
        assert_eq!(corrupted[1], Some(chunks[1].clone()));
    }
}
