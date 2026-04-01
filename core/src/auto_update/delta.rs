//! Delta update support using bsdiff/bspatch algorithm

use sha2::{Sha256, Digest};
use std::path::Path;
use std::io::{Read, Write};

pub struct DeltaPatcher {
    // Configuration for delta patching
}

impl DeltaPatcher {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }

    /// Apply delta patch to create new file
    pub async fn apply_patch(
        &self,
        old_file: &Path,
        patch_file: &Path,
        output_file: &Path,
    ) -> anyhow::Result<()> {
        // Read old file
        let old_data = tokio::fs::read(old_file).await?;

        // Read patch
        let patch_data = tokio::fs::read(patch_file).await?;

        // Apply bspatch algorithm
        let new_data = bspatch(&old_data, &patch_data)?;

        // Write output
        tokio::fs::write(output_file, &new_data).await?;

        Ok(())
    }

    /// Create delta patch between two files
    pub async fn create_patch(
        &self,
        old_file: &Path,
        new_file: &Path,
        patch_file: &Path,
    ) -> anyhow::Result<u64> {
        // Read both files
        let old_data = tokio::fs::read(old_file).await?;
        let new_data = tokio::fs::read(new_file).await?;

        // Create bsdiff patch
        let patch = bsdiff(&old_data, &new_data)?;

        // Compress patch
        let compressed = compress_patch(&patch)?;

        // Write patch file
        tokio::fs::write(patch_file, &compressed).await?;

        Ok(compressed.len() as u64)
    }

    /// Verify patch integrity
    pub fn verify_patch(old_data: &[u8], patch_data: &[u8], expected_hash: &str) -> anyhow::Result<bool> {
        let result = bspatch(old_data, patch_data)?;

        let mut hasher = sha2::Sha256::new();
        hasher.update(&result);
        let hash = format!("{:x}", hasher.finalize());

        Ok(hash == expected_hash)
    }
}

/// bspatch algorithm implementation
/// Based on Colin Percival's bsdiff/bspatch
fn bspatch(old: &[u8], patch: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut patch_cursor = std::io::Cursor::new(patch);

    // Read header
    let mut header = [0u8; 32];
    patch_cursor.read_exact(&mut header)?;

    // Verify magic
    if &header[0..8] != b"BSDIFF40" {
        return Err(anyhow::anyhow!("Invalid patch magic"));
    }

    // Read lengths from header
    let ctrl_len = u64::from_le_bytes([
        header[8], header[9], header[10], header[11],
        header[12], header[13], header[14], header[15],
    ]) as usize;

    let diff_len = u64::from_le_bytes([
        header[16], header[17], header[18], header[19],
        header[20], header[21], header[22], header[23],
    ]) as usize;

    let new_len = u64::from_le_bytes([
        header[24], header[25], header[26], header[27],
        header[28], header[29], header[30], header[31],
    ]) as usize;

    // Read compressed control blocks
    let mut ctrl_compressed = vec![0u8; ctrl_len];
    patch_cursor.read_exact(&mut ctrl_compressed)?;

    let mut diff_compressed = vec![0u8; diff_len];
    patch_cursor.read_exact(&mut diff_compressed)?;

    // Read extra data (rest of patch)
    let mut extra_compressed = Vec::new();
    patch_cursor.read_to_end(&mut extra_compressed)?;

    // Decompress control blocks
    let ctrl = decompress_bz2(&ctrl_compressed)?;
    let diff = decompress_bz2(&diff_compressed)?;
    let extra = decompress_bz2(&extra_compressed)?;

    // Apply patch
    let mut new = vec![0u8; new_len];
    let mut old_pos: usize = 0;
    let mut new_pos: usize = 0;
    let mut ctrl_pos: usize = 0;
    let mut diff_pos: usize = 0;
    let mut extra_pos: usize = 0;

    while new_pos < new_len {
        // Read control triplet
        let add = read_int(&ctrl, &mut ctrl_pos)? as usize;
        let copy = read_int(&ctrl, &mut ctrl_pos)? as usize;
        let seek = read_int(&ctrl, &mut ctrl_pos)?;

        // Sanity checks
        if new_pos + add > new_len || diff_pos + add > diff.len() {
            return Err(anyhow::anyhow!("Corrupt patch data"));
        }

        // Add diff data
        for i in 0..add {
            let v = diff[diff_pos + i].wrapping_add(old.get(old_pos + i).copied().unwrap_or(0));
            new[new_pos + i] = v;
        }

        new_pos += add;
        old_pos += add;
        diff_pos += add;

        // Sanity check
        if new_pos + copy > new_len || extra_pos + copy > extra.len() {
            return Err(anyhow::anyhow!("Corrupt patch data"));
        }

        // Copy extra data
        new[new_pos..new_pos + copy].copy_from_slice(&extra[extra_pos..extra_pos + copy]);

        new_pos += copy;
        extra_pos += copy;

        // Adjust old position
        if seek >= 0 {
            old_pos += seek as usize;
        } else {
            old_pos -= (-seek) as usize;
        }
    }

    Ok(new)
}

/// bsdiff algorithm - create binary diff
fn bsdiff(old: &[u8], new: &[u8]) -> anyhow::Result<Vec<u8>> {
    // Simplified implementation - in production, use a proper bsdiff library
    // or the divsufsort algorithm for suffix array construction

    // For now, create a simple delta format
    let mut patch = Vec::new();

    // Write header
    patch.extend_from_slice(b"BSDIFF40");

    // Placeholder: just store the new file
    // In production, implement proper suffix array matching
    let new_compressed = compress_bz2(new)?;

    // Write lengths
    patch.extend_from_slice(&(0u64).to_le_bytes()); // ctrl_len
    patch.extend_from_slice(&(0u64).to_le_bytes()); // diff_len
    patch.extend_from_slice(&(new.len() as u64).to_le_bytes()); // new_len

    // Write compressed new file
    patch.extend_from_slice(&new_compressed);

    Ok(patch)
}

fn read_int(data: &[u8], pos: &mut usize) -> anyhow::Result<i64> {
    if *pos + 8 > data.len() {
        return Err(anyhow::anyhow!("Unexpected end of control data"));
    }

    let val = i64::from_le_bytes([
        data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3],
        data[*pos + 4], data[*pos + 5], data[*pos + 6], data[*pos + 7],
    ]);

    *pos += 8;
    Ok(val)
}

fn compress_bz2(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    // Use bzip2 compression
    let mut encoder = bzip2::write::BzEncoder::new(
        Vec::new(),
        bzip2::Compression::best(),
    );

    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

fn decompress_bz2(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut decoder = bzip2::read::BzDecoder::new(std::io::Cursor::new(data));
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

fn compress_patch(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    // Use zstd for fast decompression
    zstd::encode_all(std::io::Cursor::new(data), 3)
        .map_err(|e| anyhow::anyhow!("Compression failed: {}", e))
}

/// Calculate delta statistics
pub struct DeltaStats {
    pub old_size: u64,
    pub new_size: u64,
    pub patch_size: u64,
    pub savings_percent: f32,
}

impl DeltaStats {
    pub fn new(old_size: u64, new_size: u64, patch_size: u64) -> Self {
        let savings = if new_size > 0 {
            ((new_size - patch_size) as f32 / new_size as f32) * 100.0
        } else {
            0.0
        };

        Self {
            old_size,
            new_size,
            patch_size,
            savings_percent: savings,
        }
    }
}

/// Chunked delta for large files
pub struct ChunkedDeltaPatcher {
    chunk_size: usize,
}

impl ChunkedDeltaPatcher {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// Apply chunked patch
    pub async fn apply_chunked(
        &self,
        old_file: &Path,
        patch_dir: &Path,
        output_file: &Path,
    ) -> anyhow::Result<()> {
        let old_data = tokio::fs::read(old_file).await?;

        // Read manifest
        let manifest: ChunkedManifest = {
            let content = tokio::fs::read(patch_dir.join("manifest.json")).await?;
            serde_json::from_slice(&content)?
        };

        let mut new_data = vec![0u8; manifest.total_size];

        for chunk in &manifest.chunks {
            let chunk_path = patch_dir.join(&chunk.filename);
            let chunk_data = tokio::fs::read(&chunk_path).await?;

            if chunk.is_delta {
                // Apply delta to old chunk
                let old_start = chunk.source_offset as usize;
                let old_end = (chunk.source_offset + chunk.source_length) as usize;
                let old_chunk = &old_data[old_start..old_end.min(old_data.len())];

                let new_chunk = bspatch(old_chunk, &chunk_data)?;
                new_data[chunk.target_offset as usize..][..new_chunk.len()]
                    .copy_from_slice(&new_chunk);
            } else {
                // Raw data
                new_data[chunk.target_offset as usize..][..chunk_data.len()]
                    .copy_from_slice(&chunk_data);
            }
        }

        tokio::fs::write(output_file, &new_data).await?;

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ChunkedManifest {
    pub version: u32,
    pub total_size: usize,
    pub chunks: Vec<ChunkInfo>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ChunkInfo {
    pub filename: String,
    pub target_offset: u64,
    pub target_length: u64,
    pub is_delta: bool,
    pub source_offset: u64,
    pub source_length: u64,
    pub hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let old = b"Hello, World! This is the old version.";
        let new = b"Hello, World! This is the NEW version with more text.";

        // Create patch
        let patch = bsdiff(old, new).unwrap();

        // Apply patch
        let result = bspatch(old, &patch).unwrap();

        assert_eq!(&result[..], &new[..]);
    }

    #[test]
    fn test_compress_roundtrip() {
        let data = b"Test data for compression and decompression";

        let compressed = compress_bz2(data).unwrap();
        let decompressed = decompress_bz2(&compressed).unwrap();

        assert_eq!(&decompressed[..], &data[..]);
    }
}
