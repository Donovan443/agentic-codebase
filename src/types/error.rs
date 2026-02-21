//! Error types for the AgenticCodebase system.
//!
//! All errors are typed using `thiserror` and propagated through `AcbResult`.

use std::path::PathBuf;
use thiserror::Error;

/// All error types that can occur in the AgenticCodebase system.
#[derive(Error, Debug)]
pub enum AcbError {
    /// Invalid magic bytes in file header.
    #[error("Invalid magic bytes in file header")]
    InvalidMagic,

    /// Unsupported format version.
    #[error("Unsupported format version: {0}")]
    UnsupportedVersion(u32),

    /// Code unit ID not found.
    #[error("Code unit ID {0} not found")]
    UnitNotFound(u64),

    /// Edge references an invalid code unit.
    #[error("Edge references invalid code unit: {0}")]
    InvalidEdgeTarget(u64),

    /// Self-edges are not allowed.
    #[error("Self-edge not allowed on unit {0}")]
    SelfEdge(u64),

    /// Symbol name exceeds maximum length.
    #[error("Symbol name too long: {len} > {max}")]
    NameTooLong {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },

    /// Path exceeds maximum length.
    #[error("Path too long: {len} > {max}")]
    PathTooLong {
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },

    /// Feature vector dimension does not match expected dimension.
    #[error("Feature vector dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch {
        /// Expected dimension.
        expected: usize,
        /// Actual dimension.
        got: usize,
    },

    /// Too many edges for a single code unit.
    #[error("Maximum edges per unit exceeded: {0}")]
    TooManyEdges(u32),

    /// Path does not exist on disk.
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    /// Language is not supported.
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    /// A parsing error occurred.
    #[error("Parse error in {path}: {message}")]
    ParseError {
        /// File that caused the error.
        path: PathBuf,
        /// Description of the parse failure.
        message: String,
    },

    /// A semantic analysis error occurred.
    #[error("Semantic error: {0}")]
    SemanticError(String),

    /// A git-related error occurred.
    #[error("Git error: {0}")]
    GitError(String),

    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A compression or decompression error occurred.
    #[error("Compression error: {0}")]
    Compression(String),

    /// The file is empty or truncated.
    #[error("File is empty or truncated")]
    Truncated,

    /// Corrupt data detected at the given offset.
    #[error("Corrupt data at offset {0}")]
    Corrupt(u64),

    /// A query execution error.
    #[error("Query error: {0}")]
    QueryError(String),

    /// A collective sync error.
    #[error("Collective sync error: {0}")]
    CollectiveError(String),

    /// Duplicate edge detected.
    #[error("Duplicate edge from {0} to {1}")]
    DuplicateEdge(u64, u64),
}

/// Convenience result type for AgenticCodebase operations.
pub type AcbResult<T> = Result<T, AcbError>;
