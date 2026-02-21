//! Privacy-preserving extraction.
//!
//! Determines which data items are safe to share with the collective
//! intelligence network and which must remain private.

use serde::{Deserialize, Serialize};

/// Data that is safe to share with the collective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shareable {
    /// Structural pattern signature (no actual code).
    PatternSignature(String),
    /// Aggregated complexity statistics.
    ComplexityStats {
        /// Language.
        language: String,
        /// Average complexity.
        avg_complexity: u32,
        /// Total functions analysed.
        function_count: u32,
    },
    /// Anonymous mistake category counts.
    MistakeCounts {
        /// Category name.
        category: String,
        /// Number of occurrences.
        count: u32,
    },
    /// Language usage distribution.
    LanguageDistribution {
        /// Language name.
        language: String,
        /// Percentage of codebase (0-100).
        percentage: u32,
    },
}

/// Data that must NOT be shared with the collective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NonShareable {
    /// Actual source code.
    SourceCode,
    /// File paths (may reveal project structure).
    FilePaths,
    /// Symbol names (may reveal proprietary API names).
    SymbolNames,
    /// Commit messages (may contain sensitive info).
    CommitMessages,
    /// Author names or emails.
    AuthorIdentity,
    /// Content hashes (could be used to fingerprint code).
    ContentHashes,
    /// API keys, tokens, or credentials found in code.
    Credentials,
}

/// Check whether a data item is safe to share.
///
/// Returns `true` if the item is shareable, `false` if it must stay private.
///
/// # Rules
///
/// Shareable data:
/// - Structural pattern signatures (abstracted, no real code)
/// - Aggregated statistics (complexity averages, counts)
/// - Anonymous mistake category counts
/// - Language usage distributions
///
/// Non-shareable data:
/// - Source code
/// - File paths
/// - Symbol names
/// - Commit messages
/// - Author identities
/// - Content hashes
/// - Credentials
pub fn is_shareable(item: &ShareableCheck) -> bool {
    match item {
        ShareableCheck::PatternSignature(_) => true,
        ShareableCheck::AggregateStats { .. } => true,
        ShareableCheck::MistakeCount { .. } => true,
        ShareableCheck::LanguageDistribution { .. } => true,
        ShareableCheck::SourceCode(_) => false,
        ShareableCheck::FilePath(_) => false,
        ShareableCheck::SymbolName(_) => false,
        ShareableCheck::CommitMessage(_) => false,
        ShareableCheck::AuthorIdentity(_) => false,
        ShareableCheck::ContentHash(_) => false,
        ShareableCheck::RawText(text) => !looks_like_credentials(text),
    }
}

/// An item to check for shareability.
#[derive(Debug, Clone)]
pub enum ShareableCheck {
    /// A structural pattern signature.
    PatternSignature(String),
    /// Aggregated statistics.
    AggregateStats {
        /// Statistic name.
        name: String,
        /// Statistic value.
        value: f64,
    },
    /// A mistake count.
    MistakeCount {
        /// Category.
        category: String,
        /// Count.
        count: u32,
    },
    /// Language distribution.
    LanguageDistribution {
        /// Language name.
        language: String,
        /// Percentage.
        percentage: u32,
    },
    /// Raw source code.
    SourceCode(String),
    /// A file path.
    FilePath(String),
    /// A symbol name.
    SymbolName(String),
    /// A commit message.
    CommitMessage(String),
    /// An author identity.
    AuthorIdentity(String),
    /// A content hash.
    ContentHash(String),
    /// Generic text to check for credentials.
    RawText(String),
}

/// Heuristic check for credential-like strings.
fn looks_like_credentials(text: &str) -> bool {
    let lower = text.to_lowercase();
    let credential_indicators = [
        "api_key",
        "apikey",
        "api-key",
        "secret",
        "password",
        "passwd",
        "token",
        "bearer",
        "authorization",
        "aws_access_key",
        "private_key",
    ];
    credential_indicators
        .iter()
        .any(|indicator| lower.contains(indicator))
}

/// Filter a list of items, keeping only those that are shareable.
pub fn filter_shareable(items: &[ShareableCheck]) -> Vec<&ShareableCheck> {
    items.iter().filter(|item| is_shareable(item)).collect()
}
