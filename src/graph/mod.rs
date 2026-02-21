//! In-memory graph operations for the code graph.
//!
//! This module contains the core graph data structure, builder API,
//! and traversal algorithms. No file I/O or query planning.

pub mod builder;
pub mod code_graph;
pub mod traversal;

pub use builder::GraphBuilder;
pub use code_graph::CodeGraph;
