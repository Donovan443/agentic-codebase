//! Change history tracking via git.
//!
//! Provides types and data structures for tracking file-level change history.
//! The [`ChangeHistory`] struct stores changes indexed by file path, commit,
//! and chronological order for efficient querying.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The type of change recorded for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeType {
    /// File was added for the first time.
    Add,
    /// File was modified in place.
    Modify,
    /// File was deleted.
    Delete,
    /// File was renamed (old path stored in `old_path` field of [`FileChange`]).
    Rename,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "add"),
            Self::Modify => write!(f, "modify"),
            Self::Delete => write!(f, "delete"),
            Self::Rename => write!(f, "rename"),
        }
    }
}

/// A single recorded change to a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// The file path affected (relative to repo root).
    pub path: PathBuf,
    /// The type of change.
    pub change_type: ChangeType,
    /// The commit hash (abbreviated or full).
    pub commit_id: String,
    /// Unix timestamp (seconds) of the commit.
    pub timestamp: u64,
    /// Author name or email.
    pub author: String,
    /// Whether the commit message indicates a bug fix.
    pub is_bugfix: bool,
    /// Number of lines added in this change.
    pub lines_added: u32,
    /// Number of lines deleted in this change.
    pub lines_deleted: u32,
    /// Previous path, if this was a rename.
    pub old_path: Option<PathBuf>,
}

/// Options for building or querying change history.
#[derive(Debug, Clone)]
pub struct HistoryOptions {
    /// Maximum number of commits to scan.
    pub max_commits: usize,
    /// Only include changes after this timestamp (0 = no limit).
    pub since_timestamp: u64,
    /// Only include changes before this timestamp (0 = no limit).
    pub until_timestamp: u64,
    /// Only include changes to these paths (empty = all).
    pub path_filter: Vec<PathBuf>,
}

impl Default for HistoryOptions {
    fn default() -> Self {
        Self {
            max_commits: 10000,
            since_timestamp: 0,
            until_timestamp: 0,
            path_filter: Vec::new(),
        }
    }
}

/// Aggregated change history for a codebase.
///
/// Stores changes indexed by file path and commit for efficient lookup.
/// Does not require git integration at runtime; data can be pre-populated
/// from any source (git, manual, tests).
#[derive(Debug, Clone, Default)]
pub struct ChangeHistory {
    /// Changes indexed by file path.
    by_path: HashMap<PathBuf, Vec<FileChange>>,
    /// All changes in chronological order.
    chronological: Vec<FileChange>,
    /// Changes indexed by commit ID.
    commits: HashMap<String, Vec<FileChange>>,
}

impl ChangeHistory {
    /// Create a new empty change history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a change to the history.
    ///
    /// The change is indexed by its path and commit ID, and appended
    /// to the chronological list.
    pub fn add_change(&mut self, change: FileChange) {
        self.by_path
            .entry(change.path.clone())
            .or_default()
            .push(change.clone());
        self.commits
            .entry(change.commit_id.clone())
            .or_default()
            .push(change.clone());
        self.chronological.push(change);
    }

    /// Get all changes for a given file path.
    pub fn changes_for_path(&self, path: &Path) -> &[FileChange] {
        self.by_path.get(path).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get all files changed in a given commit.
    pub fn files_in_commit(&self, commit_id: &str) -> &[FileChange] {
        self.commits
            .get(commit_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the total number of changes recorded for a path.
    pub fn change_count(&self, path: &Path) -> usize {
        self.by_path.get(path).map(|v| v.len()).unwrap_or(0)
    }

    /// Get the number of bugfix changes for a path.
    pub fn bugfix_count(&self, path: &Path) -> usize {
        self.by_path
            .get(path)
            .map(|v| v.iter().filter(|c| c.is_bugfix).count())
            .unwrap_or(0)
    }

    /// Get all unique commit IDs.
    pub fn all_commits(&self) -> Vec<&str> {
        self.commits.keys().map(|s| s.as_str()).collect()
    }

    /// Get all changes in chronological order.
    pub fn chronological(&self) -> &[FileChange] {
        &self.chronological
    }

    /// Get all tracked file paths.
    pub fn all_paths(&self) -> Vec<&Path> {
        self.by_path.keys().map(|p| p.as_path()).collect()
    }

    /// Get unique authors for a path.
    pub fn authors_for_path(&self, path: &Path) -> Vec<String> {
        let mut authors: Vec<String> = self
            .by_path
            .get(path)
            .map(|v| v.iter().map(|c| c.author.clone()).collect())
            .unwrap_or_default();
        authors.sort();
        authors.dedup();
        authors
    }

    /// Get total churn (lines added + deleted) for a path.
    pub fn total_churn(&self, path: &Path) -> u64 {
        self.by_path
            .get(path)
            .map(|v| {
                v.iter()
                    .map(|c| c.lines_added as u64 + c.lines_deleted as u64)
                    .sum()
            })
            .unwrap_or(0)
    }

    /// Get the total number of changes across all files.
    pub fn total_changes(&self) -> usize {
        self.chronological.len()
    }

    /// Get the total number of unique commits.
    pub fn total_commits(&self) -> usize {
        self.commits.len()
    }

    /// Get the most recent change timestamp for a path, or 0 if none.
    pub fn latest_timestamp(&self, path: &Path) -> u64 {
        self.by_path
            .get(path)
            .and_then(|v| v.iter().map(|c| c.timestamp).max())
            .unwrap_or(0)
    }

    /// Get the oldest change timestamp for a path, or 0 if none.
    pub fn oldest_timestamp(&self, path: &Path) -> u64 {
        self.by_path
            .get(path)
            .and_then(|v| v.iter().map(|c| c.timestamp).min())
            .unwrap_or(0)
    }
}
