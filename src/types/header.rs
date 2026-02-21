//! File header for the `.acb` binary format.
//!
//! The header is exactly 128 bytes and appears at the start of every `.acb` file.
//! It contains magic bytes, version, section offsets, and metadata.

use std::io::{Read, Write};

use super::error::{AcbError, AcbResult};
use super::{ACB_MAGIC, FORMAT_VERSION};

/// Header of an `.acb` file. Fixed size: 128 bytes.
///
/// Layout (all fields little-endian):
/// - 0x00: magic [u8; 4]
/// - 0x04: version u32
/// - 0x08: dimension u32
/// - 0x0C: language_count u32
/// - 0x10: unit_count u64
/// - 0x18: edge_count u64
/// - 0x20: unit_table_offset u64
/// - 0x28: edge_table_offset u64
/// - 0x30: string_pool_offset u64
/// - 0x38: feature_vec_offset u64
/// - 0x40: temporal_offset u64
/// - 0x48: index_offset u64
/// - 0x50: repo_hash [u8; 32]
/// - 0x70: compiled_at u64
/// - 0x78: _reserved [u8; 8]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileHeader {
    /// Magic bytes: must be [0x41, 0x43, 0x44, 0x42] ("ACDB").
    pub magic: [u8; 4],

    /// Format version (currently 1).
    pub version: u32,

    /// Feature vector dimensionality.
    pub dimension: u32,

    /// Number of supported languages in this file.
    pub language_count: u32,

    /// Total number of code units.
    pub unit_count: u64,

    /// Total number of edges.
    pub edge_count: u64,

    /// Byte offset to code unit table.
    pub unit_table_offset: u64,

    /// Byte offset to edge table.
    pub edge_table_offset: u64,

    /// Byte offset to string pool.
    pub string_pool_offset: u64,

    /// Byte offset to feature vectors.
    pub feature_vec_offset: u64,

    /// Byte offset to temporal block.
    pub temporal_offset: u64,

    /// Byte offset to index block.
    pub index_offset: u64,

    /// Repository root path hash (for cache validation).
    pub repo_hash: [u8; 32],

    /// Compilation timestamp (Unix epoch microseconds).
    pub compiled_at: u64,

    /// Reserved for future use.
    pub _reserved: [u8; 8],
}

/// The size of the file header in bytes.
pub const HEADER_SIZE: usize = 128;

impl FileHeader {
    /// Create a new header with sensible defaults and the given dimension.
    pub fn new(dimension: u32) -> Self {
        Self {
            magic: ACB_MAGIC,
            version: FORMAT_VERSION,
            dimension,
            language_count: 0,
            unit_count: 0,
            edge_count: 0,
            unit_table_offset: HEADER_SIZE as u64,
            edge_table_offset: HEADER_SIZE as u64,
            string_pool_offset: 0,
            feature_vec_offset: 0,
            temporal_offset: 0,
            index_offset: 0,
            repo_hash: [0u8; 32],
            compiled_at: crate::types::now_micros(),
            _reserved: [0u8; 8],
        }
    }

    /// Write the header to a byte writer (little-endian).
    pub fn write_to(&self, w: &mut impl Write) -> AcbResult<()> {
        w.write_all(&self.magic)?;
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.dimension.to_le_bytes())?;
        w.write_all(&self.language_count.to_le_bytes())?;
        w.write_all(&self.unit_count.to_le_bytes())?;
        w.write_all(&self.edge_count.to_le_bytes())?;
        w.write_all(&self.unit_table_offset.to_le_bytes())?;
        w.write_all(&self.edge_table_offset.to_le_bytes())?;
        w.write_all(&self.string_pool_offset.to_le_bytes())?;
        w.write_all(&self.feature_vec_offset.to_le_bytes())?;
        w.write_all(&self.temporal_offset.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.repo_hash)?;
        w.write_all(&self.compiled_at.to_le_bytes())?;
        w.write_all(&self._reserved)?;
        Ok(())
    }

    /// Read a header from a byte reader (little-endian).
    ///
    /// # Errors
    ///
    /// - `AcbError::InvalidMagic` if magic bytes don't match.
    /// - `AcbError::UnsupportedVersion` if version is not recognized.
    /// - `AcbError::Io` on read failure.
    pub fn read_from(r: &mut impl Read) -> AcbResult<Self> {
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if magic != ACB_MAGIC {
            return Err(AcbError::InvalidMagic);
        }

        let version = read_u32(r)?;
        if version > FORMAT_VERSION {
            return Err(AcbError::UnsupportedVersion(version));
        }

        let dimension = read_u32(r)?;
        let language_count = read_u32(r)?;
        let unit_count = read_u64(r)?;
        let edge_count = read_u64(r)?;
        let unit_table_offset = read_u64(r)?;
        let edge_table_offset = read_u64(r)?;
        let string_pool_offset = read_u64(r)?;
        let feature_vec_offset = read_u64(r)?;
        let temporal_offset = read_u64(r)?;
        let index_offset = read_u64(r)?;

        let mut repo_hash = [0u8; 32];
        r.read_exact(&mut repo_hash)?;

        let compiled_at = read_u64(r)?;

        let mut reserved = [0u8; 8];
        r.read_exact(&mut reserved)?;

        Ok(Self {
            magic,
            version,
            dimension,
            language_count,
            unit_count,
            edge_count,
            unit_table_offset,
            edge_table_offset,
            string_pool_offset,
            feature_vec_offset,
            temporal_offset,
            index_offset,
            repo_hash,
            compiled_at,
            _reserved: reserved,
        })
    }

    /// Serialize the header to a 128-byte array.
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        // write_to only fails on I/O, and Cursor<&mut [u8]> can't fail for 128 bytes
        self.write_to(&mut cursor)
            .expect("header write to fixed buffer");
        buf
    }

    /// Deserialize a header from a 128-byte slice.
    pub fn from_bytes(data: &[u8; HEADER_SIZE]) -> AcbResult<Self> {
        let mut cursor = std::io::Cursor::new(&data[..]);
        Self::read_from(&mut cursor)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn read_u32(r: &mut impl Read) -> AcbResult<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(r: &mut impl Read) -> AcbResult<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        let h = FileHeader::new(256);
        let bytes = h.to_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE);
        let h2 = FileHeader::from_bytes(&bytes).unwrap();
        assert_eq!(h, h2);
    }
}
