//! Incremental recompilation support.
//!
//! Uses content hashing to detect which files have changed since the last
//! compilation, then re-parses only those files and surgically updates
//! the graph.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::graph::CodeGraph;
use crate::parse::parser::ParseOptions;
use crate::parse::parser::Parser;
use crate::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};
use crate::types::AcbResult;

/// Set of files that have changed since the last compilation.
#[derive(Debug, Clone)]
pub struct ChangeSet {
    /// Files that are new (not in the previous graph).
    pub added: Vec<PathBuf>,
    /// Files whose content has changed.
    pub modified: Vec<PathBuf>,
    /// Files that no longer exist on disk.
    pub removed: Vec<PathBuf>,
}

impl ChangeSet {
    /// True if nothing changed.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }

    /// Total number of changed files.
    pub fn total(&self) -> usize {
        self.added.len() + self.modified.len() + self.removed.len()
    }
}

/// Result of an incremental recompilation.
#[derive(Debug, Clone)]
pub struct IncrementalResult {
    /// Number of code units added.
    pub units_added: usize,
    /// Number of code units removed.
    pub units_removed: usize,
    /// Number of code units modified (removed + re-added).
    pub units_modified: usize,
    /// Total duration of the incremental recompilation.
    pub duration: std::time::Duration,
}

/// Incremental compiler that tracks file content hashes to detect changes.
pub struct IncrementalCompiler {
    /// File path -> blake3 hash of contents.
    hashes: HashMap<PathBuf, String>,
}

impl IncrementalCompiler {
    /// Create a new incremental compiler with no previous state.
    pub fn new() -> Self {
        Self {
            hashes: HashMap::new(),
        }
    }

    /// Build an incremental compiler from an existing graph.
    ///
    /// Extracts file paths from the graph's code units and computes
    /// current content hashes for each file.
    pub fn from_graph(graph: &CodeGraph) -> Self {
        let mut hashes = HashMap::new();

        for unit in graph.units() {
            let path = &unit.file_path;
            if hashes.contains_key(path) {
                continue;
            }
            if let Ok(content) = std::fs::read(path) {
                let hash = blake3::hash(&content).to_hex().to_string();
                hashes.insert(path.clone(), hash);
            }
        }

        Self { hashes }
    }

    /// Detect which files have changed relative to the stored hashes.
    ///
    /// Walks the directory for supported source files and compares
    /// content hashes against the stored state.
    pub fn detect_changes(&self, dir: &Path) -> AcbResult<ChangeSet> {
        let current_files = collect_source_files(dir)?;

        let mut added = Vec::new();
        let mut modified = Vec::new();

        for path in &current_files {
            let current_hash = match std::fs::read(path) {
                Ok(content) => blake3::hash(&content).to_hex().to_string(),
                Err(_) => continue,
            };

            match self.hashes.get(path) {
                Some(stored_hash) => {
                    if *stored_hash != current_hash {
                        modified.push(path.clone());
                    }
                }
                None => {
                    added.push(path.clone());
                }
            }
        }

        // Files in stored hashes but not on disk
        let current_set: std::collections::HashSet<&PathBuf> = current_files.iter().collect();
        let removed: Vec<PathBuf> = self
            .hashes
            .keys()
            .filter(|p| !current_set.contains(p))
            .cloned()
            .collect();

        Ok(ChangeSet {
            added,
            modified,
            removed,
        })
    }

    /// Perform incremental recompilation.
    ///
    /// Re-parses changed files and rebuilds the graph. For simplicity,
    /// this performs a full rebuild when changes are detected — true
    /// surgical graph patching is a future optimisation.
    pub fn recompile(
        &mut self,
        dir: &Path,
        changes: &ChangeSet,
    ) -> AcbResult<(CodeGraph, IncrementalResult)> {
        let start = Instant::now();

        // Count units from changed files for reporting
        let changed_file_count = changes.total();

        tracing::info!(
            "Incremental: {} added, {} modified, {} removed",
            changes.added.len(),
            changes.modified.len(),
            changes.removed.len()
        );

        // For now, do a full recompile but only report the delta.
        // True incremental graph patching is a future optimisation.
        let parser = Parser::new();
        let parse_result = parser.parse_directory(dir, &ParseOptions::default())?;

        let analyzer = SemanticAnalyzer::new();
        let graph = analyzer.analyze(parse_result.units, &AnalyzeOptions::default())?;

        // Update stored hashes
        self.hashes.clear();
        for unit in graph.units() {
            let path = &unit.file_path;
            if self.hashes.contains_key(path) {
                continue;
            }
            if let Ok(content) = std::fs::read(path) {
                let hash = blake3::hash(&content).to_hex().to_string();
                self.hashes.insert(path.clone(), hash);
            }
        }

        let duration = start.elapsed();

        let result = IncrementalResult {
            units_added: changes.added.len(),
            units_removed: changes.removed.len(),
            units_modified: changes.modified.len(),
            duration,
        };

        tracing::info!(
            "Incremental recompile: {} changed files in {:.2?}",
            changed_file_count,
            duration
        );

        Ok((graph, result))
    }
}

impl Default for IncrementalCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect supported source files from a directory.
fn collect_source_files(dir: &Path) -> AcbResult<Vec<PathBuf>> {
    use ignore::WalkBuilder;

    let extensions = ["rs", "py", "ts", "tsx", "js", "jsx", "go"];
    let mut files = Vec::new();

    let walker = WalkBuilder::new(dir).hidden(true).git_ignore(true).build();

    for entry in walker {
        let entry = entry
            .map_err(|e| crate::AcbError::Io(std::io::Error::other(format!("Walk error: {e}"))))?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext) {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}
