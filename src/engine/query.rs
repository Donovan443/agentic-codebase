//! Query executor for all 24 query types.
//!
//! The [`QueryEngine`] is the single entry point for running any of the 24
//! supported queries against a [`CodeGraph`]. Each query has its own param
//! struct and result type, all defined in this module.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph::code_graph::CodeGraph;
use crate::graph::traversal::{self, Direction, TraversalOptions};
use crate::types::{
    AcbError, AcbResult, CodeUnit, CodeUnitType, EdgeType, Language, Span, Visibility,
};

// ============================================================================
// Query param / result types
// ============================================================================

/// How symbol names are matched in lookups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchMode {
    /// Exact string equality.
    Exact,
    /// Case-insensitive prefix.
    Prefix,
    /// Substring anywhere in the name.
    Contains,
    /// Fuzzy (Levenshtein distance <= threshold).
    Fuzzy,
}

/// Parameters for Query 1: Symbol Lookup.
#[derive(Debug, Clone)]
pub struct SymbolLookupParams {
    /// The search string.
    pub name: String,
    /// Matching strategy.
    pub mode: MatchMode,
    /// If non-empty, restrict to these unit types.
    pub unit_types: Vec<CodeUnitType>,
    /// If non-empty, restrict to these languages.
    pub languages: Vec<Language>,
    /// Maximum results to return (0 = unlimited).
    pub limit: usize,
    /// Fuzzy threshold (max edit distance). Only used when mode = Fuzzy.
    pub fuzzy_threshold: usize,
}

impl Default for SymbolLookupParams {
    fn default() -> Self {
        Self {
            name: String::new(),
            mode: MatchMode::Exact,
            unit_types: Vec::new(),
            languages: Vec::new(),
            limit: 0,
            fuzzy_threshold: 2,
        }
    }
}

/// Parameters for Queries 2 & 3: Dependency / Reverse-Dependency.
#[derive(Debug, Clone)]
pub struct DependencyParams {
    /// Starting code unit.
    pub unit_id: u64,
    /// Maximum traversal depth.
    pub max_depth: u32,
    /// Edge types to follow (empty = all dependency types).
    pub edge_types: Vec<EdgeType>,
    /// Whether to include transitive (multi-hop) dependencies.
    pub include_transitive: bool,
}

/// A single dependency node in a result tree.
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// The code unit id.
    pub unit_id: u64,
    /// Depth from the origin.
    pub depth: u32,
    /// The path of unit IDs from origin to this node.
    pub path: Vec<u64>,
}

/// Result of a dependency or reverse-dependency query.
#[derive(Debug, Clone)]
pub struct DependencyResult {
    /// The origin unit.
    pub root_id: u64,
    /// All dependency nodes found.
    pub nodes: Vec<DependencyNode>,
}

/// Direction of call-graph exploration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallDirection {
    /// Who calls this function.
    Callers,
    /// What does this function call.
    Callees,
    /// Both directions.
    Both,
}

/// Parameters for Query 4: Call Graph.
#[derive(Debug, Clone)]
pub struct CallGraphParams {
    /// The function unit to inspect.
    pub unit_id: u64,
    /// Which direction to explore.
    pub direction: CallDirection,
    /// Maximum depth of call chain.
    pub max_depth: u32,
}

/// A call site in the call graph.
#[derive(Debug, Clone)]
pub struct CallSite {
    /// The calling unit.
    pub caller_id: u64,
    /// The called unit.
    pub callee_id: u64,
    /// Location of the call.
    pub span: Span,
}

/// Result of a call graph query.
#[derive(Debug, Clone)]
pub struct CallGraphResult {
    /// The origin function.
    pub root_id: u64,
    /// All functions discovered.
    pub nodes: Vec<(u64, u32)>,
    /// Call sites found.
    pub call_sites: Vec<CallSite>,
}

/// Parameters for Query 5: Type Hierarchy.
#[derive(Debug, Clone)]
pub struct HierarchyParams {
    /// The type unit to inspect.
    pub unit_id: u64,
    /// Whether to include ancestors.
    pub include_ancestors: bool,
    /// Whether to include descendants.
    pub include_descendants: bool,
}

/// A node in the type hierarchy result.
#[derive(Debug, Clone)]
pub struct HierarchyNode {
    /// The unit ID.
    pub unit_id: u64,
    /// Relationship kind (inherits or implements).
    pub relation: EdgeType,
    /// Depth from the query target (positive = ancestor, negative = descendant).
    pub depth: i32,
}

/// Result of a type hierarchy query.
#[derive(Debug, Clone)]
pub struct HierarchyResult {
    /// The queried type.
    pub root_id: u64,
    /// All hierarchy nodes.
    pub nodes: Vec<HierarchyNode>,
}

/// Parameters for Query 7: Pattern Match.
#[derive(Debug, Clone)]
pub struct PatternParams {
    /// A simple pattern DSL string.
    ///
    /// Supported patterns:
    /// - `function { calls: [A, B] }` — find functions that call A and B
    /// - `class { inherits: Base }` — find classes that inherit from Base
    /// - `async function` — find async functions
    /// - `function { complexity: >N }` — find functions with complexity > N
    pub pattern: String,
}

/// A single pattern match.
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// The matched unit.
    pub unit_id: u64,
    /// Confidence of the match (0.0–1.0).
    pub confidence: f32,
    /// Which part of the pattern was matched.
    pub matched_rule: String,
}

/// Parameters for Query 8: Semantic Search.
#[derive(Debug, Clone)]
pub struct SemanticParams {
    /// Query feature vector.
    pub query_vec: Vec<f32>,
    /// Maximum number of results.
    pub top_k: usize,
    /// Optional type filter.
    pub unit_types: Vec<CodeUnitType>,
    /// Optional language filter.
    pub languages: Vec<Language>,
    /// Minimum similarity threshold (0.0–1.0).
    pub min_similarity: f32,
}

/// A single semantic search match.
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    /// The matched unit.
    pub unit_id: u64,
    /// Cosine similarity score.
    pub score: f32,
}

/// Parameters for Query 9: Impact Analysis.
#[derive(Debug, Clone)]
pub struct ImpactParams {
    /// The unit being changed.
    pub unit_id: u64,
    /// Maximum depth of impact propagation.
    pub max_depth: u32,
    /// Edge types to consider (empty = all dependency types).
    pub edge_types: Vec<EdgeType>,
}

/// A single impacted unit.
#[derive(Debug, Clone)]
pub struct ImpactedUnit {
    /// The affected unit.
    pub unit_id: u64,
    /// Distance from origin.
    pub depth: u32,
    /// Risk score (0.0 = low, 1.0 = high).
    pub risk_score: f32,
    /// Whether this unit has test coverage.
    pub has_tests: bool,
}

/// Result of an impact analysis.
#[derive(Debug, Clone)]
pub struct ImpactResult {
    /// The origin unit.
    pub root_id: u64,
    /// All impacted units.
    pub impacted: Vec<ImpactedUnit>,
    /// Overall risk score.
    pub overall_risk: f32,
    /// Recommendations.
    pub recommendations: Vec<String>,
}

/// Result of a test coverage query.
#[derive(Debug, Clone)]
pub struct CoverageResult {
    /// The unit being queried.
    pub unit_id: u64,
    /// Direct tests (test units with a Tests edge to this unit).
    pub direct_tests: Vec<u64>,
    /// Indirect tests (tests of callers).
    pub indirect_tests: Vec<u64>,
    /// Estimated coverage ratio (0.0–1.0).
    pub coverage_ratio: f32,
}

/// Parameters for Query 11: Cross-Language Trace.
#[derive(Debug, Clone)]
pub struct TraceParams {
    /// Starting unit.
    pub unit_id: u64,
    /// Maximum hops.
    pub max_hops: u32,
}

/// A single hop in a cross-language trace.
#[derive(Debug, Clone)]
pub struct TraceHop {
    /// The unit at this hop.
    pub unit_id: u64,
    /// Language of this unit.
    pub language: Language,
    /// The edge type that led here.
    pub via_edge: Option<EdgeType>,
}

/// Result of a cross-language trace.
#[derive(Debug, Clone)]
pub struct TraceResult {
    /// All hops from origin.
    pub hops: Vec<TraceHop>,
    /// Languages traversed (in order).
    pub languages_crossed: Vec<Language>,
}

/// Parameters for Query 12: Collective Patterns.
#[derive(Debug, Clone)]
pub struct CollectiveParams {
    /// Optional type filter.
    pub unit_type: Option<CodeUnitType>,
    /// Minimum collective usage count.
    pub min_usage: u64,
    /// Maximum results.
    pub limit: usize,
}

/// A single collective pattern match.
#[derive(Debug, Clone)]
pub struct CollectivePatternEntry {
    /// The unit ID.
    pub unit_id: u64,
    /// Collective usage count.
    pub usage_count: u64,
    /// Confidence/relevance.
    pub confidence: f32,
}

/// Result of a collective patterns query.
#[derive(Debug, Clone)]
pub struct CollectiveResult {
    /// Pattern entries found.
    pub patterns: Vec<CollectivePatternEntry>,
    /// Whether collective data is available.
    pub collective_available: bool,
}

/// Result of a temporal evolution query.
#[derive(Debug, Clone)]
pub struct EvolutionResult {
    /// The queried unit.
    pub unit_id: u64,
    /// Number of changes.
    pub change_count: u32,
    /// Created-at timestamp.
    pub created_at: u64,
    /// Last modified timestamp.
    pub last_modified: u64,
    /// Stability score.
    pub stability_score: f32,
    /// Trend description.
    pub trend: String,
}

/// Stability factor for a code unit.
#[derive(Debug, Clone)]
pub struct StabilityFactor {
    /// Factor name.
    pub name: String,
    /// Value (0.0–1.0, higher = better).
    pub value: f32,
    /// Explanation.
    pub description: String,
}

/// Result of a stability analysis.
#[derive(Debug, Clone)]
pub struct StabilityResult {
    /// The queried unit.
    pub unit_id: u64,
    /// Overall stability score (0.0–1.0).
    pub overall_score: f32,
    /// Individual factors.
    pub factors: Vec<StabilityFactor>,
    /// Recommendation.
    pub recommendation: String,
}

/// Parameters for Query 15: Coupling Detection.
#[derive(Debug, Clone)]
pub struct CouplingParams {
    /// Optional unit to centre the search on. If None, scan all.
    pub unit_id: Option<u64>,
    /// Minimum coupling strength threshold.
    pub min_strength: f32,
}

/// A detected coupling between two units.
#[derive(Debug, Clone)]
pub struct Coupling {
    /// First unit.
    pub unit_a: u64,
    /// Second unit.
    pub unit_b: u64,
    /// Coupling strength (0.0–1.0).
    pub strength: f32,
    /// Kind of coupling.
    pub kind: CouplingKind,
}

/// The kind of detected coupling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CouplingKind {
    /// Direct edge between units.
    Explicit,
    /// CouplesWith temporal edge.
    Temporal,
    /// Share many connections.
    Hidden,
}

/// Parameters for Query 16: Dead Code.
#[derive(Debug, Clone)]
pub struct DeadCodeParams {
    /// Consider only these unit types as potential dead code (empty = all).
    pub unit_types: Vec<CodeUnitType>,
    /// Whether to include test files in reachability roots.
    pub include_tests_as_roots: bool,
}

/// Parameters for Query 17: Prophecy.
#[derive(Debug, Clone)]
pub struct ProphecyParams {
    /// Number of predictions to return.
    pub top_k: usize,
    /// Minimum risk threshold.
    pub min_risk: f32,
}

/// A single prophecy prediction.
#[derive(Debug, Clone)]
pub struct Prediction {
    /// The unit predicted to be at risk.
    pub unit_id: u64,
    /// Risk score (0.0–1.0).
    pub risk_score: f32,
    /// Reason for the prediction.
    pub reason: String,
}

/// Result of a prophecy query.
#[derive(Debug, Clone)]
pub struct ProphecyResult {
    /// Predictions sorted by risk.
    pub predictions: Vec<Prediction>,
}

/// A concept-mapped unit and its role.
#[derive(Debug, Clone)]
pub struct ConceptUnit {
    /// The unit.
    pub unit_id: u64,
    /// Role of this unit in the concept.
    pub role: ConceptRole,
    /// Relevance score.
    pub relevance: f32,
}

/// The role a unit plays in a concept mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConceptRole {
    /// Defines the concept.
    Definition,
    /// Uses the concept.
    Usage,
    /// Extends the concept.
    Extension,
    /// Tests the concept.
    Test,
}

/// Result of a concept mapping query.
#[derive(Debug, Clone)]
pub struct ConceptMap {
    /// The queried concept.
    pub concept: String,
    /// Units related to this concept.
    pub units: Vec<ConceptUnit>,
}

/// Parameters for Query 19: Migration Path.
#[derive(Debug, Clone)]
pub struct MigrationParams {
    /// The unit to migrate from.
    pub from_unit: u64,
    /// The unit to migrate to.
    pub to_unit: u64,
}

/// Safety level of a migration step.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SafetyLevel {
    /// Safe — has tests.
    Safe,
    /// Caution — no direct tests but has callers with tests.
    Caution,
    /// Risky — no test coverage at all.
    Risky,
}

/// A single step in a migration plan.
#[derive(Debug, Clone)]
pub struct MigrationStep {
    /// The unit that needs updating.
    pub unit_id: u64,
    /// Order in which to do the migration.
    pub order: u32,
    /// Safety level.
    pub safety: SafetyLevel,
    /// Description of what to do.
    pub description: String,
}

/// Result of a migration path query.
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    /// Source unit.
    pub from_unit: u64,
    /// Target unit.
    pub to_unit: u64,
    /// Ordered steps.
    pub steps: Vec<MigrationStep>,
}

/// Parameters for Query 20: Test Gap.
#[derive(Debug, Clone)]
pub struct TestGapParams {
    /// Minimum change count to consider.
    pub min_changes: u32,
    /// Minimum complexity to consider.
    pub min_complexity: u32,
    /// Unit types to scan (empty = Function only).
    pub unit_types: Vec<CodeUnitType>,
}

/// A detected test gap.
#[derive(Debug, Clone)]
pub struct TestGap {
    /// The unit missing tests.
    pub unit_id: u64,
    /// Why this is flagged.
    pub reason: String,
    /// Priority score (higher = more urgent).
    pub priority: f32,
}

/// Parameters for Query 21: Architectural Drift.
#[derive(Debug, Clone)]
pub struct DriftParams {
    /// Architectural rules to check.
    pub rules: Vec<ArchRule>,
}

/// A single architectural rule.
#[derive(Debug, Clone)]
pub enum ArchRule {
    /// Layer A must not depend on Layer B.
    LayerDependency {
        /// Upper layer module prefix.
        upper: String,
        /// Lower layer module prefix.
        lower: String,
    },
    /// Module must not have external edges to another module.
    ModuleBoundary {
        /// Module prefix.
        module: String,
    },
    /// All units matching a prefix must follow a naming convention.
    NamingConvention {
        /// Module/path prefix.
        prefix: String,
        /// Regex pattern that names must match.
        pattern: String,
    },
    /// No dependency cycles in the given scope.
    Cyclic {
        /// Module prefix scope.
        scope: String,
    },
}

/// A single drift violation.
#[derive(Debug, Clone)]
pub struct DriftViolation {
    /// Which rule was violated.
    pub rule_index: usize,
    /// Description of the violation.
    pub description: String,
    /// Units involved.
    pub units: Vec<u64>,
}

/// Result of an architectural drift check.
#[derive(Debug, Clone)]
pub struct DriftReport {
    /// All violations found.
    pub violations: Vec<DriftViolation>,
    /// Overall conformance score (1.0 = no drift).
    pub conformance_score: f32,
}

/// Parameters for Query 22: Similarity.
#[derive(Debug, Clone)]
pub struct SimilarityParams {
    /// The unit to compare against.
    pub unit_id: u64,
    /// Number of results.
    pub top_k: usize,
    /// Minimum similarity.
    pub min_similarity: f32,
}

/// A single similarity match.
#[derive(Debug, Clone)]
pub struct SimilarityMatch {
    /// The similar unit.
    pub unit_id: u64,
    /// Cosine similarity score.
    pub score: f32,
}

/// Result of a shortest-path query.
#[derive(Debug, Clone)]
pub struct PathResult {
    /// Whether a path was found.
    pub found: bool,
    /// The path of unit IDs from source to destination.
    pub path: Vec<u64>,
    /// Edge types along the path.
    pub edge_types: Vec<EdgeType>,
    /// Total path length.
    pub length: usize,
}

/// Parameters for Query 24: Hotspot Detection.
#[derive(Debug, Clone)]
pub struct HotspotParams {
    /// Maximum results.
    pub top_k: usize,
    /// Minimum hotspot score threshold.
    pub min_score: f32,
    /// Unit types to consider (empty = all).
    pub unit_types: Vec<CodeUnitType>,
}

/// A detected hotspot.
#[derive(Debug, Clone)]
pub struct Hotspot {
    /// The unit.
    pub unit_id: u64,
    /// Hotspot score (higher = more concerning).
    pub score: f32,
    /// Contributing factors.
    pub factors: HashMap<String, f32>,
}

// ============================================================================
// Utility: Levenshtein distance
// ============================================================================

/// Compute the Levenshtein (edit) distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    // Only need two rows.
    let mut prev = vec![0usize; b_len + 1];
    let mut curr = vec![0usize; b_len + 1];

    for (j, item) in prev.iter_mut().enumerate().take(b_len + 1) {
        *item = j;
    }

    for i in 1..=a_len {
        curr[0] = i;
        for j in 1..=b_len {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}

/// Compute cosine similarity between two f32 slices.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;

    for (x, y) in a.iter().zip(b.iter()) {
        let xf = *x as f64;
        let yf = *y as f64;
        dot += xf * yf;
        norm_a += xf * xf;
        norm_b += yf * yf;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-12 {
        return 0.0;
    }

    (dot / denom) as f32
}

// ============================================================================
// QueryEngine
// ============================================================================

/// The central query executor.
///
/// Implements all 24 query types defined in the AgenticCodebase specification.
/// Each method takes a shared reference to a [`CodeGraph`] and query-specific
/// parameters, returning a strongly-typed result or an [`AcbError`].
#[derive(Debug, Clone)]
pub struct QueryEngine;

impl QueryEngine {
    /// Create a new query engine.
    pub fn new() -> Self {
        Self
    }

    // ========================================================================
    // Query 1: Symbol Lookup
    // ========================================================================

    /// Look up symbols by name with optional filters.
    pub fn symbol_lookup<'g>(
        &self,
        graph: &'g CodeGraph,
        params: SymbolLookupParams,
    ) -> AcbResult<Vec<&'g CodeUnit>> {
        let candidates: Vec<&CodeUnit> = match params.mode {
            MatchMode::Exact => graph.find_units_by_exact_name(&params.name),
            MatchMode::Prefix => graph.find_units_by_name(&params.name),
            MatchMode::Contains => {
                let lower = params.name.to_lowercase();
                graph
                    .units()
                    .iter()
                    .filter(|u| u.name.to_lowercase().contains(&lower))
                    .collect()
            }
            MatchMode::Fuzzy => {
                let lower = params.name.to_lowercase();
                let threshold = params.fuzzy_threshold;
                graph
                    .units()
                    .iter()
                    .filter(|u| levenshtein(&u.name.to_lowercase(), &lower) <= threshold)
                    .collect()
            }
        };

        // Apply type filter.
        let filtered: Vec<&CodeUnit> = candidates
            .into_iter()
            .filter(|u| params.unit_types.is_empty() || params.unit_types.contains(&u.unit_type))
            .filter(|u| params.languages.is_empty() || params.languages.contains(&u.language))
            .collect();

        // Apply limit.
        let result = if params.limit > 0 {
            filtered.into_iter().take(params.limit).collect()
        } else {
            filtered
        };

        Ok(result)
    }

    // ========================================================================
    // Query 2: Dependency Graph
    // ========================================================================

    /// Build the forward dependency graph from a unit.
    pub fn dependency_graph(
        &self,
        graph: &CodeGraph,
        params: DependencyParams,
    ) -> AcbResult<DependencyResult> {
        self.validate_unit(graph, params.unit_id)?;

        let effective_depth = if params.include_transitive {
            params.max_depth
        } else {
            1
        };

        let edge_types = if params.edge_types.is_empty() {
            vec![
                EdgeType::Calls,
                EdgeType::Imports,
                EdgeType::Inherits,
                EdgeType::Implements,
                EdgeType::UsesType,
                EdgeType::FfiBinds,
            ]
        } else {
            params.edge_types
        };

        // BFS with path tracking.
        let mut nodes = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(params.unit_id);
        queue.push_back((params.unit_id, 0u32, vec![params.unit_id]));

        while let Some((current, depth, path)) = queue.pop_front() {
            if current != params.unit_id {
                nodes.push(DependencyNode {
                    unit_id: current,
                    depth,
                    path: path.clone(),
                });
            }

            if depth >= effective_depth {
                continue;
            }

            for edge in graph.edges_from(current) {
                if !edge_types.contains(&edge.edge_type) {
                    continue;
                }
                if visited.insert(edge.target_id) {
                    let mut new_path = path.clone();
                    new_path.push(edge.target_id);
                    queue.push_back((edge.target_id, depth + 1, new_path));
                }
            }
        }

        Ok(DependencyResult {
            root_id: params.unit_id,
            nodes,
        })
    }

    // ========================================================================
    // Query 3: Reverse Dependency
    // ========================================================================

    /// Build the reverse dependency graph (who depends on this unit).
    pub fn reverse_dependency(
        &self,
        graph: &CodeGraph,
        params: DependencyParams,
    ) -> AcbResult<DependencyResult> {
        self.validate_unit(graph, params.unit_id)?;

        let effective_depth = if params.include_transitive {
            params.max_depth
        } else {
            1
        };

        let edge_types = if params.edge_types.is_empty() {
            vec![
                EdgeType::Calls,
                EdgeType::Imports,
                EdgeType::Inherits,
                EdgeType::Implements,
                EdgeType::UsesType,
                EdgeType::FfiBinds,
            ]
        } else {
            params.edge_types
        };

        let mut nodes = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(params.unit_id);
        queue.push_back((params.unit_id, 0u32, vec![params.unit_id]));

        while let Some((current, depth, path)) = queue.pop_front() {
            if current != params.unit_id {
                nodes.push(DependencyNode {
                    unit_id: current,
                    depth,
                    path: path.clone(),
                });
            }

            if depth >= effective_depth {
                continue;
            }

            // Follow INCOMING edges (reverse direction).
            for edge in graph.edges_to(current) {
                if !edge_types.contains(&edge.edge_type) {
                    continue;
                }
                if visited.insert(edge.source_id) {
                    let mut new_path = path.clone();
                    new_path.push(edge.source_id);
                    queue.push_back((edge.source_id, depth + 1, new_path));
                }
            }
        }

        Ok(DependencyResult {
            root_id: params.unit_id,
            nodes,
        })
    }

    // ========================================================================
    // Query 4: Call Graph
    // ========================================================================

    /// Build a call graph centered on a function.
    pub fn call_graph(
        &self,
        graph: &CodeGraph,
        params: CallGraphParams,
    ) -> AcbResult<CallGraphResult> {
        self.validate_unit(graph, params.unit_id)?;

        let mut all_nodes: Vec<(u64, u32)> = Vec::new();
        let mut call_sites = Vec::new();
        let mut seen = HashSet::new();

        // Callees: outgoing Calls edges.
        if params.direction == CallDirection::Callees || params.direction == CallDirection::Both {
            let opts = TraversalOptions {
                max_depth: params.max_depth as i32,
                edge_types: vec![EdgeType::Calls],
                direction: Direction::Forward,
            };
            let results = traversal::bfs(graph, params.unit_id, &opts);
            for (id, depth) in &results {
                if seen.insert(*id) {
                    all_nodes.push((*id, *depth));
                }
            }

            // Collect call sites from forward edges.
            self.collect_call_sites_forward(
                graph,
                params.unit_id,
                params.max_depth,
                &mut call_sites,
            );
        }

        // Callers: incoming Calls edges.
        if params.direction == CallDirection::Callers || params.direction == CallDirection::Both {
            let opts = TraversalOptions {
                max_depth: params.max_depth as i32,
                edge_types: vec![EdgeType::Calls],
                direction: Direction::Backward,
            };
            let results = traversal::bfs(graph, params.unit_id, &opts);
            for (id, depth) in &results {
                if seen.insert(*id) {
                    all_nodes.push((*id, *depth));
                }
            }

            // Collect call sites from backward edges.
            self.collect_call_sites_backward(
                graph,
                params.unit_id,
                params.max_depth,
                &mut call_sites,
            );
        }

        Ok(CallGraphResult {
            root_id: params.unit_id,
            nodes: all_nodes,
            call_sites,
        })
    }

    // ========================================================================
    // Query 5: Type Hierarchy
    // ========================================================================

    /// Explore the type hierarchy (ancestors and/or descendants).
    pub fn type_hierarchy(
        &self,
        graph: &CodeGraph,
        params: HierarchyParams,
    ) -> AcbResult<HierarchyResult> {
        self.validate_unit(graph, params.unit_id)?;

        let mut nodes = Vec::new();

        if params.include_ancestors {
            // Follow outgoing Inherits/Implements edges upward.
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            visited.insert(params.unit_id);
            queue.push_back((params.unit_id, 0i32));

            while let Some((current, depth)) = queue.pop_front() {
                for edge in graph.edges_from(current) {
                    if edge.edge_type != EdgeType::Inherits
                        && edge.edge_type != EdgeType::Implements
                    {
                        continue;
                    }
                    if visited.insert(edge.target_id) {
                        let ancestor_depth = depth + 1;
                        nodes.push(HierarchyNode {
                            unit_id: edge.target_id,
                            relation: edge.edge_type,
                            depth: ancestor_depth,
                        });
                        queue.push_back((edge.target_id, ancestor_depth));
                    }
                }
            }
        }

        if params.include_descendants {
            // Follow incoming Inherits/Implements edges downward.
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            visited.insert(params.unit_id);
            queue.push_back((params.unit_id, 0i32));

            while let Some((current, depth)) = queue.pop_front() {
                for edge in graph.edges_to(current) {
                    if edge.edge_type != EdgeType::Inherits
                        && edge.edge_type != EdgeType::Implements
                    {
                        continue;
                    }
                    if visited.insert(edge.source_id) {
                        let desc_depth = depth - 1;
                        nodes.push(HierarchyNode {
                            unit_id: edge.source_id,
                            relation: edge.edge_type,
                            depth: desc_depth,
                        });
                        queue.push_back((edge.source_id, desc_depth));
                    }
                }
            }
        }

        Ok(HierarchyResult {
            root_id: params.unit_id,
            nodes,
        })
    }

    // ========================================================================
    // Query 6: Containment
    // ========================================================================

    /// Find all units contained within the given unit, recursively.
    pub fn containment<'g>(
        &self,
        graph: &'g CodeGraph,
        unit_id: u64,
    ) -> AcbResult<Vec<&'g CodeUnit>> {
        self.validate_unit(graph, unit_id)?;

        let opts = TraversalOptions {
            max_depth: -1,
            edge_types: vec![EdgeType::Contains],
            direction: Direction::Forward,
        };

        let traversal = traversal::bfs(graph, unit_id, &opts);
        let mut result = Vec::new();
        for (id, _depth) in traversal {
            if id == unit_id {
                continue; // skip the root
            }
            if let Some(unit) = graph.get_unit(id) {
                result.push(unit);
            }
        }

        Ok(result)
    }

    // ========================================================================
    // Query 7: Pattern Match
    // ========================================================================

    /// Match units against a simple pattern DSL.
    pub fn pattern_match(
        &self,
        graph: &CodeGraph,
        params: PatternParams,
    ) -> AcbResult<Vec<PatternMatch>> {
        let pattern = params.pattern.trim();
        let mut results = Vec::new();

        // Parse `async function` patterns.
        if pattern.starts_with("async function") || pattern.starts_with("async_function") {
            // Match complexity constraint if present.
            let complexity_threshold = self.parse_complexity_constraint(pattern);

            for unit in graph.units() {
                if unit.unit_type == CodeUnitType::Function && unit.is_async {
                    if let Some(threshold) = complexity_threshold {
                        if unit.complexity <= threshold {
                            continue;
                        }
                    }
                    results.push(PatternMatch {
                        unit_id: unit.id,
                        confidence: 1.0,
                        matched_rule: "async function".to_string(),
                    });
                }
            }
            return Ok(results);
        }

        // Parse `function { calls: [A, B] }`.
        if pattern.starts_with("function") && pattern.contains("calls:") {
            let call_targets = self.parse_call_list(pattern);
            let complexity_threshold = self.parse_complexity_constraint(pattern);

            for unit in graph.units() {
                if unit.unit_type != CodeUnitType::Function {
                    continue;
                }
                if let Some(threshold) = complexity_threshold {
                    if unit.complexity <= threshold {
                        continue;
                    }
                }
                let callees: HashSet<String> = graph
                    .edges_from_of_type(unit.id, EdgeType::Calls)
                    .iter()
                    .filter_map(|e| graph.get_unit(e.target_id))
                    .map(|u| u.name.clone())
                    .collect();

                if call_targets.iter().all(|t| callees.contains(t)) {
                    results.push(PatternMatch {
                        unit_id: unit.id,
                        confidence: 1.0,
                        matched_rule: format!("function calls {:?}", call_targets),
                    });
                }
            }
            return Ok(results);
        }

        // Parse `class { inherits: Base }`.
        if pattern.starts_with("class") && pattern.contains("inherits:") {
            let base_name = self.parse_inherits_target(pattern);
            if let Some(base) = base_name {
                for unit in graph.units() {
                    if unit.unit_type != CodeUnitType::Type {
                        continue;
                    }
                    let parents: Vec<String> = graph
                        .edges_from_of_type(unit.id, EdgeType::Inherits)
                        .iter()
                        .filter_map(|e| graph.get_unit(e.target_id))
                        .map(|u| u.name.clone())
                        .collect();
                    if parents.contains(&base) {
                        results.push(PatternMatch {
                            unit_id: unit.id,
                            confidence: 1.0,
                            matched_rule: format!("class inherits {}", base),
                        });
                    }
                }
            }
            return Ok(results);
        }

        // Parse `function { complexity: >N }`.
        if pattern.starts_with("function") && pattern.contains("complexity:") {
            let threshold = self.parse_complexity_constraint(pattern);
            if let Some(t) = threshold {
                for unit in graph.units() {
                    if unit.unit_type == CodeUnitType::Function && unit.complexity > t {
                        results.push(PatternMatch {
                            unit_id: unit.id,
                            confidence: 1.0,
                            matched_rule: format!("function complexity > {}", t),
                        });
                    }
                }
            }
            return Ok(results);
        }

        // Fallback: no pattern matched.
        Err(AcbError::QueryError(format!(
            "Unrecognized pattern: {}",
            pattern
        )))
    }

    // ========================================================================
    // Query 8: Semantic Search
    // ========================================================================

    /// Brute-force cosine-similarity search over feature vectors.
    pub fn semantic_search(
        &self,
        graph: &CodeGraph,
        params: SemanticParams,
    ) -> AcbResult<Vec<SemanticMatch>> {
        if params.query_vec.is_empty() {
            return Err(AcbError::QueryError("Query vector is empty".to_string()));
        }

        let mut scored: Vec<SemanticMatch> = graph
            .units()
            .iter()
            .filter(|u| params.unit_types.is_empty() || params.unit_types.contains(&u.unit_type))
            .filter(|u| params.languages.is_empty() || params.languages.contains(&u.language))
            .filter_map(|u| {
                let score = cosine_similarity(&params.query_vec, &u.feature_vec);
                if score >= params.min_similarity {
                    Some(SemanticMatch {
                        unit_id: u.id,
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if params.top_k > 0 {
            scored.truncate(params.top_k);
        }

        Ok(scored)
    }

    // ========================================================================
    // Query 9: Impact Analysis
    // ========================================================================

    /// Analyse the impact of changing a unit.
    pub fn impact_analysis(
        &self,
        graph: &CodeGraph,
        params: ImpactParams,
    ) -> AcbResult<ImpactResult> {
        self.validate_unit(graph, params.unit_id)?;

        // Use reverse_dependency to find who depends on this unit.
        let dep_params = DependencyParams {
            unit_id: params.unit_id,
            max_depth: params.max_depth,
            edge_types: params.edge_types.clone(),
            include_transitive: true,
        };
        let deps = self.reverse_dependency(graph, dep_params)?;

        let mut impacted = Vec::new();
        let mut total_risk = 0.0_f32;

        for node in &deps.nodes {
            let has_tests = !graph
                .edges_to_of_type(node.unit_id, EdgeType::Tests)
                .is_empty();

            // Coupling count.
            let coupling_count = graph
                .edges_from_of_type(node.unit_id, EdgeType::CouplesWith)
                .len()
                + graph
                    .edges_to_of_type(node.unit_id, EdgeType::CouplesWith)
                    .len();

            // Risk score: base from depth, increased if no tests or high coupling.
            let depth_factor = 1.0 / (1.0 + node.depth as f32);
            let test_factor = if has_tests { 0.2 } else { 0.6 };
            let coupling_factor = (coupling_count as f32 * 0.05).min(0.3);
            let risk = (depth_factor * 0.4 + test_factor + coupling_factor).min(1.0);

            total_risk += risk;

            impacted.push(ImpactedUnit {
                unit_id: node.unit_id,
                depth: node.depth,
                risk_score: risk,
                has_tests,
            });
        }

        let overall_risk = if impacted.is_empty() {
            0.0
        } else {
            (total_risk / impacted.len() as f32).min(1.0)
        };

        let mut recommendations = Vec::new();
        let untested_count = impacted.iter().filter(|i| !i.has_tests).count();
        if untested_count > 0 {
            recommendations.push(format!(
                "Add tests for {} impacted units that lack test coverage.",
                untested_count
            ));
        }
        if impacted.len() > 10 {
            recommendations.push(
                "Consider breaking this unit into smaller pieces to reduce blast radius."
                    .to_string(),
            );
        }
        if overall_risk > 0.7 {
            recommendations
                .push("High overall risk. Deploy incrementally with feature flags.".to_string());
        }

        Ok(ImpactResult {
            root_id: params.unit_id,
            impacted,
            overall_risk,
            recommendations,
        })
    }

    // ========================================================================
    // Query 10: Test Coverage
    // ========================================================================

    /// Compute test coverage information for a unit.
    pub fn test_coverage(&self, graph: &CodeGraph, unit_id: u64) -> AcbResult<CoverageResult> {
        self.validate_unit(graph, unit_id)?;

        // Direct tests: units that have a Tests edge pointing TO this unit.
        let direct_tests: Vec<u64> = graph
            .edges_to_of_type(unit_id, EdgeType::Tests)
            .iter()
            .map(|e| e.source_id)
            .collect();

        // Indirect tests: find callers of this unit, then find tests of those callers.
        let callers: Vec<u64> = graph
            .edges_to_of_type(unit_id, EdgeType::Calls)
            .iter()
            .map(|e| e.source_id)
            .collect();

        let mut indirect_set: HashSet<u64> = HashSet::new();
        for caller_id in &callers {
            for test_edge in graph.edges_to_of_type(*caller_id, EdgeType::Tests) {
                indirect_set.insert(test_edge.source_id);
            }
        }
        // Remove any that are already direct tests.
        for dt in &direct_tests {
            indirect_set.remove(dt);
        }
        let indirect_tests: Vec<u64> = indirect_set.into_iter().collect();

        // Estimate coverage ratio.
        let total_tests = direct_tests.len() + indirect_tests.len();
        let coverage_ratio = if total_tests > 0 {
            // Heuristic: direct tests count fully, indirect count as half.
            let effective = direct_tests.len() as f32 + indirect_tests.len() as f32 * 0.5;
            (effective / (effective + 1.0)).min(1.0)
        } else {
            0.0
        };

        Ok(CoverageResult {
            unit_id,
            direct_tests,
            indirect_tests,
            coverage_ratio,
        })
    }

    // ========================================================================
    // Query 11: Cross-Language Trace
    // ========================================================================

    /// Trace FFI bindings across language boundaries.
    pub fn cross_language_trace(
        &self,
        graph: &CodeGraph,
        params: TraceParams,
    ) -> AcbResult<TraceResult> {
        self.validate_unit(graph, params.unit_id)?;

        let start_unit = graph
            .get_unit(params.unit_id)
            .ok_or(AcbError::UnitNotFound(params.unit_id))?;

        let mut hops = Vec::new();
        let mut languages_crossed = Vec::new();

        hops.push(TraceHop {
            unit_id: params.unit_id,
            language: start_unit.language,
            via_edge: None,
        });
        languages_crossed.push(start_unit.language);

        let mut visited = HashSet::new();
        visited.insert(params.unit_id);
        let mut current_frontier = vec![params.unit_id];

        for _hop in 0..params.max_hops {
            let mut next_frontier = Vec::new();
            for current_id in &current_frontier {
                // Follow FfiBinds edges in both directions.
                for edge in graph.edges_from(*current_id) {
                    if edge.edge_type != EdgeType::FfiBinds {
                        continue;
                    }
                    if visited.insert(edge.target_id) {
                        if let Some(target_unit) = graph.get_unit(edge.target_id) {
                            hops.push(TraceHop {
                                unit_id: edge.target_id,
                                language: target_unit.language,
                                via_edge: Some(EdgeType::FfiBinds),
                            });
                            if !languages_crossed.contains(&target_unit.language) {
                                languages_crossed.push(target_unit.language);
                            }
                            next_frontier.push(edge.target_id);
                        }
                    }
                }
                for edge in graph.edges_to(*current_id) {
                    if edge.edge_type != EdgeType::FfiBinds {
                        continue;
                    }
                    if visited.insert(edge.source_id) {
                        if let Some(source_unit) = graph.get_unit(edge.source_id) {
                            hops.push(TraceHop {
                                unit_id: edge.source_id,
                                language: source_unit.language,
                                via_edge: Some(EdgeType::FfiBinds),
                            });
                            if !languages_crossed.contains(&source_unit.language) {
                                languages_crossed.push(source_unit.language);
                            }
                            next_frontier.push(edge.source_id);
                        }
                    }
                }
            }
            if next_frontier.is_empty() {
                break;
            }
            current_frontier = next_frontier;
        }

        Ok(TraceResult {
            hops,
            languages_crossed,
        })
    }

    // ========================================================================
    // Query 12: Collective Patterns (placeholder)
    // ========================================================================

    /// Query collective intelligence patterns.
    ///
    /// Currently returns a placeholder result since the collective
    /// intelligence backend is not yet integrated.
    pub fn collective_patterns(
        &self,
        graph: &CodeGraph,
        params: CollectiveParams,
    ) -> AcbResult<CollectiveResult> {
        // Filter units by collective usage.
        let mut patterns: Vec<CollectivePatternEntry> = graph
            .units()
            .iter()
            .filter(|u| {
                if let Some(ref ut) = params.unit_type {
                    u.unit_type == *ut
                } else {
                    true
                }
            })
            .filter(|u| u.collective_usage >= params.min_usage)
            .map(|u| CollectivePatternEntry {
                unit_id: u.id,
                usage_count: u.collective_usage,
                confidence: if u.collective_usage > 0 {
                    (u.collective_usage as f32).ln().min(1.0)
                } else {
                    0.0
                },
            })
            .collect();

        patterns.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

        if params.limit > 0 {
            patterns.truncate(params.limit);
        }

        Ok(CollectiveResult {
            patterns,
            collective_available: false,
        })
    }

    // ========================================================================
    // Query 13: Temporal Evolution
    // ========================================================================

    /// Get the temporal evolution of a code unit.
    pub fn temporal_evolution(
        &self,
        graph: &CodeGraph,
        unit_id: u64,
    ) -> AcbResult<EvolutionResult> {
        let unit = graph
            .get_unit(unit_id)
            .ok_or(AcbError::UnitNotFound(unit_id))?;

        let trend = if unit.stability_score > 0.8 {
            "Stable — rarely changes.".to_string()
        } else if unit.stability_score > 0.5 {
            "Moderate — changes occasionally.".to_string()
        } else if unit.stability_score > 0.2 {
            "Volatile — changes frequently.".to_string()
        } else {
            "Highly volatile — constantly changing, potential hotspot.".to_string()
        };

        Ok(EvolutionResult {
            unit_id,
            change_count: unit.change_count,
            created_at: unit.created_at,
            last_modified: unit.last_modified,
            stability_score: unit.stability_score,
            trend,
        })
    }

    // ========================================================================
    // Query 14: Stability Analysis
    // ========================================================================

    /// Analyse the stability of a code unit.
    pub fn stability_analysis(
        &self,
        graph: &CodeGraph,
        unit_id: u64,
    ) -> AcbResult<StabilityResult> {
        let unit = graph
            .get_unit(unit_id)
            .ok_or(AcbError::UnitNotFound(unit_id))?;

        let mut factors = Vec::new();

        // Factor 1: Change frequency.
        let change_factor = 1.0 / (1.0 + unit.change_count as f32 * 0.1);
        factors.push(StabilityFactor {
            name: "change_frequency".to_string(),
            value: change_factor,
            description: format!(
                "Unit has {} changes. Lower change count = more stable.",
                unit.change_count
            ),
        });

        // Factor 2: Test coverage.
        let test_count = graph.edges_to_of_type(unit_id, EdgeType::Tests).len();
        let test_factor = if test_count > 0 {
            (test_count as f32 * 0.3).min(1.0)
        } else {
            0.0
        };
        factors.push(StabilityFactor {
            name: "test_coverage".to_string(),
            value: test_factor,
            description: format!(
                "{} direct tests. More tests = more stability confidence.",
                test_count
            ),
        });

        // Factor 3: Complexity.
        let complexity_factor = 1.0 / (1.0 + unit.complexity as f32 * 0.05);
        factors.push(StabilityFactor {
            name: "complexity".to_string(),
            value: complexity_factor,
            description: format!(
                "Cyclomatic complexity is {}. Lower = more stable.",
                unit.complexity
            ),
        });

        // Factor 4: Coupling.
        let coupling_count = graph
            .edges_from_of_type(unit_id, EdgeType::CouplesWith)
            .len()
            + graph.edges_to_of_type(unit_id, EdgeType::CouplesWith).len();
        let coupling_factor = 1.0 / (1.0 + coupling_count as f32 * 0.2);
        factors.push(StabilityFactor {
            name: "coupling".to_string(),
            value: coupling_factor,
            description: format!(
                "{} temporal couplings. Fewer couplings = more independent.",
                coupling_count
            ),
        });

        // Overall score: weighted average.
        let overall = change_factor * 0.3
            + test_factor * 0.25
            + complexity_factor * 0.25
            + coupling_factor * 0.2;

        let recommendation = if overall > 0.7 {
            "Unit is stable. No immediate action needed.".to_string()
        } else if overall > 0.4 {
            "Unit has moderate stability. Consider adding tests and reducing complexity."
                .to_string()
        } else {
            "Unit is unstable. Prioritize adding tests, reducing complexity, and decoupling."
                .to_string()
        };

        Ok(StabilityResult {
            unit_id,
            overall_score: overall,
            factors,
            recommendation,
        })
    }

    // ========================================================================
    // Query 15: Coupling Detection
    // ========================================================================

    /// Detect couplings between code units.
    pub fn coupling_detection(
        &self,
        graph: &CodeGraph,
        params: CouplingParams,
    ) -> AcbResult<Vec<Coupling>> {
        let mut couplings = Vec::new();
        let mut seen_pairs: HashSet<(u64, u64)> = HashSet::new();

        let units_to_check: Vec<u64> = if let Some(uid) = params.unit_id {
            self.validate_unit(graph, uid)?;
            vec![uid]
        } else {
            graph.units().iter().map(|u| u.id).collect()
        };

        for uid in &units_to_check {
            // Explicit couplings: direct dependency edges.
            for edge in graph.edges_from(*uid) {
                if !edge.edge_type.is_dependency() {
                    continue;
                }
                let pair = normalize_pair(*uid, edge.target_id);
                if edge.weight >= params.min_strength && seen_pairs.insert(pair) {
                    couplings.push(Coupling {
                        unit_a: pair.0,
                        unit_b: pair.1,
                        strength: edge.weight,
                        kind: CouplingKind::Explicit,
                    });
                }
            }

            // Temporal couplings: CouplesWith edges.
            for edge in graph.edges_from_of_type(*uid, EdgeType::CouplesWith) {
                let pair = normalize_pair(*uid, edge.target_id);
                if edge.weight >= params.min_strength && seen_pairs.insert(pair) {
                    couplings.push(Coupling {
                        unit_a: pair.0,
                        unit_b: pair.1,
                        strength: edge.weight,
                        kind: CouplingKind::Temporal,
                    });
                }
            }
            for edge in graph.edges_to_of_type(*uid, EdgeType::CouplesWith) {
                let pair = normalize_pair(edge.source_id, *uid);
                if edge.weight >= params.min_strength && seen_pairs.insert(pair) {
                    couplings.push(Coupling {
                        unit_a: pair.0,
                        unit_b: pair.1,
                        strength: edge.weight,
                        kind: CouplingKind::Temporal,
                    });
                }
            }
        }

        // Hidden couplings: units that share many outgoing targets.
        if let Some(uid) = params.unit_id {
            let my_targets: HashSet<u64> = graph
                .edges_from(uid)
                .iter()
                .filter(|e| e.edge_type.is_dependency())
                .map(|e| e.target_id)
                .collect();

            if !my_targets.is_empty() {
                for other_unit in graph.units() {
                    if other_unit.id == uid {
                        continue;
                    }
                    let other_targets: HashSet<u64> = graph
                        .edges_from(other_unit.id)
                        .iter()
                        .filter(|e| e.edge_type.is_dependency())
                        .map(|e| e.target_id)
                        .collect();

                    if other_targets.is_empty() {
                        continue;
                    }

                    let intersection = my_targets.intersection(&other_targets).count();
                    let union = my_targets.union(&other_targets).count();
                    let jaccard = if union > 0 {
                        intersection as f32 / union as f32
                    } else {
                        0.0
                    };

                    if jaccard >= params.min_strength {
                        let pair = normalize_pair(uid, other_unit.id);
                        if seen_pairs.insert(pair) {
                            couplings.push(Coupling {
                                unit_a: pair.0,
                                unit_b: pair.1,
                                strength: jaccard,
                                kind: CouplingKind::Hidden,
                            });
                        }
                    }
                }
            }
        }

        // Sort by strength descending.
        couplings.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(couplings)
    }

    // ========================================================================
    // Query 16: Dead Code
    // ========================================================================

    /// Detect potentially dead (unreachable) code.
    pub fn dead_code<'g>(
        &self,
        graph: &'g CodeGraph,
        params: DeadCodeParams,
    ) -> AcbResult<Vec<&'g CodeUnit>> {
        // Identify entry points: main functions, tests, public module exports.
        let mut roots: HashSet<u64> = HashSet::new();

        for unit in graph.units() {
            let is_entry = match unit.unit_type {
                CodeUnitType::Function => {
                    unit.name == "main"
                        || unit.name.starts_with("test_")
                        || unit.visibility == Visibility::Public
                }
                CodeUnitType::Test => true,
                CodeUnitType::Module => {
                    // Modules are implicit roots.
                    unit.visibility == Visibility::Public
                }
                _ => unit.visibility == Visibility::Public,
            };
            if is_entry {
                roots.insert(unit.id);
            }
        }

        if params.include_tests_as_roots {
            for unit in graph.find_units_by_type(CodeUnitType::Test) {
                roots.insert(unit.id);
            }
        }

        // Run reachability from all roots.
        let mut reachable: HashSet<u64> = HashSet::new();
        let opts = TraversalOptions {
            max_depth: -1,
            edge_types: Vec::new(), // all edge types
            direction: Direction::Forward,
        };

        for root_id in &roots {
            let visited = traversal::bfs(graph, *root_id, &opts);
            for (id, _) in visited {
                reachable.insert(id);
            }
        }

        // Also run backward reachability from roots to catch units
        // that are reached via incoming edges (e.g., tests targeting roots).
        let back_opts = TraversalOptions {
            max_depth: -1,
            edge_types: Vec::new(),
            direction: Direction::Backward,
        };
        for root_id in &roots {
            let visited = traversal::bfs(graph, *root_id, &back_opts);
            for (id, _) in visited {
                reachable.insert(id);
            }
        }

        // Find unreachable units.
        let mut dead: Vec<&CodeUnit> = graph
            .units()
            .iter()
            .filter(|u| {
                if params.unit_types.is_empty() {
                    true
                } else {
                    params.unit_types.contains(&u.unit_type)
                }
            })
            .filter(|u| !reachable.contains(&u.id))
            .collect();

        // Sort by name for deterministic output.
        dead.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(dead)
    }

    // ========================================================================
    // Query 17: Prophecy
    // ========================================================================

    /// Predict likely future breakages based on code patterns.
    pub fn prophecy(&self, graph: &CodeGraph, params: ProphecyParams) -> AcbResult<ProphecyResult> {
        let mut predictions = Vec::new();

        for unit in graph.units() {
            let mut risk = 0.0_f32;
            let mut reasons = Vec::new();

            // Low stability = higher risk.
            if unit.stability_score < 0.3 {
                risk += 0.4;
                reasons.push(format!("Low stability score ({:.2})", unit.stability_score));
            } else if unit.stability_score < 0.6 {
                risk += 0.2;
                reasons.push(format!(
                    "Moderate stability score ({:.2})",
                    unit.stability_score
                ));
            }

            // High complexity = higher risk.
            if unit.complexity > 20 {
                risk += 0.3;
                reasons.push(format!("High complexity ({})", unit.complexity));
            } else if unit.complexity > 10 {
                risk += 0.15;
                reasons.push(format!("Moderate complexity ({})", unit.complexity));
            }

            // High change count = higher risk.
            if unit.change_count > 50 {
                risk += 0.2;
                reasons.push(format!("Frequently changed ({} times)", unit.change_count));
            } else if unit.change_count > 20 {
                risk += 0.1;
                reasons.push(format!("Changed {} times", unit.change_count));
            }

            // BreaksWith edges = historical breakage indicator.
            let breaks_count = graph
                .edges_from_of_type(unit.id, EdgeType::BreaksWith)
                .len()
                + graph.edges_to_of_type(unit.id, EdgeType::BreaksWith).len();
            if breaks_count > 0 {
                risk += 0.2 * (breaks_count as f32).min(3.0) / 3.0;
                reasons.push(format!(
                    "{} historical breakage relationships",
                    breaks_count
                ));
            }

            // No test coverage = higher risk.
            let test_count = graph.edges_to_of_type(unit.id, EdgeType::Tests).len();
            if test_count == 0 && unit.unit_type == CodeUnitType::Function {
                risk += 0.1;
                reasons.push("No test coverage".to_string());
            }

            risk = risk.min(1.0);

            if risk >= params.min_risk {
                predictions.push(Prediction {
                    unit_id: unit.id,
                    risk_score: risk,
                    reason: reasons.join("; "),
                });
            }
        }

        // Sort by risk descending.
        predictions.sort_by(|a, b| {
            b.risk_score
                .partial_cmp(&a.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if params.top_k > 0 {
            predictions.truncate(params.top_k);
        }

        Ok(ProphecyResult { predictions })
    }

    // ========================================================================
    // Query 18: Concept Mapping
    // ========================================================================

    /// Map a concept to related code units.
    pub fn concept_mapping(&self, graph: &CodeGraph, concept: &str) -> AcbResult<ConceptMap> {
        let concept_lower = concept.to_lowercase();
        let mut units = Vec::new();

        for unit in graph.units() {
            let name_match = unit.name.to_lowercase().contains(&concept_lower)
                || unit.qualified_name.to_lowercase().contains(&concept_lower);
            let doc_match = unit
                .doc_summary
                .as_ref()
                .map(|d| d.to_lowercase().contains(&concept_lower))
                .unwrap_or(false);

            if !name_match && !doc_match {
                continue;
            }

            // Determine the role.
            let role = match unit.unit_type {
                CodeUnitType::Type | CodeUnitType::Trait | CodeUnitType::Module => {
                    ConceptRole::Definition
                }
                CodeUnitType::Test => ConceptRole::Test,
                CodeUnitType::Impl => ConceptRole::Extension,
                _ => {
                    // Check if this unit extends something related to the concept.
                    let has_inherit = graph
                        .edges_from_of_type(unit.id, EdgeType::Inherits)
                        .iter()
                        .any(|e| {
                            graph
                                .get_unit(e.target_id)
                                .map(|t| t.name.to_lowercase().contains(&concept_lower))
                                .unwrap_or(false)
                        });
                    if has_inherit {
                        ConceptRole::Extension
                    } else {
                        ConceptRole::Usage
                    }
                }
            };

            // Compute relevance.
            let mut relevance = 0.0_f32;
            if unit.name.to_lowercase() == concept_lower {
                relevance = 1.0;
            } else if name_match {
                relevance = 0.7;
            }
            if doc_match {
                relevance = (relevance + 0.3).min(1.0);
            }

            units.push(ConceptUnit {
                unit_id: unit.id,
                role,
                relevance,
            });
        }

        // Sort by relevance descending.
        units.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(ConceptMap {
            concept: concept.to_string(),
            units,
        })
    }

    // ========================================================================
    // Query 19: Migration Path
    // ========================================================================

    /// Plan a migration from one unit to another.
    pub fn migration_path(
        &self,
        graph: &CodeGraph,
        params: MigrationParams,
    ) -> AcbResult<MigrationPlan> {
        self.validate_unit(graph, params.from_unit)?;
        self.validate_unit(graph, params.to_unit)?;

        // Find all dependents of from_unit (reverse deps).
        let dep_params = DependencyParams {
            unit_id: params.from_unit,
            max_depth: 10,
            edge_types: vec![
                EdgeType::Calls,
                EdgeType::Imports,
                EdgeType::UsesType,
                EdgeType::Inherits,
                EdgeType::Implements,
            ],
            include_transitive: false,
        };
        let deps = self.reverse_dependency(graph, dep_params)?;

        let mut steps: Vec<MigrationStep> = Vec::new();
        let mut order = 0u32;

        // Step 0: The target unit itself (create/prepare).
        steps.push(MigrationStep {
            unit_id: params.to_unit,
            order,
            safety: SafetyLevel::Safe,
            description: "Prepare the target unit as the replacement.".to_string(),
        });
        order += 1;

        // Sort dependents by safety: tested first, then untested.
        let mut dependent_steps: Vec<(u64, SafetyLevel)> = Vec::new();

        for node in &deps.nodes {
            let has_direct_tests = !graph
                .edges_to_of_type(node.unit_id, EdgeType::Tests)
                .is_empty();
            let callers = graph.edges_to_of_type(node.unit_id, EdgeType::Calls);
            let caller_tested = callers.iter().any(|e| {
                !graph
                    .edges_to_of_type(e.source_id, EdgeType::Tests)
                    .is_empty()
            });

            let safety = if has_direct_tests {
                SafetyLevel::Safe
            } else if caller_tested {
                SafetyLevel::Caution
            } else {
                SafetyLevel::Risky
            };

            dependent_steps.push((node.unit_id, safety));
        }

        // Sort: Safe first, then Caution, then Risky.
        dependent_steps.sort_by(|a, b| a.1.cmp(&b.1));

        for (uid, safety) in dependent_steps {
            let unit_name = graph
                .get_unit(uid)
                .map(|u| u.qualified_name.clone())
                .unwrap_or_else(|| format!("unit_{}", uid));
            let desc = match safety {
                SafetyLevel::Safe => {
                    format!("Update {} (has tests, safe to migrate).", unit_name)
                }
                SafetyLevel::Caution => {
                    format!(
                        "Update {} (no direct tests, but callers are tested — exercise caution).",
                        unit_name
                    )
                }
                SafetyLevel::Risky => {
                    format!(
                        "Update {} (no test coverage — add tests before migrating).",
                        unit_name
                    )
                }
            };

            steps.push(MigrationStep {
                unit_id: uid,
                order,
                safety,
                description: desc,
            });
            order += 1;
        }

        // Final step: remove the old unit.
        steps.push(MigrationStep {
            unit_id: params.from_unit,
            order,
            safety: SafetyLevel::Caution,
            description: "Remove or deprecate the original unit.".to_string(),
        });

        Ok(MigrationPlan {
            from_unit: params.from_unit,
            to_unit: params.to_unit,
            steps,
        })
    }

    // ========================================================================
    // Query 20: Test Gap
    // ========================================================================

    /// Find units that have high change counts or complexity but no tests.
    pub fn test_gap(&self, graph: &CodeGraph, params: TestGapParams) -> AcbResult<Vec<TestGap>> {
        let target_types = if params.unit_types.is_empty() {
            vec![CodeUnitType::Function]
        } else {
            params.unit_types
        };

        let mut gaps = Vec::new();

        for unit in graph.units() {
            if !target_types.contains(&unit.unit_type) {
                continue;
            }

            let has_tests = !graph.edges_to_of_type(unit.id, EdgeType::Tests).is_empty();
            if has_tests {
                continue;
            }

            let high_changes = unit.change_count >= params.min_changes;
            let high_complexity = unit.complexity >= params.min_complexity;

            if !high_changes && !high_complexity {
                continue;
            }

            let mut reasons = Vec::new();
            if high_changes {
                reasons.push(format!("changed {} times", unit.change_count));
            }
            if high_complexity {
                reasons.push(format!("complexity {}", unit.complexity));
            }

            // Priority: higher change count and higher complexity = more urgent.
            let change_score =
                (unit.change_count as f32 / params.min_changes.max(1) as f32).min(2.0);
            let complexity_score =
                (unit.complexity as f32 / params.min_complexity.max(1) as f32).min(2.0);
            let priority = (change_score * 0.5 + complexity_score * 0.5).min(1.0);

            gaps.push(TestGap {
                unit_id: unit.id,
                reason: format!("No tests, but {}", reasons.join(" and ")),
                priority,
            });
        }

        // Sort by priority descending.
        gaps.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(gaps)
    }

    // ========================================================================
    // Query 21: Architectural Drift
    // ========================================================================

    /// Check for architectural drift against declared rules.
    pub fn architectural_drift(
        &self,
        graph: &CodeGraph,
        params: DriftParams,
    ) -> AcbResult<DriftReport> {
        let mut violations = Vec::new();

        for (rule_idx, rule) in params.rules.iter().enumerate() {
            match rule {
                ArchRule::LayerDependency { upper, lower } => {
                    // Lower layer must not depend on upper layer.
                    let lower_units: Vec<&CodeUnit> = graph
                        .units()
                        .iter()
                        .filter(|u| u.qualified_name.starts_with(lower.as_str()))
                        .collect();

                    for lu in &lower_units {
                        for edge in graph.edges_from(lu.id) {
                            if !edge.edge_type.is_dependency() {
                                continue;
                            }
                            if let Some(target) = graph.get_unit(edge.target_id) {
                                if target.qualified_name.starts_with(upper.as_str()) {
                                    violations.push(DriftViolation {
                                        rule_index: rule_idx,
                                        description: format!(
                                            "Lower layer '{}' depends on upper layer '{}': {} -> {}",
                                            lower, upper, lu.qualified_name, target.qualified_name
                                        ),
                                        units: vec![lu.id, target.id],
                                    });
                                }
                            }
                        }
                    }
                }
                ArchRule::ModuleBoundary { module } => {
                    // Units in the module must not have dependencies outside.
                    let module_units: Vec<&CodeUnit> = graph
                        .units()
                        .iter()
                        .filter(|u| u.qualified_name.starts_with(module.as_str()))
                        .collect();

                    for mu in &module_units {
                        for edge in graph.edges_from(mu.id) {
                            if !edge.edge_type.is_dependency() {
                                continue;
                            }
                            if let Some(target) = graph.get_unit(edge.target_id) {
                                if !target.qualified_name.starts_with(module.as_str()) {
                                    violations.push(DriftViolation {
                                        rule_index: rule_idx,
                                        description: format!(
                                            "Module boundary violation: {} depends on external {}",
                                            mu.qualified_name, target.qualified_name
                                        ),
                                        units: vec![mu.id, target.id],
                                    });
                                }
                            }
                        }
                    }
                }
                ArchRule::NamingConvention { prefix, pattern } => {
                    // Simple naming convention check.
                    // The pattern is treated as a required prefix or substring
                    // depending on its form:
                    //   - "test_*" means must start with "test_"
                    //   - "*_impl" means must end with "_impl"
                    //   - "*foo*" means must contain "foo"
                    //   - "exact" means must equal "exact"
                    let (check_start, check_end, _check_contains, literal) =
                        parse_simple_glob(pattern);

                    for unit in graph.units() {
                        if !unit.qualified_name.starts_with(prefix.as_str()) {
                            continue;
                        }

                        let name_lower = unit.name.to_lowercase();
                        let lit_lower = literal.to_lowercase();

                        let name_matches = if check_start && check_end {
                            // *foo* — contains
                            name_lower.contains(&lit_lower)
                        } else if check_start {
                            // *foo — ends with
                            name_lower.ends_with(&lit_lower)
                        } else if check_end {
                            // foo* — starts with
                            name_lower.starts_with(&lit_lower)
                        } else {
                            // exact match
                            name_lower == lit_lower
                        };

                        if !name_matches {
                            violations.push(DriftViolation {
                                rule_index: rule_idx,
                                description: format!(
                                    "Naming convention violation: '{}' does not match pattern '{}'",
                                    unit.name, pattern
                                ),
                                units: vec![unit.id],
                            });
                        }
                    }
                }
                ArchRule::Cyclic { scope } => {
                    // Check for cycles within the scoped units.
                    let scoped_ids: HashSet<u64> = graph
                        .units()
                        .iter()
                        .filter(|u| u.qualified_name.starts_with(scope.as_str()))
                        .map(|u| u.id)
                        .collect();

                    if let Some(cycle) = self.detect_cycle(graph, &scoped_ids) {
                        let description = format!(
                            "Dependency cycle detected in scope '{}': {}",
                            scope,
                            cycle
                                .iter()
                                .filter_map(|id| graph.get_unit(*id).map(|u| u.name.clone()))
                                .collect::<Vec<_>>()
                                .join(" -> ")
                        );
                        violations.push(DriftViolation {
                            rule_index: rule_idx,
                            description,
                            units: cycle,
                        });
                    }
                }
            }
        }

        let total_rules = params.rules.len();
        let violated_rules: HashSet<usize> = violations.iter().map(|v| v.rule_index).collect();
        let conformance_score = if total_rules > 0 {
            1.0 - (violated_rules.len() as f32 / total_rules as f32)
        } else {
            1.0
        };

        Ok(DriftReport {
            violations,
            conformance_score,
        })
    }

    // ========================================================================
    // Query 22: Similarity
    // ========================================================================

    /// Find units similar to a given unit by feature vector.
    pub fn similarity(
        &self,
        graph: &CodeGraph,
        params: SimilarityParams,
    ) -> AcbResult<Vec<SimilarityMatch>> {
        let target = graph
            .get_unit(params.unit_id)
            .ok_or(AcbError::UnitNotFound(params.unit_id))?;

        let target_vec = target.feature_vec.clone();

        let mut matches: Vec<SimilarityMatch> = graph
            .units()
            .iter()
            .filter(|u| u.id != params.unit_id)
            .filter_map(|u| {
                let score = cosine_similarity(&target_vec, &u.feature_vec);
                if score >= params.min_similarity {
                    Some(SimilarityMatch {
                        unit_id: u.id,
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if params.top_k > 0 {
            matches.truncate(params.top_k);
        }

        Ok(matches)
    }

    // ========================================================================
    // Query 23: Shortest Path
    // ========================================================================

    /// Find the shortest path between two units.
    pub fn shortest_path(&self, graph: &CodeGraph, from: u64, to: u64) -> AcbResult<PathResult> {
        self.validate_unit(graph, from)?;
        self.validate_unit(graph, to)?;

        match traversal::shortest_path(graph, from, to, &[]) {
            Some(path) => {
                // Reconstruct edge types along the path.
                let mut edge_types = Vec::new();
                for window in path.windows(2) {
                    let src = window[0];
                    let tgt = window[1];
                    let et = graph
                        .edges_from(src)
                        .iter()
                        .find(|e| e.target_id == tgt)
                        .map(|e| e.edge_type)
                        .unwrap_or(EdgeType::References);
                    edge_types.push(et);
                }
                let length = path.len().saturating_sub(1);
                Ok(PathResult {
                    found: true,
                    path,
                    edge_types,
                    length,
                })
            }
            None => Ok(PathResult {
                found: false,
                path: Vec::new(),
                edge_types: Vec::new(),
                length: 0,
            }),
        }
    }

    // ========================================================================
    // Query 24: Hotspot Detection
    // ========================================================================

    /// Detect code hotspots based on change frequency, stability, and complexity.
    pub fn hotspot_detection(
        &self,
        graph: &CodeGraph,
        params: HotspotParams,
    ) -> AcbResult<Vec<Hotspot>> {
        let mut hotspots = Vec::new();

        for unit in graph.units() {
            if !params.unit_types.is_empty() && !params.unit_types.contains(&unit.unit_type) {
                continue;
            }

            let mut factors: HashMap<String, f32> = HashMap::new();

            // Change frequency factor.
            let change_factor = (unit.change_count as f32 / 50.0).min(1.0);
            factors.insert("change_frequency".to_string(), change_factor);

            // Instability factor (inverted stability).
            let instability = 1.0 - unit.stability_score;
            factors.insert("instability".to_string(), instability);

            // Complexity factor.
            let complexity_factor = (unit.complexity as f32 / 30.0).min(1.0);
            factors.insert("complexity".to_string(), complexity_factor);

            // Coupling factor.
            let coupling_count = graph.edges_from(unit.id).len() + graph.edges_to(unit.id).len();
            let coupling_factor = (coupling_count as f32 / 20.0).min(1.0);
            factors.insert("coupling".to_string(), coupling_factor);

            // Weighted score.
            let score = change_factor * 0.35
                + instability * 0.25
                + complexity_factor * 0.25
                + coupling_factor * 0.15;

            if score >= params.min_score {
                hotspots.push(Hotspot {
                    unit_id: unit.id,
                    score,
                    factors,
                });
            }
        }

        // Sort by score descending.
        hotspots.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if params.top_k > 0 {
            hotspots.truncate(params.top_k);
        }

        Ok(hotspots)
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    /// Validate that a unit ID exists in the graph.
    fn validate_unit(&self, graph: &CodeGraph, unit_id: u64) -> AcbResult<()> {
        if graph.get_unit(unit_id).is_none() {
            return Err(AcbError::UnitNotFound(unit_id));
        }
        Ok(())
    }

    /// Collect call sites following forward Calls edges (BFS).
    fn collect_call_sites_forward(
        &self,
        graph: &CodeGraph,
        start_id: u64,
        max_depth: u32,
        call_sites: &mut Vec<CallSite>,
    ) {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(start_id);
        queue.push_back((start_id, 0u32));

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in graph.edges_from_of_type(current, EdgeType::Calls) {
                call_sites.push(CallSite {
                    caller_id: current,
                    callee_id: edge.target_id,
                    span: Span::point(edge.context, 0),
                });
                if visited.insert(edge.target_id) {
                    queue.push_back((edge.target_id, depth + 1));
                }
            }
        }
    }

    /// Collect call sites following backward Calls edges (BFS).
    fn collect_call_sites_backward(
        &self,
        graph: &CodeGraph,
        start_id: u64,
        max_depth: u32,
        call_sites: &mut Vec<CallSite>,
    ) {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(start_id);
        queue.push_back((start_id, 0u32));

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in graph.edges_to_of_type(current, EdgeType::Calls) {
                call_sites.push(CallSite {
                    caller_id: edge.source_id,
                    callee_id: current,
                    span: Span::point(edge.context, 0),
                });
                if visited.insert(edge.source_id) {
                    queue.push_back((edge.source_id, depth + 1));
                }
            }
        }
    }

    /// Parse the `calls: [A, B, C]` list from a pattern string.
    fn parse_call_list(&self, pattern: &str) -> Vec<String> {
        if let Some(start) = pattern.find('[') {
            if let Some(end) = pattern.find(']') {
                if start < end {
                    let inner = &pattern[start + 1..end];
                    return inner
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }
        Vec::new()
    }

    /// Parse the `inherits: Base` target name from a pattern string.
    fn parse_inherits_target(&self, pattern: &str) -> Option<String> {
        if let Some(pos) = pattern.find("inherits:") {
            let after = &pattern[pos + "inherits:".len()..];
            let trimmed = after.trim().trim_end_matches('}').trim();
            let name = trimmed.split_whitespace().next()?;
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
        None
    }

    /// Parse the `complexity: >N` constraint from a pattern string.
    fn parse_complexity_constraint(&self, pattern: &str) -> Option<u32> {
        if let Some(pos) = pattern.find("complexity:") {
            let after = &pattern[pos + "complexity:".len()..];
            let trimmed = after.trim().trim_start_matches('>').trim();
            let num_str: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
            return num_str.parse::<u32>().ok();
        }
        None
    }

    /// Detect a cycle in a subset of the graph using DFS-based cycle detection.
    ///
    /// Returns the first cycle found as a vector of unit IDs, or `None`.
    fn detect_cycle(&self, graph: &CodeGraph, scope: &HashSet<u64>) -> Option<Vec<u64>> {
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();
        let mut stack = Vec::new();

        for &uid in scope {
            if !visited.contains(&uid) {
                if let Some(cycle) = self.detect_cycle_dfs(
                    graph,
                    uid,
                    scope,
                    &mut visited,
                    &mut in_stack,
                    &mut stack,
                ) {
                    return Some(cycle);
                }
            }
        }
        None
    }

    #[allow(clippy::only_used_in_recursion)]
    fn detect_cycle_dfs(
        &self,
        graph: &CodeGraph,
        uid: u64,
        scope: &HashSet<u64>,
        visited: &mut HashSet<u64>,
        in_stack: &mut HashSet<u64>,
        stack: &mut Vec<u64>,
    ) -> Option<Vec<u64>> {
        visited.insert(uid);
        in_stack.insert(uid);
        stack.push(uid);

        for edge in graph.edges_from(uid) {
            if !edge.edge_type.is_dependency() {
                continue;
            }
            let target = edge.target_id;
            if !scope.contains(&target) {
                continue;
            }

            if !visited.contains(&target) {
                if let Some(cycle) =
                    self.detect_cycle_dfs(graph, target, scope, visited, in_stack, stack)
                {
                    return Some(cycle);
                }
            } else if in_stack.contains(&target) {
                // Found a cycle — extract it from the stack.
                let pos = stack.iter().position(|&x| x == target)?;
                let mut cycle: Vec<u64> = stack[pos..].to_vec();
                cycle.push(target); // close the loop
                return Some(cycle);
            }
        }

        stack.pop();
        in_stack.remove(&uid);
        None
    }
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a simple glob pattern into (starts_with_star, ends_with_star, is_contains, literal).
///
/// Examples:
/// - `"test_*"` -> `(false, true, false, "test_")`
/// - `"*_impl"` -> `(true, false, false, "_impl")`
/// - `"*foo*"` -> `(true, true, true, "foo")`
/// - `"exact"` -> `(false, false, false, "exact")`
fn parse_simple_glob(pattern: &str) -> (bool, bool, bool, String) {
    let starts = pattern.starts_with('*');
    let ends = pattern.ends_with('*');
    let literal = pattern
        .trim_start_matches('*')
        .trim_end_matches('*')
        .to_string();
    let contains = starts && ends;
    (starts, ends, contains, literal)
}

/// Normalize a pair of IDs so the smaller comes first (for deduplication).
fn normalize_pair(a: u64, b: u64) -> (u64, u64) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CodeUnit, CodeUnitType, Edge, EdgeType, Language, Span};
    use std::path::PathBuf;

    /// Build a small test graph for query tests.
    fn build_test_graph() -> CodeGraph {
        let mut graph = CodeGraph::with_default_dimension();

        // 0: Module "app"
        let m = CodeUnit::new(
            CodeUnitType::Module,
            Language::Rust,
            "app".to_string(),
            "app".to_string(),
            PathBuf::from("src/lib.rs"),
            Span::new(1, 0, 100, 0),
        );
        graph.add_unit(m);

        // 1: Function "process"
        let mut f1 = CodeUnit::new(
            CodeUnitType::Function,
            Language::Rust,
            "process".to_string(),
            "app::process".to_string(),
            PathBuf::from("src/lib.rs"),
            Span::new(10, 0, 20, 0),
        );
        f1.complexity = 5;
        f1.visibility = Visibility::Public;
        graph.add_unit(f1);

        // 2: Function "helper"
        let mut f2 = CodeUnit::new(
            CodeUnitType::Function,
            Language::Rust,
            "helper".to_string(),
            "app::helper".to_string(),
            PathBuf::from("src/lib.rs"),
            Span::new(25, 0, 35, 0),
        );
        f2.complexity = 2;
        f2.visibility = Visibility::Private;
        graph.add_unit(f2);

        // 3: Test "test_process"
        let t = CodeUnit::new(
            CodeUnitType::Test,
            Language::Rust,
            "test_process".to_string(),
            "app::test_process".to_string(),
            PathBuf::from("src/lib.rs"),
            Span::new(40, 0, 50, 0),
        );
        graph.add_unit(t);

        // 4: Type "Config"
        let ty = CodeUnit::new(
            CodeUnitType::Type,
            Language::Rust,
            "Config".to_string(),
            "app::Config".to_string(),
            PathBuf::from("src/lib.rs"),
            Span::new(55, 0, 65, 0),
        );
        graph.add_unit(ty);

        // Edges:
        // app contains process, helper, test_process, Config
        graph.add_edge(Edge::new(0, 1, EdgeType::Contains)).ok();
        graph.add_edge(Edge::new(0, 2, EdgeType::Contains)).ok();
        graph.add_edge(Edge::new(0, 3, EdgeType::Contains)).ok();
        graph.add_edge(Edge::new(0, 4, EdgeType::Contains)).ok();

        // process calls helper
        graph
            .add_edge(Edge::new(1, 2, EdgeType::Calls).with_context(15))
            .ok();

        // test_process tests process
        graph.add_edge(Edge::new(3, 1, EdgeType::Tests)).ok();

        graph
    }

    #[test]
    fn test_symbol_lookup_exact() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let params = SymbolLookupParams {
            name: "process".to_string(),
            mode: MatchMode::Exact,
            ..Default::default()
        };

        let result = engine.symbol_lookup(&graph, params).expect("lookup failed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "process");
    }

    #[test]
    fn test_symbol_lookup_prefix() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let params = SymbolLookupParams {
            name: "proc".to_string(),
            mode: MatchMode::Prefix,
            ..Default::default()
        };

        let result = engine.symbol_lookup(&graph, params).expect("lookup failed");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_symbol_lookup_contains() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let params = SymbolLookupParams {
            name: "elp".to_string(),
            mode: MatchMode::Contains,
            ..Default::default()
        };

        let result = engine.symbol_lookup(&graph, params).expect("lookup failed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "helper");
    }

    #[test]
    fn test_containment() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let result = engine.containment(&graph, 0).expect("containment failed");
        assert_eq!(result.len(), 4); // process, helper, test_process, Config
    }

    #[test]
    fn test_call_graph_callees() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let params = CallGraphParams {
            unit_id: 1,
            direction: CallDirection::Callees,
            max_depth: 3,
        };

        let result = engine
            .call_graph(&graph, params)
            .expect("call graph failed");
        assert!(result.nodes.len() >= 2); // process + helper
        assert!(!result.call_sites.is_empty());
    }

    #[test]
    fn test_test_coverage() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let result = engine.test_coverage(&graph, 1).expect("coverage failed");
        assert_eq!(result.direct_tests.len(), 1);
        assert_eq!(result.direct_tests[0], 3);
    }

    #[test]
    fn test_shortest_path_found() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let result = engine.shortest_path(&graph, 1, 2).expect("path failed");
        assert!(result.found);
        assert_eq!(result.path, vec![1, 2]);
        assert_eq!(result.length, 1);
    }

    #[test]
    fn test_shortest_path_not_found() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        // No path from helper (2) to app (0) via forward edges.
        let result = engine.shortest_path(&graph, 2, 0).expect("path failed");
        assert!(!result.found);
    }

    #[test]
    fn test_dependency_graph() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let params = DependencyParams {
            unit_id: 1,
            max_depth: 3,
            edge_types: vec![EdgeType::Calls],
            include_transitive: true,
        };

        let result = engine
            .dependency_graph(&graph, params)
            .expect("dep graph failed");
        assert_eq!(result.root_id, 1);
        assert!(!result.nodes.is_empty());
    }

    #[test]
    fn test_reverse_dependency() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let params = DependencyParams {
            unit_id: 2,
            max_depth: 3,
            edge_types: vec![EdgeType::Calls],
            include_transitive: true,
        };

        let result = engine
            .reverse_dependency(&graph, params)
            .expect("rev dep failed");
        // process calls helper, so process should appear as a reverse dep of helper.
        assert!(result.nodes.iter().any(|n| n.unit_id == 1));
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", ""), 3);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 1.0];
        let b = vec![1.0, 0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn test_unit_not_found_error() {
        let graph = build_test_graph();
        let engine = QueryEngine::new();

        let result = engine.containment(&graph, 999);
        assert!(result.is_err());
    }
}
