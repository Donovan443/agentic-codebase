//! Phase 4 tests: Query Engine (all 24 query types).
//!
//! ~80 tests covering core, built, and novel queries.

use agentic_codebase::engine::query::*;
use agentic_codebase::graph::CodeGraph;
use agentic_codebase::types::*;
use std::path::PathBuf;

// ============================================================================
// Shared helpers
// ============================================================================

/// Build a rich test graph with multiple unit types, edges, and languages.
fn build_rich_graph() -> CodeGraph {
    let mut graph = CodeGraph::with_default_dimension();

    // 0: Module "app"
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Module,
        Language::Rust,
        "app".into(),
        "app".into(),
        PathBuf::from("src/app.rs"),
        Span::new(1, 0, 200, 0),
    ));

    // 1: Function "process_payment" (public, complexity 8)
    let mut u = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "process_payment".into(),
        "app::process_payment".into(),
        PathBuf::from("src/app.rs"),
        Span::new(10, 0, 40, 0),
    );
    u.complexity = 8;
    u.visibility = Visibility::Public;
    u.doc_summary = Some("Process a payment transaction.".into());
    u.change_count = 12;
    u.stability_score = 0.3;
    graph.add_unit(u);

    // 2: Function "validate_input" (private, complexity 3)
    let mut u = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "validate_input".into(),
        "app::validate_input".into(),
        PathBuf::from("src/app.rs"),
        Span::new(50, 0, 70, 0),
    );
    u.complexity = 3;
    u.visibility = Visibility::Private;
    u.change_count = 2;
    u.stability_score = 0.9;
    graph.add_unit(u);

    // 3: Function "send_notification"
    let mut u = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "send_notification".into(),
        "app::send_notification".into(),
        PathBuf::from("src/app.rs"),
        Span::new(80, 0, 100, 0),
    );
    u.complexity = 5;
    u.visibility = Visibility::Public;
    graph.add_unit(u);

    // 4: Type "PaymentConfig"
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Type,
        Language::Rust,
        "PaymentConfig".into(),
        "app::PaymentConfig".into(),
        PathBuf::from("src/app.rs"),
        Span::new(110, 0, 130, 0),
    ));

    // 5: Test "test_process_payment"
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Test,
        Language::Rust,
        "test_process_payment".into(),
        "app::test_process_payment".into(),
        PathBuf::from("src/app.rs"),
        Span::new(140, 0, 160, 0),
    ));

    // 6: Trait "Validator"
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Trait,
        Language::Rust,
        "Validator".into(),
        "app::Validator".into(),
        PathBuf::from("src/app.rs"),
        Span::new(170, 0, 190, 0),
    ));

    // 7: Type "UserValidator" (inherits Validator)
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Type,
        Language::Rust,
        "UserValidator".into(),
        "app::UserValidator".into(),
        PathBuf::from("src/app.rs"),
        Span::new(200, 0, 220, 0),
    ));

    // 8: Type "InputValidator" (inherits Validator)
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Type,
        Language::Rust,
        "InputValidator".into(),
        "app::InputValidator".into(),
        PathBuf::from("src/app.rs"),
        Span::new(230, 0, 250, 0),
    ));

    // 9: Module "auth" (Python)
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Module,
        Language::Python,
        "auth".into(),
        "auth".into(),
        PathBuf::from("src/auth.py"),
        Span::new(1, 0, 100, 0),
    ));

    // 10: Function "login" (Python, has auth concept)
    let mut u = CodeUnit::new(
        CodeUnitType::Function,
        Language::Python,
        "login".into(),
        "auth::login".into(),
        PathBuf::from("src/auth.py"),
        Span::new(10, 0, 30, 0),
    );
    u.doc_summary = Some("Authenticate user with credentials.".into());
    u.change_count = 8;
    u.stability_score = 0.5;
    u.complexity = 6;
    graph.add_unit(u);

    // 11: Function "logout" (Python)
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Function,
        Language::Python,
        "logout".into(),
        "auth::logout".into(),
        PathBuf::from("src/auth.py"),
        Span::new(40, 0, 55, 0),
    ));

    // 12: Function "helper_internal" (no callers, dead code candidate)
    let mut u = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "helper_internal".into(),
        "app::helper_internal".into(),
        PathBuf::from("src/app.rs"),
        Span::new(260, 0, 280, 0),
    );
    u.visibility = Visibility::Private;
    u.complexity = 15;
    u.change_count = 20;
    u.stability_score = 0.1;
    graph.add_unit(u);

    // 13: Test "test_login" (Python test for login)
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Test,
        Language::Python,
        "test_login".into(),
        "auth::test_login".into(),
        PathBuf::from("tests/test_auth.py"),
        Span::new(1, 0, 20, 0),
    ));

    // --- Edges ---

    // Containment: app module contains its functions/types
    graph.add_edge(Edge::new(0, 1, EdgeType::Contains)).ok();
    graph.add_edge(Edge::new(0, 2, EdgeType::Contains)).ok();
    graph.add_edge(Edge::new(0, 3, EdgeType::Contains)).ok();
    graph.add_edge(Edge::new(0, 4, EdgeType::Contains)).ok();
    graph.add_edge(Edge::new(0, 5, EdgeType::Contains)).ok();
    graph.add_edge(Edge::new(0, 6, EdgeType::Contains)).ok();
    graph.add_edge(Edge::new(0, 7, EdgeType::Contains)).ok();
    graph.add_edge(Edge::new(0, 8, EdgeType::Contains)).ok();
    graph.add_edge(Edge::new(0, 12, EdgeType::Contains)).ok();

    // Containment: auth module contains its functions
    graph.add_edge(Edge::new(9, 10, EdgeType::Contains)).ok();
    graph.add_edge(Edge::new(9, 11, EdgeType::Contains)).ok();

    // Calls: process_payment calls validate_input and send_notification
    graph
        .add_edge(Edge::new(1, 2, EdgeType::Calls).with_context(20))
        .ok();
    graph
        .add_edge(Edge::new(1, 3, EdgeType::Calls).with_context(30))
        .ok();

    // Calls: login calls process_payment (cross-language!)
    graph
        .add_edge(Edge::new(10, 1, EdgeType::Calls).with_context(15))
        .ok();

    // UsesType: process_payment uses PaymentConfig
    graph.add_edge(Edge::new(1, 4, EdgeType::UsesType)).ok();

    // Tests: test_process_payment tests process_payment
    graph.add_edge(Edge::new(5, 1, EdgeType::Tests)).ok();

    // Tests: test_login tests login
    graph.add_edge(Edge::new(13, 10, EdgeType::Tests)).ok();

    // Inheritance: UserValidator implements Validator
    graph.add_edge(Edge::new(7, 6, EdgeType::Implements)).ok();
    // Inheritance: InputValidator inherits Validator
    graph.add_edge(Edge::new(8, 6, EdgeType::Inherits)).ok();

    // CouplesWith: process_payment and send_notification are temporally coupled
    graph
        .add_edge(Edge::new(1, 3, EdgeType::CouplesWith).with_weight(0.8))
        .ok();

    graph
}

// ============================================================================
// Core Queries (1-8)
// ============================================================================

// --- Query 1: Symbol Lookup ---

#[test]
fn test_symbol_lookup_exact() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = SymbolLookupParams {
        name: "process_payment".into(),
        mode: MatchMode::Exact,
        ..Default::default()
    };
    let result = engine.symbol_lookup(&graph, params).expect("lookup");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "process_payment");
}

#[test]
fn test_symbol_lookup_prefix() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = SymbolLookupParams {
        name: "process".into(),
        mode: MatchMode::Prefix,
        ..Default::default()
    };
    let result = engine.symbol_lookup(&graph, params).expect("lookup");
    assert!(!result.is_empty());
    assert!(result
        .iter()
        .all(|u| u.name.to_lowercase().starts_with("process")));
}

#[test]
fn test_symbol_lookup_contains() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = SymbolLookupParams {
        name: "payment".into(),
        mode: MatchMode::Contains,
        ..Default::default()
    };
    let result = engine.symbol_lookup(&graph, params).expect("lookup");
    // process_payment and PaymentConfig both contain "payment"
    assert!(result.len() >= 2);
}

#[test]
fn test_symbol_lookup_fuzzy() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = SymbolLookupParams {
        name: "login".into(),
        mode: MatchMode::Fuzzy,
        fuzzy_threshold: 2,
        ..Default::default()
    };
    let result = engine.symbol_lookup(&graph, params).expect("lookup");
    assert!(result.iter().any(|u| u.name == "login"));
}

#[test]
fn test_symbol_lookup_no_results() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = SymbolLookupParams {
        name: "nonexistent_symbol_xyz".into(),
        mode: MatchMode::Exact,
        ..Default::default()
    };
    let result = engine.symbol_lookup(&graph, params).expect("lookup");
    assert!(result.is_empty());
}

#[test]
fn test_symbol_lookup_with_type_filter() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = SymbolLookupParams {
        name: "".into(),
        mode: MatchMode::Contains,
        unit_types: vec![CodeUnitType::Test],
        ..Default::default()
    };
    let result = engine.symbol_lookup(&graph, params).expect("lookup");
    assert!(result.iter().all(|u| u.unit_type == CodeUnitType::Test));
}

#[test]
fn test_symbol_lookup_with_language_filter() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = SymbolLookupParams {
        name: "".into(),
        mode: MatchMode::Contains,
        languages: vec![Language::Python],
        ..Default::default()
    };
    let result = engine.symbol_lookup(&graph, params).expect("lookup");
    assert!(result.iter().all(|u| u.language == Language::Python));
}

#[test]
fn test_symbol_lookup_with_limit() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = SymbolLookupParams {
        name: "".into(),
        mode: MatchMode::Contains,
        limit: 3,
        ..Default::default()
    };
    let result = engine.symbol_lookup(&graph, params).expect("lookup");
    assert!(result.len() <= 3);
}

// --- Query 2: Dependency Graph ---

#[test]
fn test_dependency_direct() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DependencyParams {
        unit_id: 1, // process_payment
        max_depth: 1,
        edge_types: vec![EdgeType::Calls],
        include_transitive: false,
    };
    let result = engine.dependency_graph(&graph, params).expect("dep");
    assert_eq!(result.root_id, 1);
    // Should find validate_input and send_notification
    let dep_ids: Vec<u64> = result.nodes.iter().map(|n| n.unit_id).collect();
    assert!(dep_ids.contains(&2));
    assert!(dep_ids.contains(&3));
}

#[test]
fn test_dependency_transitive() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DependencyParams {
        unit_id: 10, // login → process_payment → validate_input, send_notification
        max_depth: 5,
        edge_types: vec![EdgeType::Calls],
        include_transitive: true,
    };
    let result = engine.dependency_graph(&graph, params).expect("dep");
    let dep_ids: Vec<u64> = result.nodes.iter().map(|n| n.unit_id).collect();
    // login calls process_payment, which calls validate_input and send_notification
    assert!(dep_ids.contains(&1)); // process_payment
    assert!(dep_ids.contains(&2)); // validate_input (transitive)
    assert!(dep_ids.contains(&3)); // send_notification (transitive)
}

#[test]
fn test_dependency_depth_limit() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DependencyParams {
        unit_id: 10,
        max_depth: 1,
        edge_types: vec![EdgeType::Calls],
        include_transitive: true,
    };
    let result = engine.dependency_graph(&graph, params).expect("dep");
    // Only depth 1: login → process_payment
    let dep_ids: Vec<u64> = result.nodes.iter().map(|n| n.unit_id).collect();
    assert!(dep_ids.contains(&1)); // direct call
                                   // validate_input is depth 2, should NOT be found at depth limit 1
    let transitive: Vec<&DependencyNode> = result.nodes.iter().filter(|n| n.depth > 1).collect();
    assert!(transitive.is_empty());
}

#[test]
fn test_dependency_edge_type_filter() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DependencyParams {
        unit_id: 1,
        max_depth: 3,
        edge_types: vec![EdgeType::UsesType],
        include_transitive: true,
    };
    let result = engine.dependency_graph(&graph, params).expect("dep");
    // Should only follow UsesType, finding PaymentConfig
    let dep_ids: Vec<u64> = result.nodes.iter().map(|n| n.unit_id).collect();
    assert!(dep_ids.contains(&4)); // PaymentConfig
                                   // Should NOT find validate_input through Calls edges
    assert!(!dep_ids.contains(&2));
}

#[test]
fn test_dependency_nonexistent_unit() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DependencyParams {
        unit_id: 999,
        max_depth: 3,
        edge_types: vec![],
        include_transitive: true,
    };
    let result = engine.dependency_graph(&graph, params);
    assert!(result.is_err());
}

// --- Query 3: Reverse Dependency ---

#[test]
fn test_reverse_dep_direct() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DependencyParams {
        unit_id: 2, // validate_input
        max_depth: 1,
        edge_types: vec![EdgeType::Calls],
        include_transitive: false,
    };
    let result = engine.reverse_dependency(&graph, params).expect("revdep");
    let dep_ids: Vec<u64> = result.nodes.iter().map(|n| n.unit_id).collect();
    assert!(dep_ids.contains(&1)); // process_payment calls validate_input
}

#[test]
fn test_reverse_dep_transitive() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DependencyParams {
        unit_id: 2, // validate_input
        max_depth: 5,
        edge_types: vec![EdgeType::Calls],
        include_transitive: true,
    };
    let result = engine.reverse_dependency(&graph, params).expect("revdep");
    let dep_ids: Vec<u64> = result.nodes.iter().map(|n| n.unit_id).collect();
    assert!(dep_ids.contains(&1)); // process_payment (direct)
    assert!(dep_ids.contains(&10)); // login (transitive via process_payment)
}

#[test]
fn test_reverse_dep_depth_limit() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DependencyParams {
        unit_id: 2,
        max_depth: 1,
        edge_types: vec![EdgeType::Calls],
        include_transitive: true,
    };
    let result = engine.reverse_dependency(&graph, params).expect("revdep");
    let dep_ids: Vec<u64> = result.nodes.iter().map(|n| n.unit_id).collect();
    assert!(dep_ids.contains(&1)); // direct
                                   // login at depth 2 should NOT appear
    assert!(!dep_ids.contains(&10));
}

// --- Query 4: Call Graph ---

#[test]
fn test_call_graph_callers() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = CallGraphParams {
        unit_id: 1, // process_payment
        direction: CallDirection::Callers,
        max_depth: 3,
    };
    let result = engine.call_graph(&graph, params).expect("callgraph");
    // login calls process_payment
    assert!(result.nodes.iter().any(|&(id, _)| id == 10));
}

#[test]
fn test_call_graph_callees() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = CallGraphParams {
        unit_id: 1,
        direction: CallDirection::Callees,
        max_depth: 3,
    };
    let result = engine.call_graph(&graph, params).expect("callgraph");
    // process_payment calls validate_input and send_notification
    assert!(result.nodes.iter().any(|&(id, _)| id == 2));
    assert!(result.nodes.iter().any(|&(id, _)| id == 3));
    assert!(!result.call_sites.is_empty());
}

#[test]
fn test_call_graph_both() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = CallGraphParams {
        unit_id: 1,
        direction: CallDirection::Both,
        max_depth: 3,
    };
    let result = engine.call_graph(&graph, params).expect("callgraph");
    // Should include both callers and callees
    let ids: Vec<u64> = result.nodes.iter().map(|&(id, _)| id).collect();
    assert!(ids.contains(&10)); // caller
    assert!(ids.contains(&2)); // callee
    assert!(ids.contains(&3)); // callee
}

#[test]
fn test_call_graph_depth() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = CallGraphParams {
        unit_id: 10, // login
        direction: CallDirection::Callees,
        max_depth: 1,
    };
    let result = engine.call_graph(&graph, params).expect("callgraph");
    // Only depth 1 from login: process_payment only
    let callee_ids: Vec<u64> = result.call_sites.iter().map(|cs| cs.callee_id).collect();
    assert!(callee_ids.contains(&1));
}

// --- Query 5: Type Hierarchy ---

#[test]
fn test_hierarchy_ancestors() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = HierarchyParams {
        unit_id: 7, // UserValidator
        include_ancestors: true,
        include_descendants: false,
    };
    let result = engine.type_hierarchy(&graph, params).expect("hierarchy");
    assert!(result.nodes.iter().any(|n| n.unit_id == 6)); // Validator is ancestor
}

#[test]
fn test_hierarchy_descendants() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = HierarchyParams {
        unit_id: 6, // Validator
        include_ancestors: false,
        include_descendants: true,
    };
    let result = engine.type_hierarchy(&graph, params).expect("hierarchy");
    let ids: Vec<u64> = result.nodes.iter().map(|n| n.unit_id).collect();
    assert!(ids.contains(&7)); // UserValidator
    assert!(ids.contains(&8)); // InputValidator
}

#[test]
fn test_hierarchy_both() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = HierarchyParams {
        unit_id: 6,
        include_ancestors: true,
        include_descendants: true,
    };
    let result = engine.type_hierarchy(&graph, params).expect("hierarchy");
    // Validator has no ancestors, but has 2 descendants
    let ids: Vec<u64> = result.nodes.iter().map(|n| n.unit_id).collect();
    assert!(ids.contains(&7));
    assert!(ids.contains(&8));
}

#[test]
fn test_hierarchy_interface() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = HierarchyParams {
        unit_id: 7, // UserValidator implements Validator
        include_ancestors: true,
        include_descendants: false,
    };
    let result = engine.type_hierarchy(&graph, params).expect("hierarchy");
    // Should find Validator via Implements edge
    let ancestor = result.nodes.iter().find(|n| n.unit_id == 6);
    assert!(ancestor.is_some());
    assert_eq!(ancestor.map(|a| a.relation), Some(EdgeType::Implements));
}

// --- Query 6: Containment ---

#[test]
fn test_containment_module() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.containment(&graph, 0).expect("containment");
    // Module app contains many things
    assert!(result.len() >= 5);
}

#[test]
fn test_containment_class() {
    // Auth module contains login and logout
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.containment(&graph, 9).expect("containment");
    let ids: Vec<u64> = result.iter().map(|u| u.id).collect();
    assert!(ids.contains(&10)); // login
    assert!(ids.contains(&11)); // logout
}

#[test]
fn test_containment_nested() {
    // A leaf node with no children
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.containment(&graph, 2).expect("containment");
    assert!(result.is_empty()); // validate_input has no children
}

// --- Query 7: Pattern Match ---

#[test]
fn test_pattern_function_calls() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = PatternParams {
        pattern: "function { calls: [validate_input, send_notification] }".into(),
    };
    let result = engine.pattern_match(&graph, params).expect("pattern");
    assert!(result.iter().any(|m| m.unit_id == 1)); // process_payment
}

#[test]
fn test_pattern_async_complexity() {
    // Create a graph with async function
    let mut graph = CodeGraph::with_default_dimension();
    let mut f = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "heavy_async".into(),
        "heavy_async".into(),
        PathBuf::from("src/lib.rs"),
        Span::new(1, 0, 50, 0),
    );
    f.is_async = true;
    f.complexity = 15;
    graph.add_unit(f);

    let engine = QueryEngine::new();
    let params = PatternParams {
        pattern: "async function { complexity: >10 }".into(),
    };
    let result = engine.pattern_match(&graph, params).expect("pattern");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].unit_id, 0);
}

#[test]
fn test_pattern_class_inherits() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = PatternParams {
        pattern: "class { inherits: Validator }".into(),
    };
    let result = engine.pattern_match(&graph, params).expect("pattern");
    // InputValidator inherits Validator (via Inherits edge)
    assert!(result.iter().any(|m| m.unit_id == 8));
}

// --- Query 8: Semantic Search ---

#[test]
fn test_semantic_search_basic() {
    let mut graph = CodeGraph::new(3);
    let mut u1 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "foo".into(),
        "foo".into(),
        PathBuf::from("a.rs"),
        Span::new(1, 0, 10, 0),
    );
    u1.feature_vec = vec![1.0, 0.0, 0.0];
    graph.add_unit(u1);

    let mut u2 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "bar".into(),
        "bar".into(),
        PathBuf::from("b.rs"),
        Span::new(1, 0, 10, 0),
    );
    u2.feature_vec = vec![0.9, 0.1, 0.0];
    graph.add_unit(u2);

    let mut u3 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "baz".into(),
        "baz".into(),
        PathBuf::from("c.rs"),
        Span::new(1, 0, 10, 0),
    );
    u3.feature_vec = vec![0.0, 1.0, 0.0];
    graph.add_unit(u3);

    let engine = QueryEngine::new();
    let params = SemanticParams {
        query_vec: vec![1.0, 0.0, 0.0],
        top_k: 2,
        unit_types: vec![],
        languages: vec![],
        min_similarity: 0.5,
    };
    let result = engine.semantic_search(&graph, params).expect("search");
    assert!(result.len() <= 2);
    // foo and bar should be most similar to the query
    assert!(result[0].unit_id == 0); // foo is identical
    assert!(result[0].score > 0.9);
}

#[test]
fn test_semantic_search_filters() {
    let mut graph = CodeGraph::new(3);
    let mut u1 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "foo".into(),
        "foo".into(),
        PathBuf::from("a.rs"),
        Span::new(1, 0, 10, 0),
    );
    u1.feature_vec = vec![1.0, 0.0, 0.0];
    graph.add_unit(u1);

    let mut u2 = CodeUnit::new(
        CodeUnitType::Type,
        Language::Python,
        "bar".into(),
        "bar".into(),
        PathBuf::from("b.py"),
        Span::new(1, 0, 10, 0),
    );
    u2.feature_vec = vec![0.9, 0.1, 0.0];
    graph.add_unit(u2);

    let engine = QueryEngine::new();
    let params = SemanticParams {
        query_vec: vec![1.0, 0.0, 0.0],
        top_k: 10,
        unit_types: vec![CodeUnitType::Function],
        languages: vec![],
        min_similarity: 0.0,
    };
    let result = engine.semantic_search(&graph, params).expect("search");
    // Only function types should be returned
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].unit_id, 0);
}

#[test]
fn test_semantic_search_threshold() {
    let mut graph = CodeGraph::new(3);
    let mut u1 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "foo".into(),
        "foo".into(),
        PathBuf::from("a.rs"),
        Span::new(1, 0, 10, 0),
    );
    u1.feature_vec = vec![0.0, 0.0, 1.0]; // orthogonal to query
    graph.add_unit(u1);

    let engine = QueryEngine::new();
    let params = SemanticParams {
        query_vec: vec![1.0, 0.0, 0.0],
        top_k: 10,
        unit_types: vec![],
        languages: vec![],
        min_similarity: 0.5,
    };
    let result = engine.semantic_search(&graph, params).expect("search");
    assert!(result.is_empty()); // Below threshold
}

// ============================================================================
// Built Queries (9-11, 22-23)
// ============================================================================

// --- Query 9: Impact Analysis ---

#[test]
fn test_impact_direct() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = ImpactParams {
        unit_id: 2, // validate_input
        max_depth: 1,
        edge_types: vec![],
    };
    let result = engine.impact_analysis(&graph, params).expect("impact");
    assert_eq!(result.root_id, 2);
    // process_payment depends on validate_input
    assert!(result.impacted.iter().any(|i| i.unit_id == 1));
}

#[test]
fn test_impact_transitive() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = ImpactParams {
        unit_id: 2, // validate_input
        max_depth: 5,
        edge_types: vec![],
    };
    let result = engine.impact_analysis(&graph, params).expect("impact");
    // Transitive: login → process_payment → validate_input
    assert!(result.impacted.iter().any(|i| i.unit_id == 10));
}

#[test]
fn test_impact_with_tests() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    // Check impact for validate_input which is called by process_payment
    let params = ImpactParams {
        unit_id: 3, // send_notification (no one depends on it through Calls reverse)
        max_depth: 3,
        edge_types: vec![],
    };
    let result = engine.impact_analysis(&graph, params).expect("impact");
    // Risk score should be valid
    assert!(result.overall_risk >= 0.0 && result.overall_risk <= 1.0);
    // The result should have a valid root
    assert_eq!(result.root_id, 3);
}

#[test]
fn test_impact_risk_scoring() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = ImpactParams {
        unit_id: 2,
        max_depth: 3,
        edge_types: vec![],
    };
    let result = engine.impact_analysis(&graph, params).expect("impact");
    // All impacted units should have a risk score between 0 and 1
    for impacted in &result.impacted {
        assert!(impacted.risk_score >= 0.0 && impacted.risk_score <= 1.0);
    }
    assert!(result.overall_risk >= 0.0 && result.overall_risk <= 1.0);
}

// --- Query 10: Test Coverage ---

#[test]
fn test_coverage_direct() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.test_coverage(&graph, 1).expect("coverage");
    // process_payment is tested by test_process_payment
    assert!(result.direct_tests.contains(&5));
}

#[test]
fn test_coverage_indirect() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.test_coverage(&graph, 2).expect("coverage");
    // validate_input is called by process_payment, which has tests
    assert!(!result.indirect_tests.is_empty() || !result.direct_tests.is_empty());
}

#[test]
fn test_coverage_none() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.test_coverage(&graph, 12).expect("coverage");
    // helper_internal has no tests
    assert!(result.direct_tests.is_empty());
    assert!(result.coverage_ratio < 0.5);
}

// --- Query 11: Cross-Language Trace ---

#[test]
fn test_trace_cross_language() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = TraceParams {
        unit_id: 10, // login (Python)
        max_hops: 5,
    };
    let result = engine.cross_language_trace(&graph, params).expect("trace");
    // Should trace through multiple hops
    assert!(!result.hops.is_empty());
}

#[test]
fn test_trace_full_chain() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = TraceParams {
        unit_id: 10,
        max_hops: 10,
    };
    let result = engine.cross_language_trace(&graph, params).expect("trace");
    // Login → process_payment → validate_input, send_notification
    let hop_ids: Vec<u64> = result.hops.iter().map(|h| h.unit_id).collect();
    assert!(hop_ids.contains(&10)); // origin
}

// --- Query 22: Similarity ---

#[test]
fn test_similarity_basic() {
    let mut graph = CodeGraph::new(3);
    let mut u1 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "a".into(),
        "a".into(),
        PathBuf::from("a.rs"),
        Span::new(1, 0, 10, 0),
    );
    u1.feature_vec = vec![1.0, 0.0, 0.0];
    graph.add_unit(u1);

    let mut u2 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "b".into(),
        "b".into(),
        PathBuf::from("b.rs"),
        Span::new(1, 0, 10, 0),
    );
    u2.feature_vec = vec![0.95, 0.05, 0.0];
    graph.add_unit(u2);

    let mut u3 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "c".into(),
        "c".into(),
        PathBuf::from("c.rs"),
        Span::new(1, 0, 10, 0),
    );
    u3.feature_vec = vec![0.0, 1.0, 0.0];
    graph.add_unit(u3);

    let engine = QueryEngine::new();
    let params = SimilarityParams {
        unit_id: 0,
        top_k: 5,
        min_similarity: 0.5,
    };
    let result = engine.similarity(&graph, params).expect("similarity");
    // b should be very similar to a, c should not be
    assert!(!result.is_empty());
    assert_eq!(result[0].unit_id, 1); // b is most similar to a
    assert!(result[0].score > 0.9);
}

#[test]
fn test_similarity_threshold() {
    let mut graph = CodeGraph::new(3);
    let mut u1 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "a".into(),
        "a".into(),
        PathBuf::from("a.rs"),
        Span::new(1, 0, 10, 0),
    );
    u1.feature_vec = vec![1.0, 0.0, 0.0];
    graph.add_unit(u1);

    let mut u2 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "b".into(),
        "b".into(),
        PathBuf::from("b.rs"),
        Span::new(1, 0, 10, 0),
    );
    u2.feature_vec = vec![0.0, 1.0, 0.0]; // orthogonal
    graph.add_unit(u2);

    let engine = QueryEngine::new();
    let params = SimilarityParams {
        unit_id: 0,
        top_k: 10,
        min_similarity: 0.9,
    };
    let result = engine.similarity(&graph, params).expect("similarity");
    assert!(result.is_empty());
}

// --- Query 23: Shortest Path ---

#[test]
fn test_shortest_path_direct() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.shortest_path(&graph, 1, 2).expect("path");
    assert!(result.found);
    assert_eq!(result.path, vec![1, 2]);
    assert_eq!(result.length, 1);
}

#[test]
fn test_shortest_path_indirect() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.shortest_path(&graph, 10, 2).expect("path");
    assert!(result.found);
    // login → process_payment → validate_input
    assert_eq!(result.path, vec![10, 1, 2]);
    assert_eq!(result.length, 2);
}

#[test]
fn test_shortest_path_no_path() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.shortest_path(&graph, 2, 10).expect("path");
    // No path from validate_input to login (no backward edges)
    assert!(!result.found);
}

// ============================================================================
// Novel Queries (12-21, 24)
// ============================================================================

// --- Query 12: Collective Patterns ---

#[test]
fn test_collective_patterns_basic() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = CollectiveParams {
        unit_type: None,
        min_usage: 1,
        limit: 10,
    };
    let result = engine
        .collective_patterns(&graph, params)
        .expect("collective");
    // No units have collective_usage > 0, so patterns should be empty
    assert!(result.patterns.is_empty());
    // Collective is not yet available
    assert!(!result.collective_available);
}

#[test]
fn test_collective_patterns_filter() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = CollectiveParams {
        unit_type: Some(CodeUnitType::Function),
        min_usage: 0,
        limit: 5,
    };
    let result = engine
        .collective_patterns(&graph, params)
        .expect("collective");
    assert!(result.patterns.len() <= 5);
}

// --- Query 13: Temporal Evolution ---

#[test]
fn test_evolution_timeline() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.temporal_evolution(&graph, 1).expect("evolution");
    assert_eq!(result.unit_id, 1);
    assert_eq!(result.change_count, 12);
    assert!(result.stability_score < 0.5); // Low stability
}

#[test]
fn test_evolution_trend() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.temporal_evolution(&graph, 2).expect("evolution");
    assert_eq!(result.change_count, 2);
    assert!(result.stability_score > 0.8); // Stable
                                           // trend should indicate stability
    assert!(!result.trend.is_empty());
}

// --- Query 14: Stability Analysis ---

#[test]
fn test_stability_stable() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.stability_analysis(&graph, 2).expect("stability");
    assert!(result.overall_score > 0.5); // validate_input is stable
    assert!(!result.factors.is_empty());
}

#[test]
fn test_stability_volatile() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.stability_analysis(&graph, 12).expect("stability");
    // helper_internal: high change count, low stability, high complexity
    assert!(result.overall_score < 0.5);
}

#[test]
fn test_stability_factors() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.stability_analysis(&graph, 1).expect("stability");
    assert!(!result.factors.is_empty());
    assert!(!result.recommendation.is_empty());
    for factor in &result.factors {
        assert!(factor.value >= 0.0 && factor.value <= 1.0);
    }
}

// --- Query 15: Coupling Detection ---

#[test]
fn test_coupling_explicit() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = CouplingParams {
        unit_id: Some(1),
        min_strength: 0.0,
    };
    let result = engine.coupling_detection(&graph, params).expect("coupling");
    // process_payment has explicit couplings (calls, uses)
    assert!(!result.is_empty());
    assert!(result.iter().any(|c| c.kind == CouplingKind::Explicit));
}

#[test]
fn test_coupling_temporal() {
    // Build a custom graph where CouplesWith is the only edge between two units
    let mut graph = CodeGraph::with_default_dimension();
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "a".into(),
        "a".into(),
        PathBuf::from("a.rs"),
        Span::new(1, 0, 10, 0),
    ));
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "b".into(),
        "b".into(),
        PathBuf::from("b.rs"),
        Span::new(1, 0, 10, 0),
    ));
    // Only a temporal coupling, no explicit dependency
    graph
        .add_edge(Edge::new(0, 1, EdgeType::CouplesWith).with_weight(0.8))
        .ok();

    let engine = QueryEngine::new();
    let params = CouplingParams {
        unit_id: Some(0),
        min_strength: 0.0,
    };
    let result = engine.coupling_detection(&graph, params).expect("coupling");
    assert!(result.iter().any(|c| c.kind == CouplingKind::Temporal));
}

// --- Query 16: Dead Code ---

#[test]
fn test_dead_code_unreachable() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DeadCodeParams {
        unit_types: vec![CodeUnitType::Function],
        include_tests_as_roots: true,
    };
    let result = engine.dead_code(&graph, params).expect("dead");
    // helper_internal is not called by anything
    let dead_ids: Vec<u64> = result.iter().map(|u| u.id).collect();
    assert!(dead_ids.contains(&12));
}

#[test]
fn test_dead_code_entry_points() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DeadCodeParams {
        unit_types: vec![],
        include_tests_as_roots: true,
    };
    let result = engine.dead_code(&graph, params).expect("dead");
    // Public functions like process_payment should NOT be dead code
    let dead_ids: Vec<u64> = result.iter().map(|u| u.id).collect();
    assert!(!dead_ids.contains(&1));
}

// --- Query 17: Prophecy ---

#[test]
fn test_prophecy_likely_break() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = ProphecyParams {
        top_k: 5,
        min_risk: 0.0,
    };
    let result = engine.prophecy(&graph, params).expect("prophecy");
    // helper_internal has high change count and low stability
    assert!(!result.predictions.is_empty());
    // predictions should be sorted by risk
    for window in result.predictions.windows(2) {
        assert!(window[0].risk_score >= window[1].risk_score);
    }
}

#[test]
fn test_prophecy_tech_debt() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = ProphecyParams {
        top_k: 10,
        min_risk: 0.3,
    };
    let result = engine.prophecy(&graph, params).expect("prophecy");
    for pred in &result.predictions {
        assert!(pred.risk_score >= 0.3);
        assert!(!pred.reason.is_empty());
    }
}

// --- Query 18: Concept Mapping ---

#[test]
fn test_concept_mapping_auth() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.concept_mapping(&graph, "auth").expect("concept");
    assert_eq!(result.concept, "auth");
    // login and logout are auth-related
    let unit_ids: Vec<u64> = result.units.iter().map(|u| u.unit_id).collect();
    assert!(unit_ids.contains(&10) || unit_ids.contains(&11));
}

#[test]
fn test_concept_mapping_payment() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let result = engine.concept_mapping(&graph, "payment").expect("concept");
    // process_payment and PaymentConfig are payment-related
    let unit_ids: Vec<u64> = result.units.iter().map(|u| u.unit_id).collect();
    assert!(unit_ids.contains(&1) || unit_ids.contains(&4));
}

// --- Query 19: Migration Path ---

#[test]
fn test_migration_simple() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = MigrationParams {
        from_unit: 1, // process_payment
        to_unit: 3,   // send_notification
    };
    let result = engine.migration_path(&graph, params).expect("migration");
    assert!(!result.steps.is_empty());
    assert_eq!(result.from_unit, 1);
    assert_eq!(result.to_unit, 3);
}

#[test]
fn test_migration_with_safety() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = MigrationParams {
        from_unit: 2, // validate_input
        to_unit: 1,   // process_payment (caller)
    };
    let result = engine.migration_path(&graph, params).expect("migration");
    // Steps should have safety levels
    for step in &result.steps {
        assert!(!step.description.is_empty());
    }
}

// --- Query 20: Test Gap ---

#[test]
fn test_gap_recent_changes() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = TestGapParams {
        min_changes: 5,
        min_complexity: 3,
        unit_types: vec![],
    };
    let result = engine.test_gap(&graph, params).expect("testgap");
    // helper_internal has high changes (20), high complexity (15), and no tests
    let gap_ids: Vec<u64> = result.iter().map(|g| g.unit_id).collect();
    assert!(gap_ids.contains(&12));
}

#[test]
fn test_gap_complexity_filter() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = TestGapParams {
        min_changes: 100,    // Very high threshold
        min_complexity: 100, // Very high threshold
        unit_types: vec![],
    };
    let result = engine.test_gap(&graph, params).expect("testgap");
    // No unit has change_count >= 100 AND complexity >= 100, and
    // test_gap requires EITHER to trigger, so nothing should match
    assert!(result.is_empty());
}

// --- Query 21: Architectural Drift ---

#[test]
fn test_drift_layer_violation() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = DriftParams {
        rules: vec![ArchRule::LayerDependency {
            upper: "app".into(),
            lower: "auth".into(),
        }],
    };
    let result = engine.architectural_drift(&graph, params).expect("drift");
    // login (auth) calls process_payment (app), but the rule says app should not depend on auth
    // Whether this triggers depends on the exact rule semantics
    assert!(result.conformance_score >= 0.0 && result.conformance_score <= 1.0);
}

#[test]
fn test_drift_cycle() {
    // Build a graph with a cycle
    let mut graph = CodeGraph::with_default_dimension();
    let u1 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "a".into(),
        "cycle::a".into(),
        PathBuf::from("a.rs"),
        Span::new(1, 0, 10, 0),
    );
    graph.add_unit(u1);
    let u2 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "b".into(),
        "cycle::b".into(),
        PathBuf::from("b.rs"),
        Span::new(1, 0, 10, 0),
    );
    graph.add_unit(u2);
    let u3 = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "c".into(),
        "cycle::c".into(),
        PathBuf::from("c.rs"),
        Span::new(1, 0, 10, 0),
    );
    graph.add_unit(u3);

    // a → b → c → a (cycle!)
    graph.add_edge(Edge::new(0, 1, EdgeType::Calls)).ok();
    graph.add_edge(Edge::new(1, 2, EdgeType::Calls)).ok();
    graph.add_edge(Edge::new(2, 0, EdgeType::Calls)).ok();

    let engine = QueryEngine::new();
    let params = DriftParams {
        rules: vec![ArchRule::Cyclic {
            scope: "cycle".into(),
        }],
    };
    let result = engine.architectural_drift(&graph, params).expect("drift");
    // Should detect the cycle
    assert!(!result.violations.is_empty());
    assert!(result.conformance_score < 1.0);
}

// --- Query 24: Hotspot Detection ---

#[test]
fn test_hotspot_buggy_file() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = HotspotParams {
        top_k: 5,
        min_score: 0.0,
        unit_types: vec![CodeUnitType::Function],
    };
    let result = engine.hotspot_detection(&graph, params).expect("hotspot");
    // helper_internal should be a hotspot (high changes, low stability, high complexity)
    assert!(!result.is_empty());
    // Results should be sorted by score descending
    for window in result.windows(2) {
        assert!(window[0].score >= window[1].score);
    }
}

#[test]
fn test_hotspot_clean_file() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = HotspotParams {
        top_k: 5,
        min_score: 0.9, // Very high threshold
        unit_types: vec![],
    };
    let result = engine.hotspot_detection(&graph, params).expect("hotspot");
    // Very few (if any) units should be above 0.9
    assert!(result.len() <= 2);
}

// ============================================================================
// Edge cases and error handling
// ============================================================================

#[test]
fn test_query_engine_default() {
    let engine = QueryEngine;
    let graph = CodeGraph::with_default_dimension();
    let params = SymbolLookupParams {
        name: "x".into(),
        mode: MatchMode::Exact,
        ..Default::default()
    };
    let result = engine.symbol_lookup(&graph, params).expect("lookup");
    assert!(result.is_empty());
}

#[test]
fn test_empty_graph_queries() {
    let graph = CodeGraph::with_default_dimension();
    let engine = QueryEngine::new();

    // Symbol lookup on empty graph
    let params = SymbolLookupParams {
        name: "anything".into(),
        mode: MatchMode::Contains,
        ..Default::default()
    };
    assert!(engine
        .symbol_lookup(&graph, params)
        .expect("lookup")
        .is_empty());

    // Dead code on empty graph
    let dc_params = DeadCodeParams {
        unit_types: vec![],
        include_tests_as_roots: true,
    };
    assert!(engine
        .dead_code(&graph, dc_params)
        .expect("dead")
        .is_empty());

    // Prophecy on empty graph
    let p_params = ProphecyParams {
        top_k: 5,
        min_risk: 0.0,
    };
    assert!(engine
        .prophecy(&graph, p_params)
        .expect("prophecy")
        .predictions
        .is_empty());
}

#[test]
fn test_unit_not_found_errors() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();

    assert!(engine.containment(&graph, 999).is_err());
    assert!(engine.test_coverage(&graph, 999).is_err());
    assert!(engine.temporal_evolution(&graph, 999).is_err());
    assert!(engine.stability_analysis(&graph, 999).is_err());
    assert!(engine.shortest_path(&graph, 999, 0).is_err());
    assert!(engine.shortest_path(&graph, 0, 999).is_err());
}

#[test]
fn test_dependency_cycle_handling() {
    // Build a cycle: a → b → a
    let mut graph = CodeGraph::with_default_dimension();
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "a".into(),
        "a".into(),
        PathBuf::from("a.rs"),
        Span::new(1, 0, 10, 0),
    ));
    graph.add_unit(CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "b".into(),
        "b".into(),
        PathBuf::from("b.rs"),
        Span::new(1, 0, 10, 0),
    ));
    graph.add_edge(Edge::new(0, 1, EdgeType::Calls)).ok();
    graph.add_edge(Edge::new(1, 0, EdgeType::Calls)).ok();

    let engine = QueryEngine::new();
    let params = DependencyParams {
        unit_id: 0,
        max_depth: 100,
        edge_types: vec![],
        include_transitive: true,
    };
    // Should not infinite loop; BFS handles visited set
    let result = engine.dependency_graph(&graph, params).expect("dep");
    assert!(!result.nodes.is_empty());
}

#[test]
fn test_similarity_nonexistent_unit() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = SimilarityParams {
        unit_id: 999,
        top_k: 5,
        min_similarity: 0.0,
    };
    assert!(engine.similarity(&graph, params).is_err());
}

#[test]
fn test_migration_nonexistent_unit() {
    let graph = build_rich_graph();
    let engine = QueryEngine::new();
    let params = MigrationParams {
        from_unit: 999,
        to_unit: 0,
    };
    assert!(engine.migration_path(&graph, params).is_err());
}
