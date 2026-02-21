//! Coupling detection between code units.
//!
//! Detects temporal coupling (files that change together frequently) by
//! analysing co-change patterns in the commit history.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::history::ChangeHistory;
use crate::graph::CodeGraph;

/// Options for coupling detection.
#[derive(Debug, Clone)]
pub struct CouplingOptions {
    /// Minimum number of co-changes to consider a coupling (default 3).
    pub min_cochanges: usize,
    /// Minimum coupling strength to report (default 0.5).
    pub min_strength: f32,
    /// Maximum number of couplings to return (0 = unlimited).
    pub limit: usize,
}

impl Default for CouplingOptions {
    fn default() -> Self {
        Self {
            min_cochanges: 3,
            min_strength: 0.5,
            limit: 0,
        }
    }
}

/// The type of coupling detected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CouplingType {
    /// Files change together in the same commits.
    CoChange,
    /// Files share similar bug patterns.
    SharedBugs,
    /// An explicit graph edge already exists between units in these files.
    ExplicitEdge,
}

impl std::fmt::Display for CouplingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CoChange => write!(f, "co-change"),
            Self::SharedBugs => write!(f, "shared-bugs"),
            Self::ExplicitEdge => write!(f, "explicit-edge"),
        }
    }
}

/// A detected coupling between two file paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coupling {
    /// First file path.
    pub path_a: PathBuf,
    /// Second file path.
    pub path_b: PathBuf,
    /// Number of commits where both files were changed.
    pub cochange_count: usize,
    /// Coupling strength (0.0 to 1.0).
    pub strength: f32,
    /// The type of coupling detected.
    pub coupling_type: CouplingType,
}

/// Detects temporal coupling patterns from change history.
#[derive(Debug, Clone)]
pub struct CouplingDetector {
    /// Configuration options.
    options: CouplingOptions,
}

impl CouplingDetector {
    /// Create a new coupling detector with default options.
    pub fn new() -> Self {
        Self {
            options: CouplingOptions::default(),
        }
    }

    /// Create a new coupling detector with custom options.
    pub fn with_options(options: CouplingOptions) -> Self {
        Self { options }
    }

    /// Detect all couplings from the change history.
    ///
    /// Optionally cross-references with a [`CodeGraph`] to annotate couplings
    /// that already have explicit edges.
    pub fn detect_all(&self, history: &ChangeHistory, graph: Option<&CodeGraph>) -> Vec<Coupling> {
        let matrix = self.build_cochange_matrix(history);
        let mut couplings = Vec::new();

        for ((path_a, path_b), count) in &matrix {
            if *count < self.options.min_cochanges {
                continue;
            }

            // Strength = co-change count / max(changes(a), changes(b)).
            let changes_a = history.change_count(path_a).max(1);
            let changes_b = history.change_count(path_b).max(1);
            let max_changes = changes_a.max(changes_b) as f32;
            let strength = (*count as f32 / max_changes).min(1.0);

            if strength < self.options.min_strength {
                continue;
            }

            let coupling_type = if let Some(g) = graph {
                if self.has_explicit_edge(g, path_a, path_b) {
                    CouplingType::ExplicitEdge
                } else {
                    CouplingType::CoChange
                }
            } else {
                CouplingType::CoChange
            };

            couplings.push(Coupling {
                path_a: path_a.clone(),
                path_b: path_b.clone(),
                cochange_count: *count,
                strength,
                coupling_type,
            });
        }

        // Sort by strength descending.
        couplings.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if self.options.limit > 0 {
            couplings.truncate(self.options.limit);
        }

        couplings
    }

    /// Build a co-change matrix from the history.
    ///
    /// For each commit that touches multiple files, counts the number of
    /// commits where each pair of files was changed together.
    fn build_cochange_matrix(&self, history: &ChangeHistory) -> HashMap<(PathBuf, PathBuf), usize> {
        let mut matrix: HashMap<(PathBuf, PathBuf), usize> = HashMap::new();

        for commit_id in history.all_commits() {
            let changes = history.files_in_commit(commit_id);
            let mut paths: Vec<&Path> = changes.iter().map(|c| c.path.as_path()).collect();
            paths.sort();
            paths.dedup();

            // For each pair of files in this commit, increment the co-change count.
            for i in 0..paths.len() {
                for j in (i + 1)..paths.len() {
                    let key = (paths[i].to_path_buf(), paths[j].to_path_buf());
                    *matrix.entry(key).or_insert(0) += 1;
                }
            }
        }

        matrix
    }

    /// Check if there is an explicit graph edge between units in two files.
    fn has_explicit_edge(&self, graph: &CodeGraph, path_a: &Path, path_b: &Path) -> bool {
        let units_a = self.find_units_for_path(graph, path_a);
        let units_b = self.find_units_for_path(graph, path_b);

        for &a_id in &units_a {
            for edge in graph.edges_from(a_id) {
                if units_b.contains(&edge.target_id) {
                    return true;
                }
            }
        }
        false
    }

    /// Find all unit IDs that belong to a given file path.
    fn find_units_for_path(&self, graph: &CodeGraph, path: &Path) -> Vec<u64> {
        graph
            .find_units_by_path(path)
            .iter()
            .map(|u| u.id)
            .collect()
    }
}

impl Default for CouplingDetector {
    fn default() -> Self {
        Self::new()
    }
}
