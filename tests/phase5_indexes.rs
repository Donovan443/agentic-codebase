//! Phase 5 integration tests: Indexes & mmap.
//!
//! Tests all 5 index types plus memory-mapped file I/O.

use std::path::{Path, PathBuf};

use agentic_codebase::graph::CodeGraph;
use agentic_codebase::index::embedding_index::EmbeddingIndex;
use agentic_codebase::index::language_index::LanguageIndex;
use agentic_codebase::index::path_index::PathIndex;
use agentic_codebase::index::symbol_index::SymbolIndex;
use agentic_codebase::index::type_index::TypeIndex;
use agentic_codebase::types::{CodeUnit, CodeUnitType, Edge, EdgeType, Language, Span};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_unit(name: &str, unit_type: CodeUnitType, language: Language, file: &str) -> CodeUnit {
    CodeUnit::new(
        unit_type,
        language,
        name.to_string(),
        format!("mod::{name}"),
        PathBuf::from(file),
        Span::new(1, 0, 10, 0),
    )
}

/// Build a graph with diverse units for index testing.
fn build_index_graph() -> CodeGraph {
    let mut graph = CodeGraph::new(4);

    // 0: Rust function in src/lib.rs
    graph.add_unit(make_unit(
        "process_payment",
        CodeUnitType::Function,
        Language::Rust,
        "src/lib.rs",
    ));
    // 1: Rust function in src/lib.rs
    graph.add_unit(make_unit(
        "validate_input",
        CodeUnitType::Function,
        Language::Rust,
        "src/lib.rs",
    ));
    // 2: Rust module in src/mod.rs
    graph.add_unit(make_unit(
        "payments",
        CodeUnitType::Module,
        Language::Rust,
        "src/mod.rs",
    ));
    // 3: Python function in src/auth.py
    graph.add_unit(make_unit(
        "process_refund",
        CodeUnitType::Function,
        Language::Python,
        "src/auth.py",
    ));
    // 4: TypeScript type in src/types.ts
    graph.add_unit(make_unit(
        "PaymentConfig",
        CodeUnitType::Type,
        Language::TypeScript,
        "src/types.ts",
    ));
    // 5: Rust trait in src/lib.rs
    graph.add_unit(make_unit(
        "Processor",
        CodeUnitType::Trait,
        Language::Rust,
        "src/lib.rs",
    ));
    // 6: Rust test in tests/test.rs
    graph.add_unit(make_unit(
        "test_process_payment",
        CodeUnitType::Test,
        Language::Rust,
        "tests/test.rs",
    ));

    // Add some edges
    let _ = graph.add_edge(Edge {
        source_id: 0,
        target_id: 1,
        edge_type: EdgeType::Calls,
        weight: 1.0,
        created_at: 0,
        context: 0,
    });
    let _ = graph.add_edge(Edge {
        source_id: 6,
        target_id: 0,
        edge_type: EdgeType::Tests,
        weight: 1.0,
        created_at: 0,
        context: 0,
    });

    graph
}

// ===========================================================================
// Symbol Index Tests
// ===========================================================================

#[test]
fn test_symbol_index_build() {
    let graph = build_index_graph();
    let index = SymbolIndex::build(&graph);
    assert_eq!(index.len(), 7); // 7 distinct names
    assert!(!index.is_empty());
}

#[test]
fn test_symbol_index_lookup() {
    let graph = build_index_graph();
    let index = SymbolIndex::build(&graph);

    // Exact lookups
    assert_eq!(index.lookup_exact("process_payment"), &[0]);
    assert_eq!(index.lookup_exact("validate_input"), &[1]);
    assert_eq!(index.lookup_exact("PaymentConfig"), &[4]);
    assert_eq!(index.lookup_exact("nonexistent"), &[] as &[u64]);

    // Case-sensitive: "PROCESS_PAYMENT" should not match exact
    assert_eq!(index.lookup_exact("PROCESS_PAYMENT"), &[] as &[u64]);
}

#[test]
fn test_symbol_index_prefix() {
    let graph = build_index_graph();
    let index = SymbolIndex::build(&graph);

    // "process" prefix should match process_payment (0), process_refund (3), processor (5)
    let mut results = index.lookup_prefix("process");
    results.sort();
    assert_eq!(results, vec![0, 3, 5]);

    // "pay" prefix should match payments (2) and paymentconfig (4, lowercased)
    let mut results = index.lookup_prefix("pay");
    results.sort();
    assert_eq!(results, vec![2, 4]);

    // Case-insensitive prefix (matches processor too)
    let mut results = index.lookup_prefix("PROCESS");
    results.sort();
    assert_eq!(results, vec![0, 3, 5]);

    // Empty prefix returns all
    let results = index.lookup_prefix("");
    assert_eq!(results.len(), 7);
}

#[test]
fn test_symbol_index_contains() {
    let graph = build_index_graph();
    let index = SymbolIndex::build(&graph);

    // "payment" contained in process_payment (0), payments (2),
    // paymentconfig (4), and test_process_payment (6)
    let mut results = index.lookup_contains("payment");
    results.sort();
    assert_eq!(results, vec![0, 2, 4, 6]);

    // "process" contained in process_payment (0), process_refund (3),
    // processor (5), and test_process_payment (6)
    let mut results = index.lookup_contains("process");
    results.sort();
    assert_eq!(results, vec![0, 3, 5, 6]);
}

#[test]
fn test_symbol_index_empty_graph() {
    let graph = CodeGraph::default();
    let index = SymbolIndex::build(&graph);
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
    assert_eq!(index.lookup_exact("anything"), &[] as &[u64]);
    assert!(index.lookup_prefix("any").is_empty());
    assert!(index.lookup_contains("any").is_empty());
}

// ===========================================================================
// Type Index Tests
// ===========================================================================

#[test]
fn test_type_index_build() {
    let graph = build_index_graph();
    let index = TypeIndex::build(&graph);

    // 3 functions (0,1,3), 1 module (2), 1 type (4), 1 trait (5), 1 test (6)
    assert_eq!(index.count(CodeUnitType::Function), 3);
    assert_eq!(index.count(CodeUnitType::Module), 1);
    assert_eq!(index.count(CodeUnitType::Type), 1);
    assert_eq!(index.count(CodeUnitType::Trait), 1);
    assert_eq!(index.count(CodeUnitType::Test), 1);
    assert_eq!(index.count(CodeUnitType::Import), 0);
}

#[test]
fn test_type_index_lookup() {
    let graph = build_index_graph();
    let index = TypeIndex::build(&graph);

    // 0=Function, 1=Function, 2=Module, 3=Function, 4=Type, 5=Trait, 6=Test
    let funcs = index.lookup(CodeUnitType::Function);
    assert_eq!(funcs, &[0, 1, 3]);

    assert_eq!(index.lookup(CodeUnitType::Module), &[2]);
    assert_eq!(index.lookup(CodeUnitType::Type), &[4]);
    assert_eq!(index.lookup(CodeUnitType::Trait), &[5]);
    assert_eq!(index.lookup(CodeUnitType::Test), &[6]);
}

#[test]
fn test_type_index_filter() {
    let graph = build_index_graph();
    let index = TypeIndex::build(&graph);

    let mut types = index.types();
    types.sort_by_key(|t| *t as u8);
    assert!(types.contains(&CodeUnitType::Function));
    assert!(types.contains(&CodeUnitType::Module));
    assert!(types.contains(&CodeUnitType::Type));
    assert!(types.contains(&CodeUnitType::Trait));
    assert!(types.contains(&CodeUnitType::Test));
    assert!(!types.contains(&CodeUnitType::Import));
}

// ===========================================================================
// Path Index Tests
// ===========================================================================

#[test]
fn test_path_index_build() {
    let graph = build_index_graph();
    let index = PathIndex::build(&graph);

    // 5 distinct files: src/lib.rs, src/mod.rs, src/auth.py, src/types.ts, tests/test.rs
    assert_eq!(index.file_count(), 5);
}

#[test]
fn test_path_index_lookup() {
    let graph = build_index_graph();
    let index = PathIndex::build(&graph);

    // src/lib.rs has units 0, 1, 5
    let mut lib_units = index.lookup(Path::new("src/lib.rs")).to_vec();
    lib_units.sort();
    assert_eq!(lib_units, vec![0, 1, 5]);

    // src/auth.py has unit 3
    assert_eq!(index.lookup(Path::new("src/auth.py")), &[3]);

    // nonexistent path
    assert_eq!(index.lookup(Path::new("src/nonexistent.rs")), &[] as &[u64]);
}

#[test]
fn test_path_index_paths_sorted() {
    let graph = build_index_graph();
    let index = PathIndex::build(&graph);

    let paths = index.paths();
    // Should be sorted alphabetically
    for window in paths.windows(2) {
        assert!(window[0] <= window[1], "Paths should be sorted");
    }
}

// ===========================================================================
// Language Index Tests
// ===========================================================================

#[test]
fn test_language_index_build() {
    let graph = build_index_graph();
    let index = LanguageIndex::build(&graph);

    // 5 Rust (0,1,2,5,6), 1 Python (3), 1 TypeScript (4)
    assert_eq!(index.count(Language::Rust), 5);
    assert_eq!(index.count(Language::Python), 1);
    assert_eq!(index.count(Language::TypeScript), 1);
    assert_eq!(index.count(Language::Go), 0);
}

#[test]
fn test_language_index_filter() {
    let graph = build_index_graph();
    let index = LanguageIndex::build(&graph);

    let mut rust_units = index.lookup(Language::Rust).to_vec();
    rust_units.sort();
    assert_eq!(rust_units, vec![0, 1, 2, 5, 6]);

    let py_units = index.lookup(Language::Python);
    assert_eq!(py_units, &[3]);

    let ts_units = index.lookup(Language::TypeScript);
    assert_eq!(ts_units, &[4]);

    // All languages present
    let mut langs = index.languages();
    langs.sort_by_key(|l| *l as u8);
    assert_eq!(langs.len(), 3);
}

// ===========================================================================
// Embedding Index Tests
// ===========================================================================

#[test]
fn test_embedding_index_build() {
    let dim = 4;
    let mut graph = CodeGraph::new(dim);

    let mut u0 = make_unit("fn_a", CodeUnitType::Function, Language::Rust, "src/lib.rs");
    u0.feature_vec = vec![1.0, 0.0, 0.0, 0.0];
    graph.add_unit(u0);

    let mut u1 = make_unit("fn_b", CodeUnitType::Function, Language::Rust, "src/lib.rs");
    u1.feature_vec = vec![0.0, 1.0, 0.0, 0.0];
    graph.add_unit(u1);

    // Zero-vector should be excluded
    let mut u2 = make_unit("fn_c", CodeUnitType::Function, Language::Rust, "src/lib.rs");
    u2.feature_vec = vec![0.0; dim];
    graph.add_unit(u2);

    let index = EmbeddingIndex::build(&graph);
    assert_eq!(index.len(), 2); // Only 2 non-zero vectors
    assert_eq!(index.dimension(), dim);
}

#[test]
fn test_embedding_index_search() {
    let dim = 4;
    let mut graph = CodeGraph::new(dim);

    let mut u0 = make_unit("fn_a", CodeUnitType::Function, Language::Rust, "src/lib.rs");
    u0.feature_vec = vec![1.0, 0.0, 0.0, 0.0];
    graph.add_unit(u0);

    let mut u1 = make_unit("fn_b", CodeUnitType::Function, Language::Rust, "src/lib.rs");
    u1.feature_vec = vec![0.9, 0.1, 0.0, 0.0];
    graph.add_unit(u1);

    let mut u2 = make_unit("fn_c", CodeUnitType::Function, Language::Rust, "src/lib.rs");
    u2.feature_vec = vec![0.0, 0.0, 1.0, 0.0];
    graph.add_unit(u2);

    let index = EmbeddingIndex::build(&graph);

    // Search for vector similar to u0
    let results = index.search(&[1.0, 0.0, 0.0, 0.0], 10, 0.0);
    assert_eq!(results.len(), 3);
    // First result should be exact match (score ~1.0)
    assert_eq!(results[0].unit_id, 0);
    assert!((results[0].score - 1.0).abs() < 1e-5);

    // Search with high min_similarity should filter
    let results = index.search(&[1.0, 0.0, 0.0, 0.0], 10, 0.9);
    assert!(results.len() <= 2); // u0 and u1 should pass, u2 is orthogonal
    assert!(results.iter().all(|r| r.score >= 0.9));

    // Top-k limiting
    let results = index.search(&[1.0, 0.0, 0.0, 0.0], 1, 0.0);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unit_id, 0);
}

#[test]
fn test_embedding_index_wrong_dimension() {
    let dim = 4;
    let mut graph = CodeGraph::new(dim);

    let mut u0 = make_unit("fn_a", CodeUnitType::Function, Language::Rust, "src/lib.rs");
    u0.feature_vec = vec![1.0, 0.0, 0.0, 0.0];
    graph.add_unit(u0);

    let index = EmbeddingIndex::build(&graph);

    // Wrong dimension query returns empty
    let results = index.search(&[1.0, 0.0], 10, 0.0);
    assert!(results.is_empty());

    // Zero query returns empty
    let results = index.search(&[0.0; 4], 10, 0.0);
    assert!(results.is_empty());
}

// ===========================================================================
// Mmap Tests (write → mmap → read roundtrip)
// ===========================================================================

#[test]
fn test_mmap_read_header() {
    let graph = build_index_graph();

    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("test_header.acb");

    // Write
    let writer = agentic_codebase::format::AcbWriter::new(4);
    writer
        .write_to_file(&graph, &path)
        .expect("write should succeed");

    // Mmap read
    let mapped =
        agentic_codebase::format::mmap::MappedCodeGraph::open(&path).expect("mmap should succeed");
    let read_graph = mapped.graph();

    assert_eq!(read_graph.unit_count(), graph.unit_count());
    assert_eq!(read_graph.edges().len(), graph.edges().len());
}

#[test]
fn test_mmap_read_unit() {
    let graph = build_index_graph();

    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("test_unit.acb");

    let writer = agentic_codebase::format::AcbWriter::new(4);
    writer
        .write_to_file(&graph, &path)
        .expect("write should succeed");

    let mapped =
        agentic_codebase::format::mmap::MappedCodeGraph::open(&path).expect("mmap should succeed");
    let read_graph = mapped.graph();

    // Verify first unit
    let unit = read_graph.get_unit(0).expect("unit 0 should exist");
    assert_eq!(unit.name, "process_payment");
    assert_eq!(unit.unit_type, CodeUnitType::Function);
    assert_eq!(unit.language, Language::Rust);
    assert_eq!(unit.file_path, PathBuf::from("src/lib.rs"));

    // Verify module
    let unit2 = read_graph.get_unit(2).expect("unit 2 should exist");
    assert_eq!(unit2.name, "payments");
    assert_eq!(unit2.unit_type, CodeUnitType::Module);

    // Verify Python unit
    let unit3 = read_graph.get_unit(3).expect("unit 3 should exist");
    assert_eq!(unit3.name, "process_refund");
    assert_eq!(unit3.language, Language::Python);
}

#[test]
fn test_mmap_read_edges() {
    let graph = build_index_graph();

    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("test_edges.acb");

    let writer = agentic_codebase::format::AcbWriter::new(4);
    writer
        .write_to_file(&graph, &path)
        .expect("write should succeed");

    let mapped =
        agentic_codebase::format::mmap::MappedCodeGraph::open(&path).expect("mmap should succeed");
    let read_graph = mapped.graph();

    // Should have 2 edges
    assert_eq!(read_graph.edges().len(), 2);

    // Verify edge types exist
    let has_calls = read_graph
        .edges()
        .iter()
        .any(|e| e.edge_type == EdgeType::Calls && e.source_id == 0 && e.target_id == 1);
    assert!(has_calls, "Should have Calls edge from 0 to 1");

    let has_tests = read_graph
        .edges()
        .iter()
        .any(|e| e.edge_type == EdgeType::Tests && e.source_id == 6 && e.target_id == 0);
    assert!(has_tests, "Should have Tests edge from 6 to 0");
}

#[test]
fn test_mmap_feature_vectors() {
    let dim = 4;
    let mut graph = CodeGraph::new(dim);

    let mut u0 = make_unit("fn_a", CodeUnitType::Function, Language::Rust, "src/lib.rs");
    u0.feature_vec = vec![1.0, 2.0, 3.0, 4.0];
    graph.add_unit(u0);

    let mut u1 = make_unit("fn_b", CodeUnitType::Function, Language::Rust, "src/lib.rs");
    u1.feature_vec = vec![5.0, 6.0, 7.0, 8.0];
    graph.add_unit(u1);

    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("test_fvec.acb");

    let writer = agentic_codebase::format::AcbWriter::new(dim);
    writer
        .write_to_file(&graph, &path)
        .expect("write should succeed");

    let mapped =
        agentic_codebase::format::mmap::MappedCodeGraph::open(&path).expect("mmap should succeed");
    let read_graph = mapped.graph();

    let u0_read = read_graph.get_unit(0).expect("unit 0");
    assert_eq!(u0_read.feature_vec, vec![1.0, 2.0, 3.0, 4.0]);

    let u1_read = read_graph.get_unit(1).expect("unit 1");
    assert_eq!(u1_read.feature_vec, vec![5.0, 6.0, 7.0, 8.0]);
}

#[test]
fn test_concurrent_mmap_read() {
    let graph = build_index_graph();

    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("test_concurrent.acb");

    let writer = agentic_codebase::format::AcbWriter::new(4);
    writer
        .write_to_file(&graph, &path)
        .expect("write should succeed");

    // Multiple threads reading the same file concurrently
    let path_clone = path.clone();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let p = path_clone.clone();
            std::thread::spawn(move || {
                let mapped = agentic_codebase::format::mmap::MappedCodeGraph::open(&p)
                    .expect("mmap should succeed");
                let g = mapped.graph();
                assert_eq!(g.unit_count(), 7);
                assert_eq!(g.edges().len(), 2);
                g.get_unit(0).expect("unit 0").name.clone()
            })
        })
        .collect();

    for handle in handles {
        let name = handle.join().expect("thread should not panic");
        assert_eq!(name, "process_payment");
    }
}

// ===========================================================================
// Index Rebuild / Roundtrip Tests
// ===========================================================================

#[test]
fn test_index_rebuild_after_mmap() {
    // Write → mmap → build indexes from read graph
    let graph = build_index_graph();

    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("test_rebuild.acb");

    let writer = agentic_codebase::format::AcbWriter::new(4);
    writer
        .write_to_file(&graph, &path)
        .expect("write should succeed");

    let mapped =
        agentic_codebase::format::mmap::MappedCodeGraph::open(&path).expect("mmap should succeed");
    let read_graph = mapped.graph();

    // Rebuild all indexes from the deserialized graph
    let sym_idx = SymbolIndex::build(read_graph);
    assert_eq!(sym_idx.len(), 7);
    assert_eq!(sym_idx.lookup_exact("process_payment"), &[0]);

    let type_idx = TypeIndex::build(read_graph);
    assert_eq!(type_idx.count(CodeUnitType::Function), 3);

    let path_idx = PathIndex::build(read_graph);
    assert_eq!(path_idx.file_count(), 5);

    let lang_idx = LanguageIndex::build(read_graph);
    assert_eq!(lang_idx.count(Language::Rust), 5);
}

#[test]
fn test_embedding_index_roundtrip() {
    // Write graph with feature vectors → read back → build embedding index → search
    let dim = 4;
    let mut graph = CodeGraph::new(dim);

    let mut u0 = make_unit(
        "fn_similar_a",
        CodeUnitType::Function,
        Language::Rust,
        "src/lib.rs",
    );
    u0.feature_vec = vec![1.0, 0.0, 0.0, 0.0];
    graph.add_unit(u0);

    let mut u1 = make_unit(
        "fn_similar_b",
        CodeUnitType::Function,
        Language::Rust,
        "src/lib.rs",
    );
    u1.feature_vec = vec![0.95, 0.05, 0.0, 0.0];
    graph.add_unit(u1);

    let mut u2 = make_unit(
        "fn_different",
        CodeUnitType::Function,
        Language::Rust,
        "src/lib.rs",
    );
    u2.feature_vec = vec![0.0, 0.0, 0.0, 1.0];
    graph.add_unit(u2);

    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("test_embed_rt.acb");

    let writer = agentic_codebase::format::AcbWriter::new(dim);
    writer
        .write_to_file(&graph, &path)
        .expect("write should succeed");

    let mapped =
        agentic_codebase::format::mmap::MappedCodeGraph::open(&path).expect("mmap should succeed");
    let read_graph = mapped.graph();

    let embed_idx = EmbeddingIndex::build(read_graph);
    assert_eq!(embed_idx.len(), 3);

    let results = embed_idx.search(&[1.0, 0.0, 0.0, 0.0], 2, 0.5);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].unit_id, 0);
    assert!((results[0].score - 1.0).abs() < 1e-5);
}
