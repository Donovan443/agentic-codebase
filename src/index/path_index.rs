//! Index by file path.
//!
//! Maps file paths to the code unit IDs defined within each file.
//! Building the index is O(n); lookup by path is O(1).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::graph::CodeGraph;

/// Index that maps file paths to code unit IDs.
#[derive(Debug, Clone, Default)]
pub struct PathIndex {
    by_path: HashMap<PathBuf, Vec<u64>>,
    paths: Vec<PathBuf>,
}

impl PathIndex {
    /// Build a `PathIndex` from all code units in the given graph.
    pub fn build(graph: &CodeGraph) -> Self {
        let mut by_path: HashMap<PathBuf, Vec<u64>> = HashMap::new();

        for unit in graph.units() {
            by_path
                .entry(unit.file_path.clone())
                .or_default()
                .push(unit.id);
        }

        let mut paths: Vec<PathBuf> = by_path.keys().cloned().collect();
        paths.sort();

        Self { by_path, paths }
    }

    /// Look up all unit IDs in the given file path.
    ///
    /// Returns an empty slice if no units match.
    pub fn lookup(&self, path: &Path) -> &[u64] {
        self.by_path.get(path).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Returns all distinct file paths present in the index, sorted.
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Returns the number of distinct files in the index.
    pub fn file_count(&self) -> usize {
        self.paths.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::CodeGraph;
    use crate::types::{CodeUnit, CodeUnitType, Language, Span};

    fn make_unit(file_path: &str) -> CodeUnit {
        CodeUnit::new(
            CodeUnitType::Function,
            Language::Rust,
            "test_fn".to_string(),
            "mod::test_fn".to_string(),
            PathBuf::from(file_path),
            Span::new(1, 0, 10, 0),
        )
    }

    #[test]
    fn test_empty_index() {
        let graph = CodeGraph::default();
        let index = PathIndex::build(&graph);
        assert_eq!(index.file_count(), 0);
        assert!(index.paths().is_empty());
        assert_eq!(index.lookup(Path::new("src/lib.rs")), &[] as &[u64]);
    }

    #[test]
    fn test_path_lookup() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit("src/lib.rs"));
        graph.add_unit(make_unit("src/lib.rs"));
        graph.add_unit(make_unit("src/main.rs"));

        let index = PathIndex::build(&graph);
        assert_eq!(index.file_count(), 2);
        assert_eq!(index.lookup(Path::new("src/lib.rs")), &[0, 1]);
        assert_eq!(index.lookup(Path::new("src/main.rs")), &[2]);
        assert_eq!(index.lookup(Path::new("src/other.rs")), &[] as &[u64]);
    }

    #[test]
    fn test_paths_sorted() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit("src/z_file.rs"));
        graph.add_unit(make_unit("src/a_file.rs"));
        graph.add_unit(make_unit("src/m_file.rs"));

        let index = PathIndex::build(&graph);
        let paths = index.paths();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], PathBuf::from("src/a_file.rs"));
        assert_eq!(paths[1], PathBuf::from("src/m_file.rs"));
        assert_eq!(paths[2], PathBuf::from("src/z_file.rs"));
    }
}
