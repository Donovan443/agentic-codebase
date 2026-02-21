//! Memory-mapped file access for `.acb` files.
//!
//! Provides zero-copy access to `.acb` data using OS-level memory mapping.
//! This is the preferred access method for large files.

use std::path::Path;

use crate::graph::CodeGraph;
use crate::types::{AcbError, AcbResult};

use super::reader::AcbReader;

/// A memory-mapped view of an `.acb` file.
///
/// This wraps the mmap and provides access to the graph data
/// without loading the entire file into heap memory.
pub struct MappedCodeGraph {
    _mmap: memmap2::Mmap,
    graph: CodeGraph,
}

impl MappedCodeGraph {
    /// Open and memory-map an `.acb` file.
    ///
    /// The file is mapped read-only. The graph is parsed from the mapped data.
    ///
    /// # Safety
    ///
    /// This uses `unsafe` internally for mmap. The file must not be modified
    /// while the mapping is active.
    pub fn open(path: &Path) -> AcbResult<Self> {
        if !path.exists() {
            return Err(AcbError::PathNotFound(path.to_path_buf()));
        }

        let file = std::fs::File::open(path)?;
        // SAFETY: We only map read-only, and we don't expose the raw mapping.
        // The caller must ensure the file is not modified while mapped.
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let graph = AcbReader::read_from_data(&mmap)?;

        Ok(Self { _mmap: mmap, graph })
    }

    /// Get a reference to the parsed code graph.
    pub fn graph(&self) -> &CodeGraph {
        &self.graph
    }

    /// Consume and return the parsed graph (drops the mmap).
    pub fn into_graph(self) -> CodeGraph {
        self.graph
    }
}
