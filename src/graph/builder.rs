//! Fluent API for building code graphs.
//!
//! The `GraphBuilder` provides a convenient way to construct graphs
//! by chaining method calls.

use crate::types::{AcbResult, CodeUnit, Edge};

use super::code_graph::CodeGraph;

/// Fluent builder for constructing a [`CodeGraph`].
///
/// # Examples
///
/// ```
/// use agentic_codebase::graph::GraphBuilder;
/// use agentic_codebase::types::*;
/// use std::path::PathBuf;
///
/// let graph = GraphBuilder::new(256)
///     .add_unit(CodeUnit::new(
///         CodeUnitType::Module,
///         Language::Python,
///         "mymodule".into(),
///         "mymodule".into(),
///         PathBuf::from("mymodule.py"),
///         Span::new(1, 0, 100, 0),
///     ))
///     .add_unit(CodeUnit::new(
///         CodeUnitType::Function,
///         Language::Python,
///         "my_func".into(),
///         "mymodule.my_func".into(),
///         PathBuf::from("mymodule.py"),
///         Span::new(10, 0, 20, 0),
///     ))
///     .add_edge(Edge::new(0, 1, EdgeType::Contains))
///     .build()
///     .unwrap();
/// ```
pub struct GraphBuilder {
    graph: CodeGraph,
    pending_edges: Vec<Edge>,
    errors: Vec<String>,
}

impl GraphBuilder {
    /// Create a new builder with the given feature vector dimension.
    pub fn new(dimension: usize) -> Self {
        Self {
            graph: CodeGraph::new(dimension),
            pending_edges: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Create with default dimension (256).
    pub fn with_default_dimension() -> Self {
        Self::new(crate::types::DEFAULT_DIMENSION)
    }

    /// Add a code unit to the graph.
    pub fn add_unit(mut self, unit: CodeUnit) -> Self {
        self.graph.add_unit(unit);
        self
    }

    /// Add an edge (deferred until build).
    pub fn add_edge(mut self, edge: Edge) -> Self {
        self.pending_edges.push(edge);
        self
    }

    /// Consume the builder and produce a [`CodeGraph`].
    ///
    /// All pending edges are validated and added. Returns an error if
    /// any edge is invalid.
    pub fn build(mut self) -> AcbResult<CodeGraph> {
        for edge in self.pending_edges {
            self.graph.add_edge(edge)?;
        }
        Ok(self.graph)
    }

    /// Build, ignoring invalid edges (logs warnings).
    pub fn build_lenient(mut self) -> CodeGraph {
        for edge in self.pending_edges {
            if let Err(e) = self.graph.add_edge(edge) {
                tracing::warn!("Skipping invalid edge: {}", e);
                self.errors.push(format!("{}", e));
            }
        }
        self.graph
    }

    /// Returns any errors accumulated during lenient build.
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
}
