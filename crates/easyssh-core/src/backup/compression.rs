//! Backup compression and encryption

use super::{
    BackupError, BackupResult, CompressionFormat, EncryptionAlgorithm, EncryptionSettings,
};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use rand::RngCore;
use std::io::{Read, Write};
use std::path::Path;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use zstd::stream::{read::Decoder as ZstdDecoder, write::Encoder as ZstdEncoder};

/// Compress data using the specified format
pub fn compress_data(data: &[u8], format: CompressionFormat, level: u32) -> BackupResult<Vec<u8>> {
    let level = level.clamp(1, 9);

    match format {
        CompressionFormat::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::new(level));
            encoder
                .write_all(data)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| BackupError::Compression(e.to_string()))
        }
        CompressionFormat::Zstd => {
            let mut encoder = ZstdEncoder::new(Vec::new(), level as i32)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            encoder
                .write_all(data)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| BackupError::Compression(e.to_string()))
        }
        CompressionFormat::Bzip2 => {
            let mut encoder =
                bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::new(level));
            encoder
                .write_all(data)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| BackupError::Compression(e.to_string()))
        }
        CompressionFormat::Xz => {
            let mut encoder = xz2::write::XzEncoder::new(Vec::new(), level as u32);
            encoder
                .write_all(data)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| BackupError::Compression(e.to_string()))
        }
        CompressionFormat::Tar | CompressionFormat::Zip => {
            // Tar and Zip need directory context, just pass through for now
            Ok(data.to_vec())
        }
    }
}

/// Decompress data using the specified format
pub fn decompress_data(data: &[u8], format: CompressionFormat) -> BackupResult<Vec<u8>> {
    match format {
        CompressionFormat::Gzip => {
            let mut decoder = GzDecoder::new(data);
            let mut result = Vec::new();
            decoder
                .read_to_end(&mut result)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            Ok(result)
        }
        CompressionFormat::Zstd => {
            let mut decoder =
                ZstdDecoder::new(data).map_err(|e| BackupError::Compression(e.to_string()))?;
            let mut result = Vec::new();
            decoder
                .read_to_end(&mut result)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            Ok(result)
        }
        CompressionFormat::Bzip2 => {
            let mut decoder = bzip2::read::BzDecoder::new(data);
            let mut result = Vec::new();
            decoder
                .read_to_end(&mut result)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            Ok(result)
        }
        CompressionFormat::Xz => {
            let mut decoder = xz2::read::XzDecoder::new(data);
            let mut result = Vec::new();
            decoder
                .read_to_end(&mut result)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            Ok(result)
        }
        CompressionFormat::Tar | CompressionFormat::Zip => {
            // Tar and Zip need directory context, just pass through for now
            Ok(data.to_vec())
        }
    }
}

/// Compress a directory into an archive
pub async fn compress_directory(
    source: &Path,
    output: &Path,
    format: CompressionFormat,
    level: u32,
) -> BackupResult<u64> {
    let file = std::fs::File::create(output).map_err(BackupError::Io)?;

    match format {
        CompressionFormat::Tar => {
            let mut builder = tar::Builder::new(file);
            builder
                .append_dir_all(".", source)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
            builder
                .finish()
                .map_err(|e| BackupError::Compression(e.to_string()))?;
        }
        CompressionFormat::Zip => {
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(Some(level.clamp(1, 9) as i64));

            let walker = walkdir::WalkDir::new(source);
            for entry in walker {
                let entry = entry.map_err(|e| BackupError::Compression(e.to_string()))?;
                let path = entry.path();
                let relative_path = path.strip_prefix(source).unwrap_or(path);

                if path.is_file() {
                    zip.start_file_from_path(relative_path, options)
                        .map_err(|e| BackupError::Compression(e.to_string()))?;
                    let contents = tokio::fs::read(path).await.map_err(BackupError::Io)?;
                    zip.write_all(&contents)
                        .map_err(|e| BackupError::Compression(e.to_string()))?;
                } else if path.is_dir() && relative_path.as_os_str().len() > 0 {
                    zip.add_directory_from_path(relative_path, options)
                        .map_err(|e| BackupError::Compression(e.to_string()))?;
                }
            }
            zip.finish()
                .map_err(|e| BackupError::Compression(e.to_string()))?;
        }
        _ => {
            // For other formats, create tar first then compress
            let temp_tar = tempfile::NamedTempFile::new().map_err(BackupError::Io)?;
            {
                let mut builder = tar::Builder::new(temp_tar.as_file());
                builder
                    .append_dir_all(".", source)
                    .map_err(|e| BackupError::Compression(e.to_string()))?;
                builder
                    .finish()
                    .map_err(|e| BackupError::Compression(e.to_string()))?;
            }

            let tar_data = tokio::fs::read(temp_tar.path())
                .await
                .map_err(BackupError::Io)?;
            let compressed = compress_data(&tar_data, format, level)?;
            tokio::fs::write(output, compressed)
                .await
                .map_err(BackupError::Io)?;
        }
    }

    let metadata = std::fs::metadata(output).map_err(BackupError::Io)?;
    Ok(metadata.len())
}

/// Decompress an archive to a directory
pub async fn decompress_archive(
    source: &Path,
    output: &Path,
    format: CompressionFormat,
) -> BackupResult<()> {
    tokio::fs::create_dir_all(output)
        .await
        .map_err(BackupError::Io)?;

    match format {
        CompressionFormat::Tar => {
            let file = std::fs::File::open(source).map_err(BackupError::Io)?;
            let mut archive = tar::Archive::new(file);
            archive
                .unpack(output)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
        }
        CompressionFormat::Zip => {
            let file = std::fs::File::open(source).map_err(BackupError::Io)?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| BackupError::Compression(e.to_string()))?;

            for i in 0..archive.len() {
                let mut file = archive
                    .by_index(i)
                    .map_err(|e| BackupError::Compression(e.to_string()))?;
                let outpath = match file.enclosed_name() {
                    Some(path) => output.join(path),
                    None => continue,
                };

                if file.name().ends_with('/') {
                    tokio::fs::create_dir_all(&outpath)
                        .await
                        .map_err(BackupError::Io)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        tokio::fs::create_dir_all(p)
                            .await
                            .map_err(BackupError::Io)?;
                    }
                    let mut outfile = tokio::fs::File::create(&outpath)
                        .await
                        .map_err(BackupError::Io)?;
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer)
                        .map_err(|e| BackupError::Compression(e.to_string()))?;
                    outfile.write_all(&buffer).await.map_err(BackupError::Io)?;
                }
            }
        }
        _ => {
            // For compressed tar, decompress first
            let compressed_data = tokio::fs::read(source).await.map_err(BackupError::Io)?;
            let tar_data = decompress_data(&compressed_data, format)?;

            let temp_tar = tempfile::NamedTempFile::new().map_err(BackupError::Io)?;
            tokio::fs::write(temp_tar.path(), tar_data)
                .await
                .map_err(BackupError::Io)?;

            let file = std::fs::File::open(temp_tar.path()).map_err(BackupError::Io)?;
            let mut archive = tar::Archive::new(file);
            archive
                .unpack(output)
                .map_err(|e| BackupError::Compression(e.to_string()))?;
        }
    }

    Ok(())
}

/// Compress a backup file
pub async fn compress_backup(
    source: &Path,
    output: &Path,
    format: CompressionFormat,
    level: u32,
) -> BackupResult<u64> {
    if source.is_dir() {
        compress_directory(source, output, format, level).await
    } else {
        let data = tokio::fs::read(source).await.map_err(BackupError::Io)?;
        let compressed = compress_data(&data, format, level)?;
        let len = compressed.len() as u64;
        tokio::fs::write(output, compressed)
            .await
            .map_err(BackupError::Io)?;
        Ok(len)
    }
}

/// Decompress a backup file
pub async fn decompress_backup(
    source: &Path,
    output: &Path,
    format: CompressionFormat,
) -> BackupResult<()> {
    if format == CompressionFormat::Tar
        || format == CompressionFormat::Zip
        || source.extension().and_then(|s| s.to_str()) == Some("tar")
    {
        decompress_archive(source, output, format).await
    } else {
        let compressed_data = tokio::fs::read(source).await.map_err(BackupError::Io)?;
        let decompressed = decompress_data(&compressed_data, format)?;
        tokio::fs::write(output, decompressed)
            .await
            .map_err(BackupError::Io)?;
        Ok(())
    }
}

/// Derive encryption key from password
pub fn derive_key(password: &str, salt: &[u8], algorithm: EncryptionAlgorithm) -> Vec<u8> {
    match algorithm {
        EncryptionAlgorithm::Aes256Gcm => {
            use argon2::password_hash::SaltString;
            use argon2::{Argon2, PasswordHasher};

            let argon2 = Argon2::default();
            let salt_string = SaltString::encode_b64(salt)
                .unwrap_or_else(|_| SaltString::from_b64("AAAAAAAA").unwrap());
            let password_hash = argon2
                .hash_password(password.as_bytes(), &salt_string)
                .expect("Failed to hash password");

            password_hash.hash.unwrap().as_bytes()[..32].to_vec()
        }
        EncryptionAlgorithm::ChaCha20Poly1305 => {
            // Same key derivation
            use argon2::password_hash::SaltString;
            use argon2::{Argon2, PasswordHasher};

            let argon2 = Argon2::default();
            let salt_string = SaltString::encode_b64(salt)
                .unwrap_or_else(|_| SaltString::from_b64("AAAAAAAA").unwrap());
            let password_hash = argon2
                .hash_password(password.as_bytes(), &salt_string)
                .expect("Failed to hash password");

            password_hash.hash.unwrap().as_bytes()[..32].to_vec()
        }
    }
}

/// Encrypt data with AES-256-GCM
pub fn encrypt_data(
    data: &[u8],
    key: &[u8],
    algorithm: EncryptionAlgorithm,
) -> BackupResult<Vec<u8>> {
    match algorithm {
        EncryptionAlgorithm::Aes256Gcm => {
            let cipher = Aes256Gcm::new_from_slice(key)
                .map_err(|e| BackupError::Encryption(e.to_string()))?;

            let mut nonce_bytes = [0u8; 12];
            rand::thread_rng().fill_bytes(&mut nonce_bytes);
            let nonce = Nonce::from_slice(&nonce_bytes);

            let ciphertext = cipher
                .encrypt(nonce, data)
                .map_err(|e| BackupError::Encryption(e.to_string()))?;

            let mut result = Vec::with_capacity(12 + ciphertext.len());
            result.extend_from_slice(&nonce_bytes);
            result.extend_from_slice(&ciphertext);

            Ok(result)
        }
        EncryptionAlgorithm::ChaCha20Poly1305 => {
            use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

            let key = Key::from_slice(key);
            let cipher = ChaCha20Poly1305::new(key);

            let mut nonce_bytes = [0u8; 12];
            rand::thread_rng().fill_bytes(&mut nonce_bytes);
            let nonce = Nonce::from_slice(&nonce_bytes);

            let ciphertext = cipher
                .encrypt(nonce, data)
                .map_err(|e| BackupError::Encryption(e.to_string()))?;

            let mut result = Vec::with_capacity(12 + ciphertext.len());
            result.extend_from_slice(&nonce_bytes);
            result.extend_from_slice(&ciphertext);

            Ok(result)
        }
    }
}

/// Decrypt data with AES-256-GCM
pub fn decrypt_data(
    encrypted_data: &[u8],
    key: &[u8],
    algorithm: EncryptionAlgorithm,
) -> BackupResult<Vec<u8>> {
    if encrypted_data.len() < 12 {
        return Err(BackupError::Encryption(
            "Invalid encrypted data".to_string(),
        ));
    }

    let nonce_bytes = &encrypted_data[..12];
    let ciphertext = &encrypted_data[12..];

    match algorithm {
        EncryptionAlgorithm::Aes256Gcm => {
            let cipher = Aes256Gcm::new_from_slice(key)
                .map_err(|e| BackupError::Encryption(e.to_string()))?;

            let nonce = Nonce::from_slice(nonce_bytes);
            let plaintext = cipher
                .decrypt(nonce, ciphertext)
                .map_err(|e| BackupError::Encryption(e.to_string()))?;

            Ok(plaintext)
        }
        EncryptionAlgorithm::ChaCha20Poly1305 => {
            use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

            let key = Key::from_slice(key);
            let cipher = ChaCha20Poly1305::new(key);

            let nonce = Nonce::from_slice(nonce_bytes);
            let plaintext = cipher
                .decrypt(nonce, ciphertext)
                .map_err(|e| BackupError::Encryption(e.to_string()))?;

            Ok(plaintext)
        }
    }
}

/// Encrypt a backup file
pub async fn encrypt_backup(
    source: &Path,
    output: &Path,
    password: &str,
    settings: &EncryptionSettings,
) -> BackupResult<u64> {
    let data = tokio::fs::read(source).await.map_err(BackupError::Io)?;

    // Generate random salt
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);

    // Derive key
    let key = derive_key(password, &salt, settings.algorithm);

    // Encrypt data
    let encrypted = encrypt_data(&data, &key, settings.algorithm)?;

    // Write salt + encrypted data
    let mut result = Vec::with_capacity(16 + encrypted.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&encrypted);

    tokio::fs::write(output, result)
        .await
        .map_err(BackupError::Io)?;

    let metadata = std::fs::metadata(output).map_err(BackupError::Io)?;
    Ok(metadata.len())
}

/// Decrypt a backup file
pub async fn decrypt_backup(
    source: &Path,
    output: &Path,
    password: &str,
    algorithm: EncryptionAlgorithm,
) -> BackupResult<u64> {
    let data = tokio::fs::read(source).await.map_err(BackupError::Io)?;

    if data.len() < 16 {
        return Err(BackupError::Encryption(
            "Invalid encrypted file".to_string(),
        ));
    }

    // Extract salt
    let salt = &data[..16];
    let encrypted = &data[16..];

    // Derive key
    let key = derive_key(password, salt, algorithm);

    // Decrypt data
    let decrypted = decrypt_data(encrypted, &key, algorithm)?;

    tokio::fs::write(output, decrypted)
        .await
        .map_err(BackupError::Io)?;

    Ok(encrypted.len() as u64)
}

/// Encrypt and compress a backup in one pass
pub async fn compress_and_encrypt(
    source: &Path,
    output: &Path,
    format: CompressionFormat,
    level: u32,
    password: &str,
    settings: &EncryptionSettings,
) -> BackupResult<u64> {
    // Create temp file for compressed data
    let temp_file = tempfile::NamedTempFile::new().map_err(BackupError::Io)?;

    // Compress first
    compress_backup(source, temp_file.path(), format, level).await?;

    // Then encrypt
    let size = encrypt_backup(temp_file.path(), output, password, settings).await?;

    Ok(size)
}

/// Decrypt and decompress a backup in one pass
pub async fn decrypt_and_decompress(
    source: &Path,
    output: &Path,
    format: CompressionFormat,
    password: &str,
    algorithm: EncryptionAlgorithm,
) -> BackupResult<u64> {
    // Create temp file for decrypted data
    let temp_file = tempfile::NamedTempFile::new().map_err(BackupError::Io)?;

    // Decrypt first
    decrypt_backup(source, temp_file.path(), password, algorithm).await?;

    // Then decompress
    decompress_backup(temp_file.path(), output, format).await?;

    let metadata = std::fs::metadata(output).map_err(BackupError::Io)?;
    Ok(metadata.len())
}

/// Streaming compressor for large files
pub struct StreamingCompressor<R: AsyncRead> {
    reader: R,
    format: CompressionFormat,
    level: u32,
}

impl<R: AsyncRead + Unpin> StreamingCompressor<R> {
    pub fn new(reader: R, format: CompressionFormat, level: u32) -> Self {
        Self {
            reader,
            format,
            level: level.clamp(1, 9),
        }
    }

    pub async fn compress_to<W: AsyncWrite + Unpin>(
        &mut self,
        writer: &mut W,
    ) -> BackupResult<u64> {
        let mut buffer = vec![0u8; 65536];
        let mut total_written = 0u64;

        // For streaming, we'll read all and compress (simplified)
        let mut data = Vec::new();
        loop {
            let n = self
                .reader
                .read(&mut buffer)
                .await
                .map_err(BackupError::Io)?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buffer[..n]);
        }

        let compressed = compress_data(&data, self.format, self.level)?;
        writer
            .write_all(&compressed)
            .await
            .map_err(BackupError::Io)?;
        total_written += compressed.len() as u64;

        Ok(total_written)
    }
}

/// Calculate compression ratio
pub fn compression_ratio(original_size: u64, compressed_size: u64) -> f64 {
    if original_size == 0 {
        1.0
    } else {
        compressed_size as f64 / original_size as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_compress_decompress_data() {
        let original = b"Hello, World! This is test data for compression.".to_vec();
        let formats = vec![
            CompressionFormat::Gzip,
            CompressionFormat::Zstd,
            CompressionFormat::Bzip2,
        ];

        for format in formats {
            let compressed = compress_data(&original, format, 6).unwrap();
            let decompressed = decompress_data(&compressed, format).unwrap();
            assert_eq!(original, decompressed);
        }
    }

    #[tokio::test]
    async fn test_encrypt_decrypt() {
        let original = b"Secret backup data".to_vec();
        let password = "test_password";
        let salt = b"random_salt_here";

        let key = derive_key(password, salt, EncryptionAlgorithm::Aes256Gcm);
        let encrypted = encrypt_data(&original, &key, EncryptionAlgorithm::Aes256Gcm).unwrap();
        let decrypted = decrypt_data(&encrypted, &key, EncryptionAlgorithm::Aes256Gcm).unwrap();

        assert_eq!(original, decrypted);
    }

    #[tokio::test]
    async fn test_compress_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_file = temp_dir.path().join("output.tar.gz");

        // Create test directory structure
        tokio::fs::create_dir_all(&source_dir).await.unwrap();
        tokio::fs::write(source_dir.join("file1.txt"), "content1")
            .await
            .unwrap();
        tokio::fs::write(source_dir.join("file2.txt"), "content2")
            .await
            .unwrap();
        tokio::fs::create_dir_all(source_dir.join("subdir"))
            .await
            .unwrap();
        tokio::fs::write(source_dir.join("subdir/file3.txt"), "content3")
            .await
            .unwrap();

        // Compress
        let size = compress_directory(&source_dir, &output_file, CompressionFormat::Gzip, 6)
            .await
            .unwrap();
        assert!(size > 0);
        assert!(output_file.exists());
    }
}
