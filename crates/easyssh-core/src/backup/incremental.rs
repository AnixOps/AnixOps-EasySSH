//! Incremental backup with deduplication

use super::{BackupError, BackupResult, SnapshotId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// File hash for deduplication
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileHash {
    /// BLAKE3 hash of file content
    pub hash: String,
    /// File size in bytes
    pub size: u64,
    /// Last modified time
    pub modified_time: DateTime<Utc>,
}

impl FileHash {
    /// Create from file path
    pub async fn from_file(path: &Path) -> BackupResult<Self> {
        let metadata = tokio::fs::metadata(path).await.map_err(BackupError::Io)?;
        let modified: DateTime<Utc> = metadata
            .modified()
            .map_err(|e| BackupError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?
            .into();

        // Calculate hash
        let data = tokio::fs::read(path).await.map_err(BackupError::Io)?;
        let hash = blake3::hash(&data).to_hex().to_string();

        Ok(Self {
            hash,
            size: metadata.len(),
            modified_time: modified,
        })
    }

    /// Create from data
    pub fn from_data(data: &[u8]) -> Self {
        let hash = blake3::hash(data).to_hex().to_string();
        Self {
            hash,
            size: data.len() as u64,
            modified_time: Utc::now(),
        }
    }

    /// Check if two files are identical
    pub fn is_identical(&self, other: &FileHash) -> bool {
        self.hash == other.hash && self.size == other.size
    }
}

/// File entry in the incremental index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path from backup root
    pub relative_path: PathBuf,
    /// File hash
    pub hash: FileHash,
    /// File permissions (Unix mode)
    pub permissions: u32,
    /// Is a directory
    pub is_directory: bool,
    /// Is a symlink
    pub is_symlink: bool,
    /// Symlink target (if applicable)
    pub symlink_target: Option<PathBuf>,
    /// Extended attributes (platform-specific)
    pub extended_attrs: HashMap<String, Vec<u8>>,
}

/// Incremental backup index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalIndex {
    pub snapshot_id: SnapshotId,
    pub parent_id: Option<SnapshotId>,
    pub created_at: DateTime<Utc>,
    pub files: HashMap<PathBuf, FileEntry>,
    pub total_size: u64,
    pub file_count: u64,
    pub directory_count: u64,
}

impl IncrementalIndex {
    /// Create a new incremental index
    pub fn new(snapshot_id: SnapshotId, parent_id: Option<SnapshotId>) -> Self {
        Self {
            snapshot_id,
            parent_id,
            created_at: Utc::now(),
            files: HashMap::new(),
            total_size: 0,
            file_count: 0,
            directory_count: 0,
        }
    }

    /// Add a file entry
    pub fn add_file(&mut self, entry: FileEntry) {
        self.total_size += entry.hash.size;
        if entry.is_directory {
            self.directory_count += 1;
        } else {
            self.file_count += 1;
        }
        self.files.insert(entry.relative_path.clone(), entry);
    }

    /// Get a file entry
    pub fn get_file(&self, path: &Path) -> Option<&FileEntry> {
        self.files.get(path)
    }

    /// Check if a file has changed compared to another index
    pub fn has_changed(&self, path: &Path, hash: &FileHash) -> bool {
        match self.get_file(path) {
            Some(entry) => !entry.hash.is_identical(hash),
            None => true,
        }
    }

    /// Calculate the difference between this index and another
    pub fn diff(&self, other: &IncrementalIndex) -> IndexDiff {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();
        let mut unchanged = Vec::new();

        // Find added and modified files
        for (path, entry) in &self.files {
            if let Some(other_entry) = other.get_file(path) {
                if entry.hash.is_identical(&other_entry.hash) {
                    unchanged.push(path.clone());
                } else {
                    modified.push(path.clone());
                }
            } else {
                added.push(path.clone());
            }
        }

        // Find deleted files
        for path in other.files.keys() {
            if !self.files.contains_key(path) {
                deleted.push(path.clone());
            }
        }

        IndexDiff {
            added,
            modified,
            deleted,
            unchanged,
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> BackupResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| BackupError::Config(e.to_string()))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> BackupResult<Self> {
        serde_json::from_str(json).map_err(|e| BackupError::Config(e.to_string()))
    }

    /// Get total size in a human-readable format
    pub fn size_human(&self) -> String {
        crate::backup::format_bytes(self.total_size)
    }
}

/// Difference between two index snapshots
#[derive(Debug, Clone, Default)]
pub struct IndexDiff {
    pub added: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub deleted: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
}

impl IndexDiff {
    /// Get total changed files count
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.modified.is_empty() || !self.deleted.is_empty()
    }
}

/// Incremental backup manager
pub struct IncrementalBackupManager {
    /// Base directory for storing indices
    index_dir: PathBuf,
    /// Cache of loaded indices
    cache: HashMap<SnapshotId, IncrementalIndex>,
}

impl IncrementalBackupManager {
    /// Create a new incremental backup manager
    pub fn new(index_dir: impl AsRef<Path>) -> BackupResult<Self> {
        let index_dir = index_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&index_dir).map_err(BackupError::Io)?;

        Ok(Self {
            index_dir,
            cache: HashMap::new(),
        })
    }

    /// Get the path for an index file
    fn index_path(&self, snapshot_id: SnapshotId) -> PathBuf {
        self.index_dir.join(format!("{}.json", snapshot_id.0))
    }

    /// Build an index from a directory
    pub async fn build_index(
        &self,
        source: &Path,
        snapshot_id: SnapshotId,
        parent_id: Option<SnapshotId>,
        filter: &super::BackupFilter,
    ) -> BackupResult<IncrementalIndex> {
        let mut index = IncrementalIndex::new(snapshot_id, parent_id);

        let base_path = source.to_path_buf();

        let entries = walkdir::WalkDir::new(source)
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in entries {
            let path = entry.path();
            let relative_path = path.strip_prefix(&base_path).unwrap_or(path);

            let metadata = entry.metadata().map_err(|e| BackupError::Io(e.into()))?;

            // Check filter
            if !filter.should_include(path, &metadata) && !relative_path.as_os_str().is_empty() {
                continue;
            }

            let is_directory = metadata.is_dir();
            let is_symlink = metadata.file_type().is_symlink();

            // For directories, just record metadata
            if is_directory {
                let file_entry = FileEntry {
                    relative_path: relative_path.to_path_buf(),
                    hash: FileHash {
                        hash: String::new(),
                        size: 0,
                        modified_time: Utc::now(),
                    },
                    #[cfg(unix)]
                    permissions: std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()),
                    #[cfg(not(unix))]
                    permissions: 0o644, // Default for non-Unix systems
                    is_directory: true,
                    is_symlink: false,
                    symlink_target: None,
                    extended_attrs: HashMap::new(),
                };
                index.add_file(file_entry);
                continue;
            }

            // For symlinks, record target
            let symlink_target = if is_symlink {
                tokio::fs::read_link(path).await.ok()
            } else {
                None
            };

            // Calculate hash for files
            let hash = if is_symlink {
                // Hash the symlink target path
                let target = symlink_target
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                FileHash::from_data(target.as_bytes())
            } else {
                FileHash::from_file(path).await?
            };

            let file_entry = FileEntry {
                relative_path: relative_path.to_path_buf(),
                hash,
                #[cfg(unix)]
                permissions: std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()),
                #[cfg(not(unix))]
                permissions: 0o644,
                is_directory: false,
                is_symlink,
                symlink_target,
                extended_attrs: HashMap::new(),
            };

            index.add_file(file_entry);
        }

        Ok(index)
    }

    /// Build an incremental index against a parent
    pub async fn build_incremental_index(
        &self,
        source: &Path,
        snapshot_id: SnapshotId,
        parent_id: SnapshotId,
        filter: &super::BackupFilter,
    ) -> BackupResult<(IncrementalIndex, IndexDiff)> {
        // Load parent index
        let parent_index = self.load_index(parent_id).await?;

        // Build new index
        let new_index = self
            .build_index(source, snapshot_id, Some(parent_id), filter)
            .await?;

        // Calculate diff
        let diff = new_index.diff(&parent_index);

        Ok((new_index, diff))
    }

    /// Save an index to disk
    pub async fn save_index(&mut self, index: &IncrementalIndex) -> BackupResult<()> {
        let path = self.index_path(index.snapshot_id);
        let json = index.to_json()?;
        tokio::fs::write(&path, json)
            .await
            .map_err(BackupError::Io)?;

        // Update cache
        self.cache.insert(index.snapshot_id, index.clone());

        Ok(())
    }

    /// Load an index from disk
    pub async fn load_index(&self, snapshot_id: SnapshotId) -> BackupResult<IncrementalIndex> {
        // Check cache first
        if let Some(index) = self.cache.get(&snapshot_id) {
            return Ok(index.clone());
        }

        let path = self.index_path(snapshot_id);
        let json = tokio::fs::read_to_string(&path)
            .await
            .map_err(BackupError::Io)?;
        let index = IncrementalIndex::from_json(&json)?;

        Ok(index)
    }

    /// Get the files that need to be backed up incrementally
    pub fn get_changed_files(
        &self,
        current_index: &IncrementalIndex,
        parent_index: Option<&IncrementalIndex>,
    ) -> Vec<PathBuf> {
        match parent_index {
            Some(parent) => {
                let diff = current_index.diff(parent);
                let mut changed: Vec<PathBuf> = Vec::new();
                changed.extend(diff.added);
                changed.extend(diff.modified);
                changed
            }
            None => current_index.files.keys().cloned().collect(),
        }
    }

    /// List all stored indices
    pub async fn list_indices(&self) -> BackupResult<Vec<SnapshotId>> {
        let mut entries = tokio::fs::read_dir(&self.index_dir)
            .await
            .map_err(BackupError::Io)?;
        let mut indices = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(BackupError::Io)? {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            if file_name.ends_with(".json") {
                if let Ok(uuid) = uuid::Uuid::parse_str(&file_name[..file_name.len() - 5]) {
                    indices.push(SnapshotId(uuid));
                }
            }
        }

        Ok(indices)
    }

    /// Delete an index
    pub async fn delete_index(&mut self, snapshot_id: SnapshotId) -> BackupResult<()> {
        let path = self.index_path(snapshot_id);
        tokio::fs::remove_file(&path)
            .await
            .map_err(BackupError::Io)?;
        self.cache.remove(&snapshot_id);
        Ok(())
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get statistics about the index store
    pub async fn stats(&self) -> BackupResult<IndexStoreStats> {
        let indices = self.list_indices().await?;
        let mut total_size = 0u64;

        for snapshot_id in &indices {
            let path = self.index_path(*snapshot_id);
            if let Ok(metadata) = tokio::fs::metadata(&path).await {
                total_size += metadata.len();
            }
        }

        Ok(IndexStoreStats {
            index_count: indices.len() as u64,
            total_size,
        })
    }
}

/// Statistics about the index store
#[derive(Debug, Clone)]
pub struct IndexStoreStats {
    pub index_count: u64,
    pub total_size: u64,
}

/// Chunk-based deduplication for large files
#[derive(Debug, Clone)]
pub struct ChunkDeduplicator {
    /// Chunk size (default 4MB)
    chunk_size: usize,
    /// Hash to chunk ID mapping
    chunk_map: HashMap<String, ChunkId>,
    /// Next chunk ID
    next_chunk_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(u64);

/// Chunk information
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: ChunkId,
    pub hash: String,
    pub size: u64,
    pub data: Vec<u8>,
}

impl ChunkDeduplicator {
    /// Create a new chunk deduplicator
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            chunk_map: HashMap::new(),
            next_chunk_id: 0,
        }
    }

    /// Split data into chunks
    pub fn split_into_chunks(&mut self, data: &[u8]) -> (Vec<ChunkId>, Vec<Chunk>) {
        let mut chunk_ids = Vec::new();
        let mut new_chunks = Vec::new();

        for chunk_data in data.chunks(self.chunk_size) {
            let hash = blake3::hash(chunk_data).to_hex().to_string();

            if let Some(&existing_id) = self.chunk_map.get(&hash) {
                chunk_ids.push(existing_id);
            } else {
                let id = ChunkId(self.next_chunk_id);
                self.next_chunk_id += 1;
                self.chunk_map.insert(hash.clone(), id);

                let chunk = Chunk {
                    id,
                    hash: hash.clone(),
                    size: chunk_data.len() as u64,
                    data: chunk_data.to_vec(),
                };

                chunk_ids.push(id);
                new_chunks.push(chunk);
            }
        }

        (chunk_ids, new_chunks)
    }

    /// Get deduplication ratio
    pub fn dedup_ratio(&self, total_data_size: u64) -> f64 {
        let unique_size: u64 = self.chunk_map.len() as u64 * self.chunk_size as u64;
        if total_data_size == 0 {
            1.0
        } else {
            unique_size as f64 / total_data_size as f64
        }
    }
}

/// File manifest for chunked storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkedFileManifest {
    pub path: PathBuf,
    pub total_size: u64,
    pub chunks: Vec<ChunkId>,
    pub chunk_hashes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, b"Hello, World!")
            .await
            .unwrap();

        let hash = FileHash::from_file(&file_path).await.unwrap();
        assert_eq!(hash.size, 13);
        assert!(!hash.hash.is_empty());
    }

    #[tokio::test]
    async fn test_incremental_index() {
        let temp_dir = TempDir::new().unwrap();
        let manager = IncrementalBackupManager::new(temp_dir.path()).unwrap();

        // Create test directory structure
        let source_dir = temp_dir.path().join("source");
        tokio::fs::create_dir_all(&source_dir).await.unwrap();
        tokio::fs::write(source_dir.join("file1.txt"), b"content1")
            .await
            .unwrap();
        tokio::fs::write(source_dir.join("file2.txt"), b"content2")
            .await
            .unwrap();
        tokio::fs::create_dir_all(source_dir.join("subdir"))
            .await
            .unwrap();
        tokio::fs::write(source_dir.join("subdir/file3.txt"), b"content3")
            .await
            .unwrap();

        // Build index
        let snapshot_id = SnapshotId::new();
        let filter = super::super::BackupFilter::default();
        let index = manager
            .build_index(&source_dir, snapshot_id, None, &filter)
            .await
            .unwrap();

        assert_eq!(index.file_count, 3); // file1, file2, file3
        assert_eq!(index.directory_count, 1); // subdir
        assert!(index.total_size > 0);
    }

    #[tokio::test]
    async fn test_index_diff() {
        let temp_dir = TempDir::new().unwrap();

        // Create first index
        let mut index1 = IncrementalIndex::new(SnapshotId::new(), None);
        index1.add_file(FileEntry {
            relative_path: PathBuf::from("file1.txt"),
            hash: FileHash::from_data(b"content1"),
            permissions: 0o644,
            is_directory: false,
            is_symlink: false,
            symlink_target: None,
            extended_attrs: HashMap::new(),
        });
        index1.add_file(FileEntry {
            relative_path: PathBuf::from("file2.txt"),
            hash: FileHash::from_data(b"content2"),
            permissions: 0o644,
            is_directory: false,
            is_symlink: false,
            symlink_target: None,
            extended_attrs: HashMap::new(),
        });

        // Create second index with changes
        let mut index2 = IncrementalIndex::new(SnapshotId::new(), None);
        index2.add_file(FileEntry {
            relative_path: PathBuf::from("file1.txt"),
            hash: FileHash::from_data(b"modified_content1"), // Modified
            permissions: 0o644,
            is_directory: false,
            is_symlink: false,
            symlink_target: None,
            extended_attrs: HashMap::new(),
        });
        // file2 deleted
        index2.add_file(FileEntry {
            relative_path: PathBuf::from("file3.txt"), // Added
            hash: FileHash::from_data(b"content3"),
            permissions: 0o644,
            is_directory: false,
            is_symlink: false,
            symlink_target: None,
            extended_attrs: HashMap::new(),
        });

        let diff = index2.diff(&index1);

        assert_eq!(diff.added.len(), 1);
        assert!(diff.added.contains(&PathBuf::from("file3.txt")));

        assert_eq!(diff.modified.len(), 1);
        assert!(diff.modified.contains(&PathBuf::from("file1.txt")));

        assert_eq!(diff.deleted.len(), 1);
        assert!(diff.deleted.contains(&PathBuf::from("file2.txt")));

        assert_eq!(diff.unchanged.len(), 0);
    }

    #[test]
    fn test_chunk_deduplicator() {
        let mut dedup = ChunkDeduplicator::new(1024);

        // Same data twice should deduplicate
        let data = vec![0u8; 4096]; // 4KB of zeros
        let (chunk_ids1, new_chunks1) = dedup.split_into_chunks(&data);
        let (chunk_ids2, new_chunks2) = dedup.split_into_chunks(&data);

        assert_eq!(chunk_ids1, chunk_ids2); // Same IDs
        assert_eq!(new_chunks1.len(), 4); // 4 new chunks
        assert_eq!(new_chunks2.len(), 0); // No new chunks (all deduplicated)
    }
}
