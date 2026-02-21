//! LZ4 string pool compression and decompression.
//!
//! The string pool stores all variable-length strings (names, paths, docs)
//! in a single contiguous block, compressed with LZ4 for compactness.

use crate::types::{AcbError, AcbResult};

/// Compress data using LZ4 block compression.
///
/// The output includes a prepended size header for safe decompression.
pub fn compress(data: &[u8]) -> Vec<u8> {
    lz4_flex::compress_prepend_size(data)
}

/// Decompress LZ4-compressed data with prepended size.
///
/// # Errors
///
/// Returns `AcbError::Compression` if the data is corrupt or truncated.
pub fn decompress(data: &[u8]) -> AcbResult<Vec<u8>> {
    lz4_flex::decompress_size_prepended(data)
        .map_err(|e| AcbError::Compression(format!("LZ4 decompression failed: {}", e)))
}

/// A string pool builder that collects strings and records their offsets.
///
/// Strings are stored contiguously as UTF-8. Each string is referenced
/// by `(offset, length)`.
#[derive(Debug, Default)]
pub struct StringPoolBuilder {
    data: Vec<u8>,
}

impl StringPoolBuilder {
    /// Create a new empty string pool builder.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Add a string to the pool and return its (offset, length).
    pub fn add(&mut self, s: &str) -> (u32, u16) {
        let offset = self.data.len() as u32;
        let len = s.len() as u16;
        self.data.extend_from_slice(s.as_bytes());
        (offset, len)
    }

    /// Return the uncompressed data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Return the uncompressed size.
    pub fn uncompressed_size(&self) -> usize {
        self.data.len()
    }

    /// Compress and return the pool data.
    pub fn compress(&self) -> Vec<u8> {
        compress(&self.data)
    }
}

/// A read-only string pool backed by decompressed data.
#[derive(Debug, Clone)]
pub struct StringPool {
    data: Vec<u8>,
}

impl StringPool {
    /// Create a string pool from decompressed data.
    pub fn from_data(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Create a string pool by decompressing LZ4 data.
    pub fn from_compressed(compressed: &[u8]) -> AcbResult<Self> {
        let data = decompress(compressed)?;
        Ok(Self { data })
    }

    /// Get a string by offset and length.
    ///
    /// # Errors
    ///
    /// Returns `AcbError::Corrupt` if the range is out of bounds or not valid UTF-8.
    pub fn get(&self, offset: u32, len: u16) -> AcbResult<&str> {
        let start = offset as usize;
        let end = start + len as usize;
        if end > self.data.len() {
            return Err(AcbError::Corrupt(offset as u64));
        }
        std::str::from_utf8(&self.data[start..end]).map_err(|_| AcbError::Corrupt(offset as u64))
    }

    /// Total size of the decompressed pool.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
