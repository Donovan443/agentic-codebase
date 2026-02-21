//! Core graph structure holding code units and edges.
//!
//! The `CodeGraph` is the central data structure. It stores all code units
//! (nodes) and edges (relationships) and provides O(1) lookup by ID.

use std::collections::{HashMap, HashSet};

use crate::types::{
    AcbError, AcbResult, CodeUnit, CodeUnitType, Edge, EdgeType, Language, MAX_EDGES_PER_UNIT,
};

/// The core in-memory graph of code units and their relationships.
///
/// Units are stored by their sequential ID. Edges are indexed by source ID
/// for efficient forward traversal.
#[derive(Debug, Clone)]
pub struct CodeGraph {
    /// All code units, indexed by ID.
    units: Vec<CodeUnit>,

    /// All edges.
    edges: Vec<Edge>,

    /// Edges indexed by source unit ID.
    edges_by_source: HashMap<u64, Vec<usize>>,

    /// Edges indexed by target unit ID (reverse index).
    edges_by_target: HashMap<u64, Vec<usize>>,

    /// Feature vector dimensionality.
    dimension: usize,

    /// Set of languages present in the graph.
    languages: HashSet<Language>,
}

impl CodeGraph {
    /// Create a new empty code graph with the given feature vector dimension.
    pub fn new(dimension: usize) -> Self {
        Self {
            units: Vec::new(),
            edges: Vec::new(),
            edges_by_source: HashMap::new(),
            edges_by_target: HashMap::new(),
            dimension,
            languages: HashSet::new(),
        }
    }

    /// Create with default dimension (256).
    pub fn with_default_dimension() -> Self {
        Self::new(crate::types::DEFAULT_DIMENSION)
    }

    /// Add a code unit to the graph, assigning it a sequential ID.
    ///
    /// Returns the assigned ID.
    pub fn add_unit(&mut self, mut unit: CodeUnit) -> u64 {
        let id = self.units.len() as u64;
        unit.id = id;
        self.languages.insert(unit.language);
        self.units.push(unit);
        id
    }

    /// Add an edge between two code units.
    ///
    /// # Errors
    ///
    /// - `AcbError::SelfEdge` if source and target are the same.
    /// - `AcbError::UnitNotFound` if source or target don't exist.
    /// - `AcbError::TooManyEdges` if the source unit already has too many edges.
    pub fn add_edge(&mut self, edge: Edge) -> AcbResult<()> {
        // Validate: no self-edges
        if edge.source_id == edge.target_id {
            return Err(AcbError::SelfEdge(edge.source_id));
        }

        // Validate: source exists
        if edge.source_id >= self.units.len() as u64 {
            return Err(AcbError::UnitNotFound(edge.source_id));
        }

        // Validate: target exists
        if edge.target_id >= self.units.len() as u64 {
            return Err(AcbError::InvalidEdgeTarget(edge.target_id));
        }

        // Validate: not too many edges from source
        let source_edge_count = self
            .edges_by_source
            .get(&edge.source_id)
            .map(|v| v.len() as u32)
            .unwrap_or(0);
        if source_edge_count >= MAX_EDGES_PER_UNIT {
            return Err(AcbError::TooManyEdges(source_edge_count));
        }

        let idx = self.edges.len();
        self.edges_by_source
            .entry(edge.source_id)
            .or_default()
            .push(idx);
        self.edges_by_target
            .entry(edge.target_id)
            .or_default()
            .push(idx);
        self.edges.push(edge);

        Ok(())
    }

    /// Returns the number of code units.
    pub fn unit_count(&self) -> usize {
        self.units.len()
    }

    /// Returns the number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns the feature vector dimension.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Returns the set of languages in the graph.
    pub fn languages(&self) -> &HashSet<Language> {
        &self.languages
    }

    /// Get a code unit by ID.
    pub fn get_unit(&self, id: u64) -> Option<&CodeUnit> {
        self.units.get(id as usize)
    }

    /// Get a mutable reference to a code unit by ID.
    pub fn get_unit_mut(&mut self, id: u64) -> Option<&mut CodeUnit> {
        self.units.get_mut(id as usize)
    }

    /// Iterate over all code units.
    pub fn units(&self) -> &[CodeUnit] {
        &self.units
    }

    /// Iterate over all edges.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Get all edges originating from the given unit.
    pub fn edges_from(&self, source_id: u64) -> Vec<&Edge> {
        self.edges_by_source
            .get(&source_id)
            .map(|indices| indices.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    /// Get all edges targeting the given unit.
    pub fn edges_to(&self, target_id: u64) -> Vec<&Edge> {
        self.edges_by_target
            .get(&target_id)
            .map(|indices| indices.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    /// Get edges from a source filtered by edge type.
    pub fn edges_from_of_type(&self, source_id: u64, edge_type: EdgeType) -> Vec<&Edge> {
        self.edges_from(source_id)
            .into_iter()
            .filter(|e| e.edge_type == edge_type)
            .collect()
    }

    /// Get edges to a target filtered by edge type.
    pub fn edges_to_of_type(&self, target_id: u64, edge_type: EdgeType) -> Vec<&Edge> {
        self.edges_to(target_id)
            .into_iter()
            .filter(|e| e.edge_type == edge_type)
            .collect()
    }

    /// Find units by name (case-insensitive prefix match).
    pub fn find_units_by_name(&self, prefix: &str) -> Vec<&CodeUnit> {
        let prefix_lower = prefix.to_lowercase();
        self.units
            .iter()
            .filter(|u| u.name.to_lowercase().starts_with(&prefix_lower))
            .collect()
    }

    /// Find units by exact name.
    pub fn find_units_by_exact_name(&self, name: &str) -> Vec<&CodeUnit> {
        self.units.iter().filter(|u| u.name == name).collect()
    }

    /// Find units by type.
    pub fn find_units_by_type(&self, unit_type: CodeUnitType) -> Vec<&CodeUnit> {
        self.units
            .iter()
            .filter(|u| u.unit_type == unit_type)
            .collect()
    }

    /// Find units by language.
    pub fn find_units_by_language(&self, language: Language) -> Vec<&CodeUnit> {
        self.units
            .iter()
            .filter(|u| u.language == language)
            .collect()
    }

    /// Find units in a specific file.
    pub fn find_units_by_path(&self, path: &std::path::Path) -> Vec<&CodeUnit> {
        self.units.iter().filter(|u| u.file_path == path).collect()
    }

    /// Check if an edge between two units of a given type already exists.
    pub fn has_edge(&self, source_id: u64, target_id: u64, edge_type: EdgeType) -> bool {
        self.edges_from(source_id)
            .iter()
            .any(|e| e.target_id == target_id && e.edge_type == edge_type)
    }

    /// Get summary statistics about the graph.
    pub fn stats(&self) -> GraphStats {
        let mut type_counts: HashMap<CodeUnitType, usize> = HashMap::new();
        let mut edge_type_counts: HashMap<EdgeType, usize> = HashMap::new();
        let mut lang_counts: HashMap<Language, usize> = HashMap::new();

        for unit in &self.units {
            *type_counts.entry(unit.unit_type).or_default() += 1;
            *lang_counts.entry(unit.language).or_default() += 1;
        }
        for edge in &self.edges {
            *edge_type_counts.entry(edge.edge_type).or_default() += 1;
        }

        GraphStats {
            unit_count: self.units.len(),
            edge_count: self.edges.len(),
            dimension: self.dimension,
            type_counts,
            edge_type_counts,
            language_counts: lang_counts,
        }
    }
}

impl Default for CodeGraph {
    fn default() -> Self {
        Self::with_default_dimension()
    }
}

/// Summary statistics about a code graph.
#[derive(Debug, Clone)]
pub struct GraphStats {
    /// Total number of code units.
    pub unit_count: usize,
    /// Total number of edges.
    pub edge_count: usize,
    /// Feature vector dimension.
    pub dimension: usize,
    /// Count of units per type.
    pub type_counts: HashMap<CodeUnitType, usize>,
    /// Count of edges per type.
    pub edge_type_counts: HashMap<EdgeType, usize>,
    /// Count of units per language.
    pub language_counts: HashMap<Language, usize>,
}
