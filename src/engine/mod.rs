//! High-level compilation pipeline and query engine.
//!
//! Orchestrates parsing, semantic analysis, and graph building.
//! Orchestrates query execution across indexes.

pub mod compile;
pub mod incremental;
pub mod query;

pub use compile::{CompileOptions, CompilePipeline, CompileResult, CompileStats};
pub use incremental::{ChangeSet, IncrementalCompiler, IncrementalResult};
pub use query::QueryEngine;
