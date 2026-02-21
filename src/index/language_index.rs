//! Index by language.
//!
//! Groups code units by their [`Language`] for fast filtered lookups.
//! Building the index is O(n); lookups are O(1).

use std::collections::HashMap;

use crate::graph::CodeGraph;
use crate::types::Language;

/// Index that groups code units by language.
#[derive(Debug, Clone, Default)]
pub struct LanguageIndex {
    by_language: HashMap<Language, Vec<u64>>,
}

impl LanguageIndex {
    /// Build a `LanguageIndex` from all code units in the given graph.
    pub fn build(graph: &CodeGraph) -> Self {
        let mut by_language: HashMap<Language, Vec<u64>> = HashMap::new();

        for unit in graph.units() {
            by_language.entry(unit.language).or_default().push(unit.id);
        }

        Self { by_language }
    }

    /// Look up all unit IDs written in the given language.
    ///
    /// Returns an empty slice if no units match.
    pub fn lookup(&self, language: Language) -> &[u64] {
        self.by_language
            .get(&language)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns the count of units for the given language.
    pub fn count(&self, language: Language) -> usize {
        self.by_language
            .get(&language)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Returns all languages present in the index.
    pub fn languages(&self) -> Vec<Language> {
        self.by_language.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::CodeGraph;
    use crate::types::{CodeUnit, CodeUnitType, Language, Span};
    use std::path::PathBuf;

    fn make_unit(language: Language) -> CodeUnit {
        CodeUnit::new(
            CodeUnitType::Function,
            language,
            "test_fn".to_string(),
            "mod::test_fn".to_string(),
            PathBuf::from("src/lib.rs"),
            Span::new(1, 0, 10, 0),
        )
    }

    #[test]
    fn test_empty_index() {
        let graph = CodeGraph::default();
        let index = LanguageIndex::build(&graph);
        assert_eq!(index.count(Language::Rust), 0);
        assert_eq!(index.lookup(Language::Rust), &[] as &[u64]);
        assert!(index.languages().is_empty());
    }

    #[test]
    fn test_grouped_lookup() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit(Language::Rust));
        graph.add_unit(make_unit(Language::Rust));
        graph.add_unit(make_unit(Language::Python));
        graph.add_unit(make_unit(Language::TypeScript));

        let index = LanguageIndex::build(&graph);
        assert_eq!(index.count(Language::Rust), 2);
        assert_eq!(index.count(Language::Python), 1);
        assert_eq!(index.count(Language::TypeScript), 1);
        assert_eq!(index.count(Language::Go), 0);

        assert_eq!(index.lookup(Language::Rust), &[0, 1]);
        assert_eq!(index.lookup(Language::Python), &[2]);
    }

    #[test]
    fn test_languages_list() {
        let mut graph = CodeGraph::default();
        graph.add_unit(make_unit(Language::Rust));
        graph.add_unit(make_unit(Language::Go));

        let index = LanguageIndex::build(&graph);
        let mut langs = index.languages();
        langs.sort_by_key(|l| *l as u8);
        assert_eq!(langs, vec![Language::Rust, Language::Go]);
    }
}
