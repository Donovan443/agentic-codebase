//! Pattern aggregation.
//!
//! Types and utilities for extracting, categorising, and aggregating
//! usage patterns from code graphs.

use serde::{Deserialize, Serialize};

/// The category of a usage pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternCategory {
    /// A structural design pattern (e.g., Singleton, Factory).
    DesignPattern,
    /// An API usage pattern (e.g., common call sequences).
    ApiUsage,
    /// An error handling pattern (e.g., retry with backoff).
    ErrorHandling,
    /// A concurrency pattern (e.g., producer-consumer).
    Concurrency,
    /// A data transformation pattern (e.g., map-filter-reduce).
    DataTransform,
    /// A testing pattern (e.g., AAA, fixture-based).
    TestingPattern,
}

impl std::fmt::Display for PatternCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DesignPattern => write!(f, "design-pattern"),
            Self::ApiUsage => write!(f, "api-usage"),
            Self::ErrorHandling => write!(f, "error-handling"),
            Self::Concurrency => write!(f, "concurrency"),
            Self::DataTransform => write!(f, "data-transform"),
            Self::TestingPattern => write!(f, "testing-pattern"),
        }
    }
}

/// Complexity bucket for classifying code units.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplexityBucket {
    /// Cyclomatic complexity 1-5.
    Low,
    /// Cyclomatic complexity 6-15.
    Medium,
    /// Cyclomatic complexity 16-30.
    High,
    /// Cyclomatic complexity > 30.
    VeryHigh,
}

impl ComplexityBucket {
    /// Classify a complexity value into a bucket.
    pub fn from_complexity(complexity: u32) -> Self {
        match complexity {
            0..=5 => Self::Low,
            6..=15 => Self::Medium,
            16..=30 => Self::High,
            _ => Self::VeryHigh,
        }
    }
}

impl std::fmt::Display for ComplexityBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::VeryHigh => write!(f, "very-high"),
        }
    }
}

/// Quality assessment of a pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternQuality {
    /// Well-established, widely used pattern.
    Established,
    /// Emerging pattern, growing in usage.
    Emerging,
    /// Declining pattern, being replaced.
    Declining,
    /// Unknown or unclassified quality.
    Unknown,
}

impl std::fmt::Display for PatternQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Established => write!(f, "established"),
            Self::Emerging => write!(f, "emerging"),
            Self::Declining => write!(f, "declining"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Structural signature of a usage pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSignature {
    /// Language this pattern was observed in.
    pub language: String,
    /// Category of the pattern.
    pub category: PatternCategory,
    /// Structural hash for deduplication.
    pub structure_hash: String,
    /// Human-readable description of the pattern structure.
    pub description: String,
}

/// A detected usage pattern with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    /// Pattern signature for identification.
    pub signature: PatternSignature,
    /// How many times this pattern was observed.
    pub occurrence_count: u32,
    /// Quality assessment.
    pub quality: PatternQuality,
    /// Confidence (0.0 to 1.0).
    pub confidence: f32,
    /// Typical complexity bucket for code using this pattern.
    pub typical_complexity: ComplexityBucket,
    /// Example file paths where this pattern was found.
    pub example_paths: Vec<String>,
}

/// Extracts usage patterns from a code graph.
///
/// Currently a placeholder implementation that demonstrates the type system
/// without performing deep pattern mining.
#[derive(Debug, Clone)]
pub struct PatternExtractor {
    /// Minimum number of occurrences to report a pattern.
    min_occurrences: u32,
}

impl PatternExtractor {
    /// Create a new pattern extractor.
    pub fn new() -> Self {
        Self { min_occurrences: 2 }
    }

    /// Create a pattern extractor with a custom minimum occurrence threshold.
    pub fn with_min_occurrences(min_occurrences: u32) -> Self {
        Self { min_occurrences }
    }

    /// Extract patterns from a code graph.
    ///
    /// This is a placeholder implementation. In a full implementation,
    /// this would perform structural pattern mining on the graph.
    /// Currently returns an empty list.
    pub fn extract(&self, _graph: &crate::graph::CodeGraph) -> Vec<UsagePattern> {
        // Placeholder: real implementation would mine the graph for patterns.
        tracing::debug!(
            "PatternExtractor::extract called (min_occurrences={}); returning empty (placeholder).",
            self.min_occurrences
        );
        Vec::new()
    }

    /// Get the minimum occurrence threshold.
    pub fn min_occurrences(&self) -> u32 {
        self.min_occurrences
    }
}

impl Default for PatternExtractor {
    fn default() -> Self {
        Self::new()
    }
}
