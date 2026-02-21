//! Index by symbol name for fast lookup.
//!
//! Provides O(1) exact name lookup, case-insensitive search, prefix search
//! via binary search on sorted names, and substring search via linear scan.

use std::collections::HashMap;

use crate::graph::CodeGraph;

/// Index that maps symbol names to code unit IDs for O(1) lookup.
#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    /// Exact name -> list of unit IDs.
    exact: HashMap<String, Vec<u64>>,
    /// Lowercase name -> list of unit IDs (for case-insensitive search).
    lowercase: HashMap<String, Vec<u64>>,
    /// Sorted (lowercase name, unit ID) pairs for prefix search (binary search).
    sorted_names: Vec<(String, u64)>,
}

impl SymbolIndex {
    /// Build a `SymbolIndex` from all code units in the given graph.
    ///
    /// Iterates over every unit once, populating the exact, lowercase, and
    /// sorted name collections.
    pub fn build(graph: &CodeGraph) -> Self {
        let mut exact: HashMap<String, Vec<u64>> = HashMap::new();
        let mut lowercase: HashMap<String, Vec<u64>> = HashMap::new();
        let mut sorted_names: Vec<(String, u64)> = Vec::with_capacity(graph.unit_count());

        for unit in graph.units() {
            exact.entry(unit.name.clone()).or_default().push(unit.id);

            let lower = unit.name.to_lowercase();
            lowercase.entry(lower.clone()).or_default().push(unit.id);

            sorted_names.push((lower, unit.id));
        }

        sorted_names.sort_by(|a, b| a.0.cmp(&b.0));

        Self {
            exact,
            lowercase,
            sorted_names,
        }
    }

    /// Look up unit IDs by exact symbol name (case-sensitive).
    ///
    /// Returns an empty slice if no units match.
    pub fn lookup_exact(&self, name: &str) -> &[u64] {
        self.exact.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Look up unit IDs whose lowercase name starts with the given prefix
    /// (case-insensitive).
    ///
    /// Uses binary search on sorted names for efficient prefix matching.
    pub fn lookup_prefix(&self, prefix: &str) -> Vec<u64> {
        let prefix_lower = prefix.to_lowercase();
        if prefix_lower.is_empty() {
            return self.sorted_names.iter().map(|(_, id)| *id).collect();
        }

        // Find the first entry >= prefix using binary search.
        let start = self
            .sorted_names
            .partition_point(|(name, _)| name.as_str() < prefix_lower.as_str());

        let mut results = Vec::new();
        for (name, id) in &self.sorted_names[start..] {
            if name.starts_with(&prefix_lower) {
                results.push(*id);
            } else {
                break;
            }
        }
        results
    }

    /// Look up unit IDs whose lowercase name contains the given substring
    /// (case-insensitive).
    ///
    /// This is a linear scan over the lowercase map; use `lookup_prefix` or
    /// `lookup_exact` when possible.
    pub fn lookup_contains(&self, substring: &str) -> Vec<u64> {
        let sub_lower = substring.to_lowercase();
        let mut results = Vec::new();
        for (name, ids) in &self.lowercase {
            if name.contains(&sub_lower) {
                results.extend(ids.iter().copied());
            }
        }
        results
    }

    /// Returns the number of distinct exact symbol names in the index.
    pub fn len(&self) -> usize {
        self.exact.len()
    }

    /// Returns `true` if the index contains no entries.
    pub fn is_empty(&self) -> bool {
        self.exact.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::CodeGraph;
    use crate::types::{CodeUnit, CodeUnitType, Language, Span};
    use std::path::PathBuf;

    fn make_unit(name: &str) -> CodeUnit {
        CodeUnit::new(
            CodeUnitType::Function,
            Language::Rust,
            name.to_string(),
            format!("mod::{name}"),
            PathBuf::from("src/lib.rs"),
            Span::new(1, 0, 10, 0),
        )
    }

    #[test]
    fn test_empty_index() {
        let graph = CodeGraph::default();
        let index = SymbolIndex::build(&graph);
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert_eq!(index.lookup_exact("foo"), &[] as &[u64]);
    }

    #[test]
    fn test_exact_lookup() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit("process_payment"));
        graph.add_unit(make_unit("validate_input"));

        let index = SymbolIndex::build(&graph);
        assert_eq!(index.len(), 2);
        assert_eq!(index.lookup_exact("process_payment"), &[0]);
        assert_eq!(index.lookup_exact("validate_input"), &[1]);
        assert_eq!(index.lookup_exact("nonexistent"), &[] as &[u64]);
    }

    #[test]
    fn test_prefix_lookup() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit("process_payment"));
        graph.add_unit(make_unit("process_refund"));
        graph.add_unit(make_unit("validate_input"));

        let index = SymbolIndex::build(&graph);
        let mut results = index.lookup_prefix("process");
        results.sort();
        assert_eq!(results, vec![0, 1]);
    }

    #[test]
    fn test_contains_lookup() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit("process_payment"));
        graph.add_unit(make_unit("validate_payment"));
        graph.add_unit(make_unit("send_email"));

        let index = SymbolIndex::build(&graph);
        let mut results = index.lookup_contains("payment");
        results.sort();
        assert_eq!(results, vec![0, 1]);
    }

    #[test]
    fn test_case_insensitive_prefix() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit("ProcessPayment"));
        graph.add_unit(make_unit("processRefund"));

        let index = SymbolIndex::build(&graph);
        let mut results = index.lookup_prefix("PROCESS");
        results.sort();
        assert_eq!(results, vec![0, 1]);
    }
}
