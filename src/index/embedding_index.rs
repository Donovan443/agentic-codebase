//! Vector similarity index for semantic search.
//!
//! Brute-force cosine similarity with optional pre-filtering.
//! Entries are pre-copied from the graph for cache-friendly iteration
//! during search.

use crate::graph::CodeGraph;

/// Threshold below which a vector's norm is considered zero.
const NORM_EPSILON: f32 = 1e-10;

/// Brute-force vector index for semantic search.
///
/// Stores (unit_id, feature_vec) pairs copied from the graph for cache
/// locality during sequential scan.
#[derive(Debug, Clone)]
pub struct EmbeddingIndex {
    /// Unit IDs and their feature vectors (pre-copied for cache locality).
    entries: Vec<(u64, Vec<f32>)>,
    /// Dimensionality of the feature vectors.
    dimension: usize,
}

/// A single search result with its similarity score.
#[derive(Debug, Clone)]
pub struct EmbeddingMatch {
    /// The code unit ID.
    pub unit_id: u64,
    /// Cosine similarity score in [-1.0, 1.0].
    pub score: f32,
}

impl EmbeddingIndex {
    /// Build an `EmbeddingIndex` from all code units in the given graph.
    ///
    /// Only includes units whose feature vectors have the expected dimension
    /// and are not all-zero (i.e., have a non-negligible norm).
    pub fn build(graph: &CodeGraph) -> Self {
        let dimension = graph.dimension();
        let mut entries = Vec::with_capacity(graph.unit_count());

        for unit in graph.units() {
            if unit.feature_vec.len() == dimension {
                let norm = vec_norm(&unit.feature_vec);
                if norm > NORM_EPSILON {
                    entries.push((unit.id, unit.feature_vec.clone()));
                }
            }
        }

        Self { entries, dimension }
    }

    /// Search for the most similar vectors to `query`.
    ///
    /// Returns at most `top_k` results with cosine similarity >= `min_similarity`,
    /// ordered by descending score.
    ///
    /// If the query vector dimension does not match the index dimension, or the
    /// query vector has near-zero norm, returns an empty vector.
    pub fn search(&self, query: &[f32], top_k: usize, min_similarity: f32) -> Vec<EmbeddingMatch> {
        if query.len() != self.dimension {
            return Vec::new();
        }

        let query_norm = vec_norm(query);
        if query_norm < NORM_EPSILON {
            return Vec::new();
        }

        let mut results: Vec<EmbeddingMatch> = self
            .entries
            .iter()
            .filter_map(|(id, vec)| {
                let score = cosine_similarity(query, vec, query_norm);
                if score >= min_similarity {
                    Some(EmbeddingMatch {
                        unit_id: *id,
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by descending score; break ties by unit_id ascending.
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.unit_id.cmp(&b.unit_id))
        });

        results.truncate(top_k);
        results
    }

    /// Returns the dimensionality of vectors in this index.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Returns the number of indexed entries (units with valid feature vectors).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the index contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for EmbeddingIndex {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            dimension: crate::types::DEFAULT_DIMENSION,
        }
    }
}

/// Compute the L2 norm of a vector.
fn vec_norm(v: &[f32]) -> f32 {
    let sum: f32 = v.iter().map(|x| x * x).sum();
    sum.sqrt()
}

/// Compute cosine similarity between `a` and `b`, given the pre-computed
/// norm of `a`. Returns 0.0 if `b` has near-zero norm.
fn cosine_similarity(a: &[f32], b: &[f32], a_norm: f32) -> f32 {
    let b_norm = vec_norm(b);
    if b_norm < NORM_EPSILON || a_norm < NORM_EPSILON {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    dot / (a_norm * b_norm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::CodeGraph;
    use crate::types::{CodeUnit, CodeUnitType, Language, Span};
    use std::path::PathBuf;

    fn make_unit_with_vec(feature_vec: Vec<f32>) -> CodeUnit {
        let mut unit = CodeUnit::new(
            CodeUnitType::Function,
            Language::Rust,
            "test_fn".to_string(),
            "mod::test_fn".to_string(),
            PathBuf::from("src/lib.rs"),
            Span::new(1, 0, 10, 0),
        );
        unit.feature_vec = feature_vec;
        unit
    }

    #[test]
    fn test_empty_index() {
        let graph = CodeGraph::default();
        let index = EmbeddingIndex::build(&graph);
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert_eq!(index.dimension(), 256);
    }

    #[test]
    fn test_zero_vectors_excluded() {
        let dim = 4;
        let mut graph = CodeGraph::new(dim);
        // All-zero vector should be excluded.
        graph.add_unit(make_unit_with_vec(vec![0.0; dim]));
        // Non-zero vector should be included.
        graph.add_unit(make_unit_with_vec(vec![1.0, 0.0, 0.0, 0.0]));

        let index = EmbeddingIndex::build(&graph);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_search_identical_vector() {
        let dim = 4;
        let mut graph = CodeGraph::new(dim);
        graph.add_unit(make_unit_with_vec(vec![1.0, 0.0, 0.0, 0.0]));
        graph.add_unit(make_unit_with_vec(vec![0.0, 1.0, 0.0, 0.0]));

        let index = EmbeddingIndex::build(&graph);

        // Search for the same vector as unit 0.
        let results = index.search(&[1.0, 0.0, 0.0, 0.0], 10, 0.0);
        assert_eq!(results.len(), 2);
        // First result should be the identical vector with score ~1.0.
        assert_eq!(results[0].unit_id, 0);
        assert!((results[0].score - 1.0).abs() < 1e-6);
        // Second result is orthogonal with score ~0.0.
        assert_eq!(results[1].unit_id, 1);
        assert!(results[1].score.abs() < 1e-6);
    }

    #[test]
    fn test_search_top_k() {
        let dim = 4;
        let mut graph = CodeGraph::new(dim);
        graph.add_unit(make_unit_with_vec(vec![1.0, 0.0, 0.0, 0.0]));
        graph.add_unit(make_unit_with_vec(vec![0.9, 0.1, 0.0, 0.0]));
        graph.add_unit(make_unit_with_vec(vec![0.5, 0.5, 0.0, 0.0]));

        let index = EmbeddingIndex::build(&graph);
        let results = index.search(&[1.0, 0.0, 0.0, 0.0], 2, 0.0);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_min_similarity() {
        let dim = 4;
        let mut graph = CodeGraph::new(dim);
        graph.add_unit(make_unit_with_vec(vec![1.0, 0.0, 0.0, 0.0]));
        graph.add_unit(make_unit_with_vec(vec![0.0, 1.0, 0.0, 0.0]));

        let index = EmbeddingIndex::build(&graph);
        let results = index.search(&[1.0, 0.0, 0.0, 0.0], 10, 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].unit_id, 0);
    }

    #[test]
    fn test_search_wrong_dimension() {
        let dim = 4;
        let mut graph = CodeGraph::new(dim);
        graph.add_unit(make_unit_with_vec(vec![1.0, 0.0, 0.0, 0.0]));

        let index = EmbeddingIndex::build(&graph);
        // Query with wrong dimension should return empty.
        let results = index.search(&[1.0, 0.0], 10, 0.0);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_zero_query() {
        let dim = 4;
        let mut graph = CodeGraph::new(dim);
        graph.add_unit(make_unit_with_vec(vec![1.0, 0.0, 0.0, 0.0]));

        let index = EmbeddingIndex::build(&graph);
        let results = index.search(&[0.0; 4], 10, 0.0);
        assert!(results.is_empty());
    }
}
