//! Backup storage backends (local, S3, GCS, Azure)

use super::{BackupError, BackupResult, CloudCredentials, BandwidthLimit};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a file
    async fn store(&self, key: &str, data: &[u8], metadata: HashMap<String, String>) -> BackupResult<u64>;

    /// Store from a local file
    async fn store_file(&self, key: &str, path: &Path, metadata: HashMap<String, String>) -> BackupResult<u64>;

    /// Retrieve a file
    async fn retrieve(&self, key: &str) -> BackupResult<Vec<u8>>;

    /// Retrieve to a local file
    async fn retrieve_file(&self, key: &str, path: &Path) -> BackupResult<u64>;

    /// Delete a file
    async fn delete(&self, key: &str) -> BackupResult<()>;

    /// List files with prefix
    async fn list(&self, prefix: &str) -> BackupResult<Vec<StorageObject>>;

    /// Check if a file exists
    async fn exists(&self, key: &str) -> BackupResult<bool>;

    /// Get file metadata
    async fn metadata(&self, key: &str) -> BackupResult<StorageObject>;

    /// Get storage statistics
    async fn stats(&self) -> BackupResult<StorageStats>;
}

/// Storage object metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObject {
    pub key: String,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub checksum: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Storage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_objects: u64,
    pub total_bytes: u64,
    pub available_bytes: Option<u64>,
}

/// Local filesystem storage
pub struct LocalStorage {
    base_path: PathBuf,
    bandwidth_limit: BandwidthLimit,
}

impl LocalStorage {
    /// Create new local storage
    pub fn new(base_path: impl AsRef<Path>) -> BackupResult<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_path).map_err(BackupError::Io)?;

        Ok(Self {
            base_path,
            bandwidth_limit: BandwidthLimit::default(),
        })
    }

    /// Set bandwidth limit
    pub fn with_bandwidth_limit(mut self, limit: BandwidthLimit) -> Self {
        self.bandwidth_limit = limit;
        self
    }

    /// Get full path for a key
    fn full_path(&self, key: &str) -> PathBuf {
        // Sanitize key to prevent directory traversal
        let safe_key = key.replace("..", "").replace(':', "_");
        self.base_path.join(safe_key)
    }

    /// Calculate bandwidth-limited chunk size
    fn get_chunk_size(&self) -> usize {
        if self.bandwidth_limit.bytes_per_second == 0 {
            65536 // Default 64KB chunks
        } else {
            let window_size = self.bandwidth_limit.bytes_per_second as usize;
            window_size.min(1048576).max(4096) // Min 4KB, max 1MB
        }
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn store(&self, key: &str, data: &[u8], metadata: HashMap<String, String>) -> BackupResult<u64> {
        let path = self.full_path(key);

        // Create parent directories
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(BackupError::Io)?;
        }

        // Write file with metadata as extended attributes (platform-specific)
        tokio::fs::write(&path, data).await.map_err(BackupError::Io)?;

        // Store metadata in a sidecar file
        if !metadata.is_empty() {
            let meta_path = path.with_extension("meta.json");
            let meta_json = serde_json::to_string(&metadata)
                .map_err(|e| BackupError::Config(e.to_string()))?;
            tokio::fs::write(&meta_path, meta_json).await.map_err(BackupError::Io)?;
        }

        Ok(data.len() as u64)
    }

    async fn store_file(&self, key: &str, source: &Path, metadata: HashMap<String, String>) -> BackupResult<u64> {
        let dest = self.full_path(key);

        // Create parent directories
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(BackupError::Io)?;
        }

        // Copy file with bandwidth limiting
        if self.bandwidth_limit.bytes_per_second > 0 {
            // Throttled copy
            let mut src = tokio::fs::File::open(source).await.map_err(BackupError::Io)?;
            let mut dst = tokio::fs::File::create(&dest).await.map_err(BackupError::Io)?;

            let chunk_size = self.get_chunk_size();
            let mut buffer = vec![0u8; chunk_size];
            let mut total_written = 0u64;

            loop {
                let n = src.read(&mut buffer).await.map_err(BackupError::Io)?;
                if n == 0 {
                    break;
                }
                dst.write_all(&buffer[..n]).await.map_err(BackupError::Io)?;
                total_written += n as u64;

                // Rate limiting
                if self.bandwidth_limit.bytes_per_second > 0 {
                    let delay_ms = (n as u64 * 1000) / self.bandwidth_limit.bytes_per_second;
                    if delay_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }

            dst.flush().await.map_err(BackupError::Io)?;
        } else {
            tokio::fs::copy(source, &dest).await.map_err(BackupError::Io)?;
        }

        // Store metadata
        if !metadata.is_empty() {
            let meta_path = dest.with_extension("meta.json");
            let meta_json = serde_json::to_string(&metadata)
                .map_err(|e| BackupError::Config(e.to_string()))?;
            tokio::fs::write(&meta_path, meta_json).await.map_err(BackupError::Io)?;
        }

        let metadata = tokio::fs::metadata(&dest).await.map_err(BackupError::Io)?;
        Ok(metadata.len())
    }

    async fn retrieve(&self, key: &str) -> BackupResult<Vec<u8>> {
        let path = self.full_path(key);
        tokio::fs::read(&path).await.map_err(BackupError::Io)
    }

    async fn retrieve_file(&self, key: &str, dest: &Path) -> BackupResult<u64> {
        let source = self.full_path(key);
        let bytes = tokio::fs::copy(&source, dest).await.map_err(BackupError::Io)?;
        Ok(bytes)
    }

    async fn delete(&self, key: &str) -> BackupResult<()> {
        let path = self.full_path(key);
        tokio::fs::remove_file(&path).await.map_err(BackupError::Io)?;

        // Also delete metadata file if exists
        let meta_path = path.with_extension("meta.json");
        let _ = tokio::fs::remove_file(&meta_path).await;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> BackupResult<Vec<StorageObject>> {
        let prefix_path = self.full_path(prefix);
        let mut objects = Vec::new();

        if !prefix_path.exists() {
            return Ok(objects);
        }

        let mut entries = tokio::fs::read_dir(&prefix_path).await.map_err(BackupError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(BackupError::Io)? {
            let metadata = entry.metadata().await.map_err(BackupError::Io)?;
            if metadata.is_file() && entry.file_name() != "meta.json" {
                let key = entry.path().strip_prefix(&self.base_path)
                    .map_err(|e| BackupError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?
                    .to_string_lossy()
                    .to_string();

                let modified: DateTime<Utc> = metadata.modified()
                    .map_err(|e| BackupError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?
                    .into();
                let created: DateTime<Utc> = metadata.created()
                    .map_err(|e| BackupError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?
                    .into();

                // Load metadata if exists
                let meta_path = entry.path().with_extension("meta.json");
                let meta = if let Ok(content) = tokio::fs::read_to_string(&meta_path).await {
                    serde_json::from_str(&content).unwrap_or_default()
                } else {
                    HashMap::new()
                };

                objects.push(StorageObject {
                    key,
                    size: metadata.len(),
                    created_at: created,
                    modified_at: modified,
                    checksum: meta.get("checksum").cloned(),
                    metadata: meta,
                });
            }
        }

        Ok(objects)
    }

    async fn exists(&self, key: &str) -> BackupResult<bool> {
        let path = self.full_path(key);
        Ok(path.exists())
    }

    async fn metadata(&self, key: &str) -> BackupResult<StorageObject> {
        let path = self.full_path(key);
        let metadata = tokio::fs::metadata(&path).await.map_err(BackupError::Io)?;

        let modified: DateTime<Utc> = metadata.modified()
            .map_err(|e| BackupError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?
            .into();
        let created: DateTime<Utc> = metadata.created()
            .map_err(|e| BackupError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?
            .into();

        // Load metadata if exists
        let meta_path = path.with_extension("meta.json");
        let meta = if let Ok(content) = tokio::fs::read_to_string(&meta_path).await {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(StorageObject {
            key: key.to_string(),
            size: metadata.len(),
            created_at: created,
            modified_at: modified,
            checksum: meta.get("checksum").cloned(),
            metadata: meta,
        })
    }

    async fn stats(&self) -> BackupResult<StorageStats> {
        let mut total_bytes = 0u64;
        let mut total_objects = 0u64;

        let mut entries = walkdir::WalkDir::new(&self.base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && e.file_name() != "meta.json");

        for entry in &mut entries {
            if let Ok(metadata) = entry.metadata() {
                total_bytes += metadata.len();
                total_objects += 1;
            }
        }

        // Get available space
        let available = if let Ok(info) = fs4::free_space(&self.base_path) {
            Some(info)
        } else {
            None
        };

        Ok(StorageStats {
            total_objects,
            total_bytes,
            available_bytes: available,
        })
    }
}

/// AWS S3 storage
#[cfg(feature = "backup-aws")]
pub struct S3Storage {
    #[cfg(feature = "backup-aws")]
    client: aws_sdk_s3::Client,
    bucket: String,
    prefix: String,
    bandwidth_limit: BandwidthLimit,
}

#[cfg(feature = "backup-aws")]
impl S3Storage {
    /// Create new S3 storage
    pub async fn new(bucket: &str, prefix: &str, _credentials: &CloudCredentials, region: &str) -> BackupResult<Self> {
        // AWS SDK initialization for version 0.35/0.57
        // Use Region from aws_sdk_s3::config which implements ProvideRegion
        let region = aws_sdk_s3::config::Region::new(region.to_string());
        let config = aws_config::from_env()
            .region(region)
            .load()
            .await;

        let client = aws_sdk_s3::Client::new(&config);

        Ok(Self {
            client,
            bucket: bucket.to_string(),
            prefix: prefix.to_string(),
            bandwidth_limit: BandwidthLimit::default(),
        })
    }

    fn make_key(&self, key: &str) -> String {
        format!("{}{}", self.prefix, key)
    }
}

#[cfg(feature = "backup-aws")]
#[async_trait]
impl StorageBackend for S3Storage {
    async fn store(&self, key: &str, data: &[u8], metadata: HashMap<String, String>) -> BackupResult<u64> {
        let s3_key = self.make_key(key);

        let mut builder = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .body(data.to_vec().into());

        // Add metadata
        for (k, v) in metadata {
            builder = builder.metadata(k, v);
        }

        builder.send().await.map_err(|e| BackupError::Cloud(e.to_string()))?;

        Ok(data.len() as u64)
    }

    async fn store_file(&self, key: &str, path: &Path, metadata: HashMap<String, String>) -> BackupResult<u64> {
        let s3_key = self.make_key(key);
        let data = tokio::fs::read(path).await.map_err(BackupError::Io)?;
        self.store(key, &data, metadata).await
    }

    async fn retrieve(&self, key: &str) -> BackupResult<Vec<u8>> {
        let s3_key = self.make_key(key);

        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| BackupError::Cloud(e.to_string()))?;

        let data = response.body.collect().await
            .map_err(|e| BackupError::Cloud(e.to_string()))?;

        Ok(data.to_vec())
    }

    async fn retrieve_file(&self, key: &str, dest: &Path) -> BackupResult<u64> {
        let data = self.retrieve(key).await?;
        tokio::fs::write(dest, &data).await.map_err(BackupError::Io)?;
        Ok(data.len() as u64)
    }

    async fn delete(&self, key: &str) -> BackupResult<()> {
        let s3_key = self.make_key(key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| BackupError::Cloud(e.to_string()))?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> BackupResult<Vec<StorageObject>> {
        let s3_prefix = self.make_key(prefix);

        let response = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&s3_prefix)
            .send()
            .await
            .map_err(|e| BackupError::Cloud(e.to_string()))?;

        let mut objects = Vec::new();

        if let Some(contents) = response.contents {
            for obj in contents {
                let key = obj.key.clone().unwrap_or_default();
                let size = obj.size as u64;
                // Handle AWS SDK DateTime conversion
                let modified = obj.last_modified
                    .and_then(|t| {
                        // AWS SDK 0.28 uses different DateTime format
                        // Convert to chrono DateTime<Utc>
                        let secs = t.as_secs_f64() as i64;
                        DateTime::from_timestamp(secs, 0)
                    })
                    .unwrap_or_else(Utc::now);

                objects.push(StorageObject {
                    key,
                    size,
                    created_at: modified,
                    modified_at: modified,
                    checksum: obj.e_tag,
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(objects)
    }

    async fn exists(&self, key: &str) -> BackupResult<bool> {
        match self.metadata(key).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn metadata(&self, key: &str) -> BackupResult<StorageObject> {
        let s3_key = self.make_key(key);

        let response = self.client
            .head_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| BackupError::Cloud(e.to_string()))?;

        let size = response.content_length as u64;
        // Handle AWS SDK DateTime conversion
        let modified = response.last_modified
            .and_then(|t| {
                let secs = t.as_secs_f64() as i64;
                DateTime::from_timestamp(secs, 0)
            })
            .unwrap_or_else(Utc::now);

        Ok(StorageObject {
            key: key.to_string(),
            size,
            created_at: modified,
            modified_at: modified,
            checksum: response.e_tag,
            metadata: response.metadata.unwrap_or_default(),
        })
    }

    async fn stats(&self) -> BackupResult<StorageStats> {
        let response = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&self.prefix)
            .send()
            .await
            .map_err(|e| BackupError::Cloud(e.to_string()))?;

        let mut total_bytes = 0u64;
        let mut total_objects = 0u64;

        if let Some(contents) = response.contents {
            for obj in contents {
                total_bytes += obj.size as u64;
                total_objects += 1;
            }
        }

        Ok(StorageStats {
            total_objects,
            total_bytes,
            available_bytes: None,
        })
    }
}

/// Cloud storage factory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCredentials {
    pub provider: CloudProvider,
    pub bucket: String,
    pub prefix: String,
    pub region: Option<String>,
    pub credentials: CloudCredentials,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudProvider {
    Aws,
    Gcp,
    Azure,
}

/// Backup storage that can use multiple backends
#[derive(Clone)]
pub struct BackupStorage {
    primary: Arc<dyn StorageBackend>,
    mirrors: Vec<Arc<dyn StorageBackend>>,
}

impl std::fmt::Debug for BackupStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackupStorage")
            .field("mirrors_count", &self.mirrors.len())
            .finish_non_exhaustive()
    }
}

impl BackupStorage {
    /// Create new backup storage with primary backend
    pub fn new(primary: Box<dyn StorageBackend>) -> Self {
        Self {
            primary: Arc::from(primary),
            mirrors: Vec::new(),
        }
    }

    /// Add a mirror storage
    pub fn add_mirror(mut self, mirror: Box<dyn StorageBackend>) -> Self {
        self.mirrors.push(Arc::from(mirror));
        self
    }

    /// Store to all backends
    pub async fn store(&self, key: &str, data: &[u8], metadata: HashMap<String, String>) -> BackupResult<u64> {
        // Store to primary
        let size = self.primary.store(key, data, metadata.clone()).await?;

        // Store to mirrors concurrently
        let mirror_futures: Vec<_> = self.mirrors.iter()
            .map(|m| m.store(key, data, metadata.clone()))
            .collect();

        let results = futures::future::join_all(mirror_futures).await;
        for (i, result) in results.iter().enumerate() {
            if let Err(e) = result {
                tracing::warn!("Mirror {} failed: {}", i, e);
            }
        }

        Ok(size)
    }

    /// Retrieve from primary (or first available mirror)
    pub async fn retrieve(&self, key: &str) -> BackupResult<Vec<u8>> {
        match self.primary.retrieve(key).await {
            Ok(data) => Ok(data),
            Err(_) => {
                // Try mirrors
                for mirror in &self.mirrors {
                    if let Ok(data) = mirror.retrieve(key).await {
                        return Ok(data);
                    }
                }
                Err(BackupError::Storage("Key not found in any storage".to_string()))
            }
        }
    }

    /// Delete from all backends
    pub async fn delete(&self, key: &str) -> BackupResult<()> {
        self.primary.delete(key).await?;

        for mirror in &self.mirrors {
            let _ = mirror.delete(key).await;
        }

        Ok(())
    }

    /// List objects from primary storage
    pub async fn list(&self, prefix: &str) -> BackupResult<Vec<StorageObject>> {
        self.primary.list(prefix).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path()).unwrap();

        // Test store
        let data = b"Hello, World!";
        let metadata = HashMap::new();
        let size = storage.store("test.txt", data, metadata).await.unwrap();
        assert_eq!(size, data.len() as u64);

        // Test exists
        assert!(storage.exists("test.txt").await.unwrap());

        // Test retrieve
        let retrieved = storage.retrieve("test.txt").await.unwrap();
        assert_eq!(retrieved, data);

        // Test delete
        storage.delete("test.txt").await.unwrap();
        assert!(!storage.exists("test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_local_storage_list() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path()).unwrap();

        // Store multiple files
        storage.store("file1.txt", b"content1", HashMap::new()).await.unwrap();
        storage.store("file2.txt", b"content2", HashMap::new()).await.unwrap();
        tokio::fs::create_dir_all(temp_dir.path().join("subdir")).await.unwrap();
        storage.store("subdir/file3.txt", b"content3", HashMap::new()).await.unwrap();

        // List all
        let objects = storage.list("").await.unwrap();
        assert_eq!(objects.len(), 2); // file1 and file2 only

        // List subdirectory
        let objects = storage.list("subdir/").await.unwrap();
        assert_eq!(objects.len(), 1);
    }
}
