//! Index by code unit type.
//!
//! Groups code units by their [`CodeUnitType`] for fast filtered lookups.
//! Building the index is O(n) in the number of units; lookups are O(1).

use std::collections::HashMap;

use crate::graph::CodeGraph;
use crate::types::CodeUnitType;

/// Index that groups code units by their type.
#[derive(Debug, Clone, Default)]
pub struct TypeIndex {
    by_type: HashMap<CodeUnitType, Vec<u64>>,
}

impl TypeIndex {
    /// Build a `TypeIndex` from all code units in the given graph.
    pub fn build(graph: &CodeGraph) -> Self {
        let mut by_type: HashMap<CodeUnitType, Vec<u64>> = HashMap::new();

        for unit in graph.units() {
            by_type.entry(unit.unit_type).or_default().push(unit.id);
        }

        Self { by_type }
    }

    /// Look up all unit IDs of the given type.
    ///
    /// Returns an empty slice if no units match.
    pub fn lookup(&self, unit_type: CodeUnitType) -> &[u64] {
        self.by_type
            .get(&unit_type)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns the count of units of the given type.
    pub fn count(&self, unit_type: CodeUnitType) -> usize {
        self.by_type.get(&unit_type).map(|v| v.len()).unwrap_or(0)
    }

    /// Returns all code unit types present in the index.
    pub fn types(&self) -> Vec<CodeUnitType> {
        self.by_type.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::CodeGraph;
    use crate::types::{CodeUnit, CodeUnitType, Language, Span};
    use std::path::PathBuf;

    fn make_unit(unit_type: CodeUnitType) -> CodeUnit {
        CodeUnit::new(
            unit_type,
            Language::Rust,
            "test_unit".to_string(),
            "mod::test_unit".to_string(),
            PathBuf::from("src/lib.rs"),
            Span::new(1, 0, 10, 0),
        )
    }

    #[test]
    fn test_empty_index() {
        let graph = CodeGraph::default();
        let index = TypeIndex::build(&graph);
        assert_eq!(index.count(CodeUnitType::Function), 0);
        assert_eq!(index.lookup(CodeUnitType::Function), &[] as &[u64]);
        assert!(index.types().is_empty());
    }

    #[test]
    fn test_grouped_lookup() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit(CodeUnitType::Function));
        graph.add_unit(make_unit(CodeUnitType::Function));
        graph.add_unit(make_unit(CodeUnitType::Module));
        graph.add_unit(make_unit(CodeUnitType::Type));

        let index = TypeIndex::build(&graph);
        assert_eq!(index.count(CodeUnitType::Function), 2);
        assert_eq!(index.count(CodeUnitType::Module), 1);
        assert_eq!(index.count(CodeUnitType::Type), 1);
        assert_eq!(index.count(CodeUnitType::Import), 0);

        assert_eq!(index.lookup(CodeUnitType::Function), &[0, 1]);
        assert_eq!(index.lookup(CodeUnitType::Module), &[2]);
    }

    #[test]
    fn test_types_list() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit(CodeUnitType::Function));
        graph.add_unit(make_unit(CodeUnitType::Trait));

        let index = TypeIndex::build(&graph);
        let mut types = index.types();
        types.sort_by_key(|t| *t as u8);
        assert_eq!(types, vec![CodeUnitType::Function, CodeUnitType::Trait]);
    }
}
