//! Core data types for the AgenticCodebase semantic code compiler.
//!
//! This module contains all type definitions used throughout the system.
//! No logic or I/O — pure struct definitions, enum definitions, and trait
//! implementations.

pub mod code_unit;
pub mod edge;
pub mod error;
pub mod header;
pub mod language;
pub mod span;

pub use code_unit::{CodeUnit, CodeUnitBuilder, CodeUnitType, Visibility};
pub use edge::{Edge, EdgeType};
pub use error::{AcbError, AcbResult};
pub use header::FileHeader;
pub use language::Language;
pub use span::Span;

/// Magic bytes at the start of every .acb file: "ACDB"
pub const ACB_MAGIC: [u8; 4] = [0x41, 0x43, 0x44, 0x42];

/// Current format version.
pub const FORMAT_VERSION: u32 = 1;

/// Default feature vector dimensionality.
pub const DEFAULT_DIMENSION: usize = 256;

/// Maximum symbol name length.
pub const MAX_SYMBOL_NAME: usize = 1024;

/// Maximum qualified name length.
pub const MAX_QUALIFIED_NAME: usize = 4096;

/// Maximum file path length.
pub const MAX_PATH_LENGTH: usize = 4096;

/// Maximum edges per code unit.
pub const MAX_EDGES_PER_UNIT: u32 = 16384;

/// Maximum signature length.
pub const MAX_SIGNATURE_LENGTH: usize = 2048;

/// Maximum doc summary length.
pub const MAX_DOC_LENGTH: usize = 512;

/// Returns the current time as Unix epoch microseconds.
pub fn now_micros() -> u64 {
    chrono::Utc::now().timestamp_micros() as u64
}
