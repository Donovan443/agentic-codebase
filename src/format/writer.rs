//! Writes `.acb` files from a `CodeGraph`.
//!
//! The writer serializes the graph into the binary `.acb` format:
//! header, unit table, edge table, compressed string pool, feature vectors,
//! temporal block, and index block.

use std::io::Write;
use std::path::Path;

use crate::graph::CodeGraph;
use crate::types::header::{FileHeader, HEADER_SIZE};
use crate::types::{AcbResult, DEFAULT_DIMENSION};

use super::compression::StringPoolBuilder;

/// Size of one code unit record on disk (96 bytes).
pub const UNIT_RECORD_SIZE: usize = 96;

/// Size of one edge record on disk (40 bytes).
pub const EDGE_RECORD_SIZE: usize = 40;

/// Writes `CodeGraph` instances to `.acb` binary format.
pub struct AcbWriter {
    dimension: usize,
}

impl AcbWriter {
    /// Create a new writer with the given feature vector dimension.
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    /// Create a writer with default dimension.
    pub fn with_default_dimension() -> Self {
        Self::new(DEFAULT_DIMENSION)
    }

    /// Write a code graph to a file path.
    ///
    /// # Errors
    ///
    /// Returns `AcbError::Io` on write failure.
    pub fn write_to_file(&self, graph: &CodeGraph, path: &Path) -> AcbResult<()> {
        let mut file = std::fs::File::create(path)?;
        self.write_to(graph, &mut file)
    }

    /// Write a code graph to any writer.
    pub fn write_to(&self, graph: &CodeGraph, w: &mut impl Write) -> AcbResult<()> {
        // 1. Build string pool, collecting offsets for each unit
        let mut pool = StringPoolBuilder::new();
        let mut unit_strings: Vec<UnitStrings> = Vec::with_capacity(graph.unit_count());

        for unit in graph.units() {
            let (name_offset, name_len) = pool.add(&unit.name);
            let (qname_offset, qname_len) = pool.add(&unit.qualified_name);
            let path_str = unit.file_path.to_string_lossy();
            let (path_offset, path_len) = pool.add(&path_str);
            unit_strings.push(UnitStrings {
                name_offset,
                name_len,
                qname_offset,
                qname_len,
                path_offset,
                path_len,
            });
        }

        let compressed_pool = pool.compress();

        // 2. Sort edges by source_id then target_id for contiguous grouping
        let mut sorted_edges: Vec<_> = graph.edges().to_vec();
        sorted_edges.sort_by(|a, b| {
            a.source_id
                .cmp(&b.source_id)
                .then(a.target_id.cmp(&b.target_id))
        });

        // Compute edge offsets per unit
        let mut unit_edge_offsets: Vec<(u64, u32)> = vec![(0, 0); graph.unit_count()];
        {
            let mut current_source = u64::MAX;
            let mut current_offset = 0u64;
            let mut current_count = 0u32;

            for (i, edge) in sorted_edges.iter().enumerate() {
                if edge.source_id != current_source {
                    if current_source != u64::MAX {
                        unit_edge_offsets[current_source as usize] =
                            (current_offset, current_count);
                    }
                    current_source = edge.source_id;
                    current_offset = (i as u64) * EDGE_RECORD_SIZE as u64;
                    current_count = 0;
                }
                current_count += 1;
            }
            if current_source != u64::MAX {
                unit_edge_offsets[current_source as usize] = (current_offset, current_count);
            }
        }

        // 3. Calculate section offsets
        let unit_table_offset = HEADER_SIZE as u64;
        let edge_table_offset =
            unit_table_offset + (graph.unit_count() as u64) * UNIT_RECORD_SIZE as u64;
        let string_pool_offset =
            edge_table_offset + (sorted_edges.len() as u64) * EDGE_RECORD_SIZE as u64;
        // String pool section: 8 bytes uncompressed size + compressed data
        let string_pool_section_size = 8 + compressed_pool.len() as u64;
        let feature_vec_offset = string_pool_offset + string_pool_section_size;
        let feature_vec_size = (graph.unit_count() as u64) * (self.dimension as u64) * 4;
        let temporal_offset = feature_vec_offset + feature_vec_size;
        // Empty temporal block for now (just 16 bytes: two u64 zeros)
        let temporal_size = 16u64;
        let index_offset = temporal_offset + temporal_size;

        // 4. Build header
        let mut header = FileHeader::new(self.dimension as u32);
        header.unit_count = graph.unit_count() as u64;
        header.edge_count = sorted_edges.len() as u64;
        header.language_count = graph.languages().len() as u32;
        header.unit_table_offset = unit_table_offset;
        header.edge_table_offset = edge_table_offset;
        header.string_pool_offset = string_pool_offset;
        header.feature_vec_offset = feature_vec_offset;
        header.temporal_offset = temporal_offset;
        header.index_offset = index_offset;

        // 5. Write header
        header.write_to(w)?;

        // 6. Write unit table
        for (i, unit) in graph.units().iter().enumerate() {
            let us = &unit_strings[i];
            let (eoff, ecnt) = unit_edge_offsets[i];
            write_unit_record(w, unit, us, eoff, ecnt)?;
        }

        // 7. Write edge table
        for edge in &sorted_edges {
            write_edge_record(w, edge)?;
        }

        // 8. Write string pool (uncompressed size + compressed data)
        w.write_all(&(pool.uncompressed_size() as u64).to_le_bytes())?;
        w.write_all(&compressed_pool)?;

        // 9. Write feature vectors
        for unit in graph.units() {
            for &val in &unit.feature_vec {
                w.write_all(&val.to_le_bytes())?;
            }
            // Pad if vector is shorter than dimension
            for _ in unit.feature_vec.len()..self.dimension {
                w.write_all(&0.0f32.to_le_bytes())?;
            }
        }

        // 10. Write temporal block (empty placeholder)
        w.write_all(&0u64.to_le_bytes())?; // history_size = 0
        w.write_all(&0u64.to_le_bytes())?; // coupling_count = 0

        // 11. Write index block (end marker only for now)
        w.write_all(&0xFFFFFFFFu32.to_le_bytes())?;

        Ok(())
    }
}

/// Intermediate struct for string pool references.
struct UnitStrings {
    name_offset: u32,
    name_len: u16,
    qname_offset: u32,
    qname_len: u16,
    path_offset: u32,
    path_len: u16,
}

/// Write a 96-byte code unit record.
fn write_unit_record(
    w: &mut impl Write,
    unit: &crate::types::CodeUnit,
    strings: &UnitStrings,
    edge_offset: u64,
    edge_count: u32,
) -> AcbResult<()> {
    // Identity: 16 bytes
    w.write_all(&unit.id.to_le_bytes())?; // 8
    w.write_all(&[unit.unit_type as u8])?; // 1
    w.write_all(&[unit.language as u8])?; // 1
    w.write_all(&[unit.visibility as u8])?; // 1
    let flags: u8 = (unit.is_async as u8) | ((unit.is_generator as u8) << 1);
    w.write_all(&[flags])?; // 1
    let complexity_u16 = unit.complexity as u16;
    w.write_all(&complexity_u16.to_le_bytes())?; // 2
    w.write_all(&[0u8; 2])?; // _pad1: 2

    // String references: 24 bytes
    w.write_all(&strings.name_offset.to_le_bytes())?; // 4
    w.write_all(&strings.name_len.to_le_bytes())?; // 2
    w.write_all(&strings.qname_offset.to_le_bytes())?; // 4
    w.write_all(&strings.qname_len.to_le_bytes())?; // 2
    w.write_all(&strings.path_offset.to_le_bytes())?; // 4
    w.write_all(&strings.path_len.to_le_bytes())?; // 2
    w.write_all(&[0u8; 6])?; // _pad2: 6

    // Source location: 16 bytes
    w.write_all(&unit.span.start_line.to_le_bytes())?; // 4
    let start_col_u16 = unit.span.start_col as u16;
    w.write_all(&start_col_u16.to_le_bytes())?; // 2
    w.write_all(&unit.span.end_line.to_le_bytes())?; // 4
    let end_col_u16 = unit.span.end_col as u16;
    w.write_all(&end_col_u16.to_le_bytes())?; // 2
    w.write_all(&[0u8; 4])?; // _pad3: 4

    // Temporal: 24 bytes
    w.write_all(&unit.created_at.to_le_bytes())?; // 8
    w.write_all(&unit.last_modified.to_le_bytes())?; // 8
    let change_count_u32 = unit.change_count;
    w.write_all(&change_count_u32.to_le_bytes())?; // 4
    let stability_x100 = (unit.stability_score * 100.0).round() as u16;
    w.write_all(&stability_x100.to_le_bytes())?; // 2
    w.write_all(&[0u8; 2])?; // _pad4: 2

    // Graph: 16 bytes
    w.write_all(&edge_offset.to_le_bytes())?; // 8
    w.write_all(&edge_count.to_le_bytes())?; // 4
    w.write_all(&[0u8; 4])?; // _pad5: 4

    Ok(())
}

/// Write a 40-byte edge record.
fn write_edge_record(w: &mut impl Write, edge: &crate::types::Edge) -> AcbResult<()> {
    w.write_all(&edge.source_id.to_le_bytes())?; // 8
    w.write_all(&edge.target_id.to_le_bytes())?; // 8
    w.write_all(&[edge.edge_type as u8])?; // 1
    w.write_all(&[0u8; 3])?; // _pad1: 3
    w.write_all(&edge.weight.to_bits().to_le_bytes())?; // 4
    w.write_all(&edge.created_at.to_le_bytes())?; // 8
    w.write_all(&edge.context.to_le_bytes())?; // 4
    w.write_all(&[0u8; 4])?; // _pad2: 4

    Ok(())
}
