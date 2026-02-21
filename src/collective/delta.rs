//! Delta compression for collective sync.
//!
//! Provides types and utilities for creating, compressing, and decompressing
//! deltas that represent knowledge to be shared across codebases.

use serde::{Deserialize, Serialize};

use crate::types::AcbError;
use crate::types::AcbResult;

/// Category of mistake that can be reported.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MistakeCategory {
    /// A bug pattern that was discovered.
    BugPattern,
    /// A performance anti-pattern.
    PerformanceAntiPattern,
    /// A security vulnerability pattern.
    SecurityVulnerability,
    /// An API misuse pattern.
    ApiMisuse,
    /// A code smell or maintainability issue.
    CodeSmell,
}

impl std::fmt::Display for MistakeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BugPattern => write!(f, "bug-pattern"),
            Self::PerformanceAntiPattern => write!(f, "performance-anti-pattern"),
            Self::SecurityVulnerability => write!(f, "security-vulnerability"),
            Self::ApiMisuse => write!(f, "api-misuse"),
            Self::CodeSmell => write!(f, "code-smell"),
        }
    }
}

/// A report of a coding mistake or anti-pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistakeReport {
    /// Category of the mistake.
    pub category: MistakeCategory,
    /// Short description of what went wrong.
    pub description: String,
    /// The pattern signature that triggered the detection.
    pub pattern_signature: String,
    /// Suggested fix or improvement.
    pub suggestion: String,
    /// Severity (0.0 = informational, 1.0 = critical).
    pub severity: f32,
}

/// A collective delta containing patterns and mistakes to share.
///
/// Deltas are the unit of exchange for collective intelligence.
/// They can be compressed for efficient transmission and storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveDelta {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Unique identifier for this delta (blake3 hash of contents).
    pub delta_id: String,
    /// Unix timestamp when this delta was created.
    pub created_at: u64,
    /// Source codebase identifier (anonymised).
    pub source_id: String,
    /// Patterns discovered in this delta.
    pub patterns: Vec<DeltaPattern>,
    /// Mistakes reported in this delta.
    pub mistakes: Vec<MistakeReport>,
}

/// A pattern included in a collective delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaPattern {
    /// Pattern name or identifier.
    pub name: String,
    /// Language this pattern applies to.
    pub language: String,
    /// Structural signature of the pattern.
    pub signature: String,
    /// How many times this pattern was observed.
    pub occurrence_count: u32,
    /// Confidence that this is a meaningful pattern (0.0 to 1.0).
    pub confidence: f32,
}

impl CollectiveDelta {
    /// Create a new empty delta.
    pub fn new(source_id: String) -> Self {
        let now = crate::types::now_micros();
        Self {
            version: 1,
            delta_id: String::new(),
            created_at: now,
            source_id,
            patterns: Vec::new(),
            mistakes: Vec::new(),
        }
    }

    /// Add a pattern to the delta.
    pub fn add_pattern(&mut self, pattern: DeltaPattern) {
        self.patterns.push(pattern);
    }

    /// Add a mistake report to the delta.
    pub fn add_mistake(&mut self, mistake: MistakeReport) {
        self.mistakes.push(mistake);
    }

    /// Compute and set the delta ID based on content hash.
    pub fn finalize(&mut self) -> AcbResult<()> {
        let json = serde_json::to_vec(self)
            .map_err(|e| AcbError::Compression(format!("Failed to serialize delta: {}", e)))?;
        let hash = blake3::hash(&json);
        self.delta_id = hash.to_hex().to_string();
        Ok(())
    }

    /// Compress the delta to a byte vector using lz4.
    ///
    /// Serialises to JSON and then compresses with lz4_flex.
    pub fn compress(&self) -> AcbResult<Vec<u8>> {
        let json = serde_json::to_vec(self)
            .map_err(|e| AcbError::Compression(format!("Failed to serialize delta: {}", e)))?;
        let compressed = lz4_flex::compress_prepend_size(&json);
        Ok(compressed)
    }

    /// Decompress a delta from a compressed byte vector.
    ///
    /// Decompresses with lz4_flex and deserialises from JSON.
    pub fn decompress(data: &[u8]) -> AcbResult<Self> {
        let decompressed = lz4_flex::decompress_size_prepended(data)
            .map_err(|e| AcbError::Compression(format!("Failed to decompress delta: {}", e)))?;
        let delta: CollectiveDelta = serde_json::from_slice(&decompressed)
            .map_err(|e| AcbError::Compression(format!("Failed to deserialize delta: {}", e)))?;
        Ok(delta)
    }

    /// Returns true if this delta has any content to share.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty() && self.mistakes.is_empty()
    }
}
