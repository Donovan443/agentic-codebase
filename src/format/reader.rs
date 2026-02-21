//! Reads `.acb` files into a `CodeGraph`.
//!
//! The reader deserializes the binary `.acb` format back into the
//! in-memory graph structure.

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::graph::CodeGraph;
use crate::types::header::{FileHeader, HEADER_SIZE};
use crate::types::{
    AcbError, AcbResult, CodeUnit, CodeUnitType, Edge, EdgeType, Language, Span, Visibility,
    ACB_MAGIC, FORMAT_VERSION,
};

use super::compression::StringPool;
use super::writer::{EDGE_RECORD_SIZE, UNIT_RECORD_SIZE};
use super::AcbWriter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StorageMigrationPolicy {
    AutoSafe,
    Strict,
    Off,
}

impl StorageMigrationPolicy {
    fn from_env(name: &str) -> Self {
        let raw = std::env::var(name).unwrap_or_else(|_| "auto-safe".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "strict" => Self::Strict,
            "off" | "disabled" | "none" => Self::Off,
            _ => Self::AutoSafe,
        }
    }
}

/// Reads `.acb` files into `CodeGraph` instances.
pub struct AcbReader;

impl AcbReader {
    /// Read a code graph from a file path.
    ///
    /// # Errors
    ///
    /// Returns errors on I/O failure, corrupt data, or version mismatch.
    pub fn read_from_file(path: &Path) -> AcbResult<CodeGraph> {
        if !path.exists() {
            return Err(AcbError::PathNotFound(path.to_path_buf()));
        }
        let data = std::fs::read(path)?;
        if data.len() < HEADER_SIZE {
            return Err(AcbError::Truncated);
        }
        let legacy_version = detect_legacy_version(&data);
        let migration_policy = StorageMigrationPolicy::from_env("ACB_STORAGE_MIGRATION_POLICY");
        if let Some(from_version) = legacy_version {
            if migration_policy == StorageMigrationPolicy::Strict {
                return Err(AcbError::UnsupportedVersion(from_version));
            }
        }

        let graph = Self::read_from_data(&data)?;
        if let Some(from_version) = legacy_version {
            match migration_policy {
                StorageMigrationPolicy::AutoSafe => {
                    if let Err(err) = migrate_file_in_place(path, &graph, from_version) {
                        tracing::warn!(
                            "Failed to auto-migrate {} from v{}: {}",
                            path.display(),
                            from_version,
                            err
                        );
                    }
                }
                StorageMigrationPolicy::Off => {
                    tracing::warn!(
                        "Legacy .acb version {} loaded for {} with migration disabled",
                        from_version,
                        path.display()
                    );
                }
                StorageMigrationPolicy::Strict => {}
            }
        }
        Ok(graph)
    }

    /// Read a code graph from a byte slice.
    pub fn read_from_data(data: &[u8]) -> AcbResult<CodeGraph> {
        if data.len() < HEADER_SIZE {
            return Err(AcbError::Truncated);
        }

        // 1. Read header
        let header_bytes: [u8; HEADER_SIZE] = data[..HEADER_SIZE]
            .try_into()
            .map_err(|_| AcbError::Truncated)?;
        let header = FileHeader::from_bytes(&header_bytes)?;

        // Validate offsets are within file bounds
        let file_len = data.len() as u64;
        validate_offset(header.unit_table_offset, file_len)?;
        if header.unit_count > 0 {
            let unit_table_end =
                header.unit_table_offset + header.unit_count * UNIT_RECORD_SIZE as u64;
            if unit_table_end > file_len {
                return Err(AcbError::Truncated);
            }
        }
        if header.edge_count > 0 {
            validate_offset(header.edge_table_offset, file_len)?;
            let edge_table_end =
                header.edge_table_offset + header.edge_count * EDGE_RECORD_SIZE as u64;
            if edge_table_end > file_len {
                return Err(AcbError::Truncated);
            }
        }

        // 2. Read string pool
        let pool = if header.string_pool_offset > 0 && header.string_pool_offset < file_len {
            let pool_start = header.string_pool_offset as usize;
            if pool_start + 8 > data.len() {
                return Err(AcbError::Truncated);
            }
            let _uncompressed_size =
                u64::from_le_bytes(data[pool_start..pool_start + 8].try_into().unwrap());
            let compressed_data = &data[pool_start + 8..];
            // The compressed data extends until the feature_vec_offset
            let compressed_end = if header.feature_vec_offset > 0 {
                (header.feature_vec_offset as usize).saturating_sub(pool_start + 8)
            } else {
                compressed_data.len()
            };
            let compressed_slice = &compressed_data[..compressed_end.min(compressed_data.len())];
            StringPool::from_compressed(compressed_slice)?
        } else {
            StringPool::from_data(Vec::new())
        };

        // 3. Read unit table
        let mut graph = CodeGraph::new(header.dimension as usize);
        let mut unit_edge_info: Vec<(u64, u32)> = Vec::with_capacity(header.unit_count as usize);

        for i in 0..header.unit_count {
            let offset = header.unit_table_offset as usize + (i as usize) * UNIT_RECORD_SIZE;
            let record = &data[offset..offset + UNIT_RECORD_SIZE];
            let (unit, edge_offset, edge_count) = read_unit_record(record, &pool)?;
            unit_edge_info.push((edge_offset, edge_count));
            graph.add_unit(unit);
        }

        // 4. Read edge table
        for i in 0..header.edge_count {
            let offset = header.edge_table_offset as usize + (i as usize) * EDGE_RECORD_SIZE;
            let record = &data[offset..offset + EDGE_RECORD_SIZE];
            let edge = read_edge_record(record)?;
            // Lenient: skip invalid edges rather than failing the entire read
            if let Err(e) = graph.add_edge(edge) {
                tracing::warn!("Skipping invalid edge during read: {}", e);
            }
        }

        // 5. Read feature vectors
        if header.feature_vec_offset > 0 && header.feature_vec_offset < file_len {
            let dim = header.dimension as usize;
            for i in 0..header.unit_count {
                let vec_offset = header.feature_vec_offset as usize + (i as usize) * dim * 4;
                if vec_offset + dim * 4 <= data.len() {
                    let mut fv = Vec::with_capacity(dim);
                    for d in 0..dim {
                        let fo = vec_offset + d * 4;
                        let val = f32::from_le_bytes(data[fo..fo + 4].try_into().unwrap());
                        fv.push(val);
                    }
                    if let Some(unit) = graph.get_unit_mut(i) {
                        unit.feature_vec = fv;
                    }
                }
            }
        }

        Ok(graph)
    }

    /// Read a code graph from a reader (consumes all bytes).
    pub fn read_from(reader: &mut impl Read) -> AcbResult<CodeGraph> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Self::read_from_data(&data)
    }
}

fn validate_offset(offset: u64, file_len: u64) -> AcbResult<()> {
    if offset > file_len {
        Err(AcbError::Truncated)
    } else {
        Ok(())
    }
}

fn detect_legacy_version(data: &[u8]) -> Option<u32> {
    if data.len() < 8 {
        return None;
    }
    if data[0..4] != ACB_MAGIC {
        return None;
    }
    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if version < FORMAT_VERSION {
        Some(version)
    } else {
        None
    }
}

fn migrate_file_in_place(path: &Path, graph: &CodeGraph, from_version: u32) -> AcbResult<()> {
    let migration_dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".acb-migrations");
    std::fs::create_dir_all(&migration_dir)?;

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("graph");
    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let checkpoint = migration_dir.join(format!("{stem}.v{from_version}.{ts}.acb.checkpoint"));
    std::fs::copy(path, &checkpoint)?;

    let writer = AcbWriter::new(graph.dimension());
    writer.write_to_file(graph, path)?;
    tracing::info!(
        "Auto-migrated {} from v{} to v{} (checkpoint: {})",
        path.display(),
        from_version,
        FORMAT_VERSION,
        checkpoint.display()
    );
    Ok(())
}

/// Read a 96-byte code unit record from a slice.
fn read_unit_record(data: &[u8], pool: &StringPool) -> AcbResult<(CodeUnit, u64, u32)> {
    let id = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let unit_type = CodeUnitType::from_u8(data[8]).ok_or(AcbError::Corrupt(0))?;
    let language = Language::from_u8(data[9]).ok_or(AcbError::Corrupt(1))?;
    let visibility = Visibility::from_u8(data[10]).ok_or(AcbError::Corrupt(2))?;
    let flags = data[11];
    let is_async = (flags & 1) != 0;
    let is_generator = (flags & 2) != 0;
    let complexity = u16::from_le_bytes(data[12..14].try_into().unwrap()) as u32;
    // _pad1: 14..16

    // String references
    let name_offset = u32::from_le_bytes(data[16..20].try_into().unwrap());
    let name_len = u16::from_le_bytes(data[20..22].try_into().unwrap());
    let qname_offset = u32::from_le_bytes(data[22..26].try_into().unwrap());
    let qname_len = u16::from_le_bytes(data[26..28].try_into().unwrap());
    let path_offset = u32::from_le_bytes(data[28..32].try_into().unwrap());
    let path_len = u16::from_le_bytes(data[32..34].try_into().unwrap());
    // _pad2: 34..40

    // Source location
    let start_line = u32::from_le_bytes(data[40..44].try_into().unwrap());
    let start_col = u16::from_le_bytes(data[44..46].try_into().unwrap()) as u32;
    let end_line = u32::from_le_bytes(data[46..50].try_into().unwrap());
    let end_col = u16::from_le_bytes(data[50..52].try_into().unwrap()) as u32;
    // _pad3: 52..56

    // Temporal
    let created_at = u64::from_le_bytes(data[56..64].try_into().unwrap());
    let last_modified = u64::from_le_bytes(data[64..72].try_into().unwrap());
    let change_count = u32::from_le_bytes(data[72..76].try_into().unwrap());
    let stability_x100 = u16::from_le_bytes(data[76..78].try_into().unwrap());
    let stability_score = stability_x100 as f32 / 100.0;
    // _pad4: 78..80

    // Graph
    let edge_offset = u64::from_le_bytes(data[80..88].try_into().unwrap());
    let edge_count = u32::from_le_bytes(data[88..92].try_into().unwrap());
    // _pad5: 92..96

    // Resolve strings from pool
    let name = if name_len > 0 {
        pool.get(name_offset, name_len)?.to_string()
    } else {
        String::new()
    };
    let qualified_name = if qname_len > 0 {
        pool.get(qname_offset, qname_len)?.to_string()
    } else {
        String::new()
    };
    let file_path = if path_len > 0 {
        PathBuf::from(pool.get(path_offset, path_len)?)
    } else {
        PathBuf::new()
    };

    let mut unit = CodeUnit::new(
        unit_type,
        language,
        name,
        qualified_name,
        file_path,
        Span::new(start_line, start_col, end_line, end_col),
    );
    unit.id = id;
    unit.visibility = visibility;
    unit.is_async = is_async;
    unit.is_generator = is_generator;
    unit.complexity = complexity;
    unit.created_at = created_at;
    unit.last_modified = last_modified;
    unit.change_count = change_count;
    unit.stability_score = stability_score;

    Ok((unit, edge_offset, edge_count))
}

/// Read a 40-byte edge record from a slice.
fn read_edge_record(data: &[u8]) -> AcbResult<Edge> {
    let source_id = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let target_id = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let edge_type = EdgeType::from_u8(data[16]).ok_or(AcbError::Corrupt(16))?;
    // _pad1: 17..20
    let weight_bits = u32::from_le_bytes(data[20..24].try_into().unwrap());
    let weight = f32::from_bits(weight_bits);
    let created_at = u64::from_le_bytes(data[24..32].try_into().unwrap());
    let context = u32::from_le_bytes(data[32..36].try_into().unwrap());
    // _pad2: 36..40

    Ok(Edge {
        source_id,
        target_id,
        edge_type,
        weight,
        created_at,
        context,
    })
}
