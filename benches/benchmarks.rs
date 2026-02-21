//! Criterion benchmarks for AgenticCodebase.
//!
//! Covers: graph construction, queries, I/O, string pool, and index operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use agentic_codebase::engine::query::{
    CallDirection, CallGraphParams, DependencyParams, ImpactParams, MatchMode, QueryEngine,
    SymbolLookupParams,
};
use agentic_codebase::format::{AcbReader, AcbWriter};
use agentic_codebase::graph::{CodeGraph, GraphBuilder};
use agentic_codebase::index::{EmbeddingIndex, LanguageIndex, PathIndex, SymbolIndex, TypeIndex};
use agentic_codebase::types::*;

// ---------------------------------------------------------------------------
// Fixture builder
// ---------------------------------------------------------------------------

fn build_graph(n: usize) -> CodeGraph {
    let mut builder = GraphBuilder::new(DEFAULT_DIMENSION);
    builder = builder.add_unit(CodeUnit::new(
        CodeUnitType::Module,
        Language::Python,
        "root_module".into(),
        "root_module".into(),
        "src/root.py".into(),
        Span::new(1, 0, 100, 0),
    ));
    for i in 1..n {
        let name = format!("function_{}", i);
        let file = format!("src/mod_{}.py", i % 100);
        let lang = match i % 3 {
            0 => Language::Rust,
            1 => Language::Python,
            _ => Language::TypeScript,
        };
        builder = builder.add_unit(CodeUnit::new(
            CodeUnitType::Function,
            lang,
            name.clone(),
            format!("mod_{}.{}", i % 100, name),
            file.into(),
            Span::new(1, 0, 20, 0),
        ));
        if i > 1 {
            builder = builder.add_edge(Edge::new((i - 1) as u64, i as u64, EdgeType::Calls));
        }
        if i % 10 == 0 {
            builder = builder.add_edge(Edge::new(i as u64, 0, EdgeType::Imports));
        }
    }
    builder.build().unwrap()
}

// ---------------------------------------------------------------------------
// Graph construction benchmarks
// ---------------------------------------------------------------------------

fn bench_build_1k(c: &mut Criterion) {
    c.bench_function("build_graph_1k", |b| {
        b.iter(|| {
            let g = build_graph(black_box(1_000));
            black_box(g.unit_count());
        })
    });
}

fn bench_build_10k(c: &mut Criterion) {
    c.bench_function("build_graph_10k", |b| {
        b.iter(|| {
            let g = build_graph(black_box(10_000));
            black_box(g.unit_count());
        })
    });
}

// ---------------------------------------------------------------------------
// I/O benchmarks
// ---------------------------------------------------------------------------

fn bench_write_1k(c: &mut Criterion) {
    let graph = build_graph(1_000);
    let writer = AcbWriter::with_default_dimension();
    c.bench_function("write_acb_1k", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(256 * 1024);
            writer.write_to(&graph, &mut buf).unwrap();
            black_box(buf.len());
        })
    });
}

fn bench_write_10k(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let writer = AcbWriter::with_default_dimension();
    c.bench_function("write_acb_10k", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(2 * 1024 * 1024);
            writer.write_to(&graph, &mut buf).unwrap();
            black_box(buf.len());
        })
    });
}

fn bench_read_1k(c: &mut Criterion) {
    let graph = build_graph(1_000);
    let writer = AcbWriter::with_default_dimension();
    let mut buf = Vec::new();
    writer.write_to(&graph, &mut buf).unwrap();
    c.bench_function("read_acb_1k", |b| {
        b.iter(|| {
            let g = AcbReader::read_from_data(black_box(&buf)).unwrap();
            black_box(g.unit_count());
        })
    });
}

fn bench_read_10k(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let writer = AcbWriter::with_default_dimension();
    let mut buf = Vec::new();
    writer.write_to(&graph, &mut buf).unwrap();
    c.bench_function("read_acb_10k", |b| {
        b.iter(|| {
            let g = AcbReader::read_from_data(black_box(&buf)).unwrap();
            black_box(g.unit_count());
        })
    });
}

// ---------------------------------------------------------------------------
// Query benchmarks
// ---------------------------------------------------------------------------

fn bench_symbol_lookup_exact(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let engine = QueryEngine::new();
    c.bench_function("symbol_lookup_exact_10k", |b| {
        b.iter(|| {
            let results = engine.symbol_lookup(
                black_box(&graph),
                SymbolLookupParams {
                    name: "function_5000".into(),
                    mode: MatchMode::Exact,
                    ..Default::default()
                },
            );
            black_box(results.is_ok());
        })
    });
}

fn bench_symbol_lookup_prefix(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let engine = QueryEngine::new();
    c.bench_function("symbol_lookup_prefix_10k", |b| {
        b.iter(|| {
            let results = engine.symbol_lookup(
                black_box(&graph),
                SymbolLookupParams {
                    name: "function_50".into(),
                    mode: MatchMode::Prefix,
                    limit: 100,
                    ..Default::default()
                },
            );
            black_box(results.is_ok());
        })
    });
}

fn bench_symbol_lookup_contains(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let engine = QueryEngine::new();
    c.bench_function("symbol_lookup_contains_10k", |b| {
        b.iter(|| {
            let results = engine.symbol_lookup(
                black_box(&graph),
                SymbolLookupParams {
                    name: "500".into(),
                    mode: MatchMode::Contains,
                    limit: 100,
                    ..Default::default()
                },
            );
            black_box(results.is_ok());
        })
    });
}

fn bench_dependency_graph(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let engine = QueryEngine::new();
    c.bench_function("dependency_graph_depth5_10k", |b| {
        b.iter(|| {
            let result = engine.dependency_graph(
                black_box(&graph),
                DependencyParams {
                    unit_id: 5000,
                    max_depth: 5,
                    edge_types: vec![],
                    include_transitive: true,
                },
            );
            black_box(result.is_ok());
        })
    });
}

fn bench_impact_analysis(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let engine = QueryEngine::new();
    c.bench_function("impact_analysis_10k", |b| {
        b.iter(|| {
            let result = engine.impact_analysis(
                black_box(&graph),
                ImpactParams {
                    unit_id: 5000,
                    max_depth: 5,
                    edge_types: vec![],
                },
            );
            black_box(result.is_ok());
        })
    });
}

fn bench_call_graph(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let engine = QueryEngine::new();
    c.bench_function("call_graph_10k", |b| {
        b.iter(|| {
            let result = engine.call_graph(
                black_box(&graph),
                CallGraphParams {
                    unit_id: 5000,
                    direction: CallDirection::Both,
                    max_depth: 3,
                },
            );
            black_box(result.is_ok());
        })
    });
}

// ---------------------------------------------------------------------------
// Index benchmarks
// ---------------------------------------------------------------------------

fn bench_symbol_index_build(c: &mut Criterion) {
    let graph = build_graph(10_000);
    c.bench_function("symbol_index_build_10k", |b| {
        b.iter(|| {
            let idx = SymbolIndex::build(black_box(&graph));
            black_box(idx.len());
        })
    });
}

fn bench_symbol_index_lookup(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let idx = SymbolIndex::build(&graph);
    c.bench_function("symbol_index_exact_10k", |b| {
        b.iter(|| {
            let results = idx.lookup_exact(black_box("function_5000"));
            black_box(results.len());
        })
    });
}

fn bench_symbol_index_prefix(c: &mut Criterion) {
    let graph = build_graph(10_000);
    let idx = SymbolIndex::build(&graph);
    c.bench_function("symbol_index_prefix_10k", |b| {
        b.iter(|| {
            let results = idx.lookup_prefix(black_box("function_50"));
            black_box(results.len());
        })
    });
}

fn bench_type_index_build(c: &mut Criterion) {
    let graph = build_graph(10_000);
    c.bench_function("type_index_build_10k", |b| {
        b.iter(|| {
            let idx = TypeIndex::build(black_box(&graph));
            black_box(idx.count(CodeUnitType::Function));
        })
    });
}

fn bench_language_index_build(c: &mut Criterion) {
    let graph = build_graph(10_000);
    c.bench_function("language_index_build_10k", |b| {
        b.iter(|| {
            let idx = LanguageIndex::build(black_box(&graph));
            black_box(idx.count(Language::Python));
        })
    });
}

fn bench_path_index_build(c: &mut Criterion) {
    let graph = build_graph(10_000);
    c.bench_function("path_index_build_10k", |b| {
        b.iter(|| {
            let idx = PathIndex::build(black_box(&graph));
            black_box(idx.file_count());
        })
    });
}

fn bench_embedding_index_search(c: &mut Criterion) {
    let mut graph = build_graph(1_000);
    let dim = graph.dimension();
    for i in 0..graph.unit_count() {
        let mut vec = vec![0.0f32; dim];
        for (d, v) in vec.iter_mut().enumerate().take(dim) {
            *v = ((i * 7 + d * 3) as f32 % 100.0) / 100.0;
        }
        if let Some(unit) = graph.get_unit_mut(i as u64) {
            unit.feature_vec = vec;
        }
    }
    let idx = EmbeddingIndex::build(&graph);
    let query: Vec<f32> = (0..dim).map(|d| (d as f32 % 100.0) / 100.0).collect();
    c.bench_function("embedding_search_1k_top10", |b| {
        b.iter(|| {
            let results = idx.search(black_box(&query), 10, 0.0);
            black_box(results.len());
        })
    });
}

// ---------------------------------------------------------------------------
// String pool benchmarks
// ---------------------------------------------------------------------------

fn bench_string_compression(c: &mut Criterion) {
    use agentic_codebase::format::compression::StringPoolBuilder;
    let strings: Vec<String> = (0..10_000)
        .map(|i| format!("some_module.some_class.function_{}", i))
        .collect();
    c.bench_function("string_pool_compress_10k", |b| {
        b.iter(|| {
            let mut pool = StringPoolBuilder::new();
            for s in &strings {
                pool.add(black_box(s));
            }
            let compressed = pool.compress();
            black_box(compressed.len());
        })
    });
}

fn bench_string_decompression(c: &mut Criterion) {
    use agentic_codebase::format::compression::{StringPool, StringPoolBuilder};
    let mut pool_builder = StringPoolBuilder::new();
    let mut offsets = Vec::new();
    for i in 0..10_000 {
        let s = format!("some_module.some_class.function_{}", i);
        let (offset, len) = pool_builder.add(&s);
        offsets.push((offset, len));
    }
    let compressed = pool_builder.compress();
    c.bench_function("string_pool_decompress_10k", |b| {
        b.iter(|| {
            let pool = StringPool::from_compressed(black_box(&compressed)).unwrap();
            for &(offset, len) in &offsets {
                let _ = black_box(pool.get(offset, len));
            }
        })
    });
}

// ---------------------------------------------------------------------------
// Register benchmark groups
// ---------------------------------------------------------------------------

criterion_group!(graph_benches, bench_build_1k, bench_build_10k);

criterion_group!(
    io_benches,
    bench_write_1k,
    bench_write_10k,
    bench_read_1k,
    bench_read_10k,
);

criterion_group!(
    query_benches,
    bench_symbol_lookup_exact,
    bench_symbol_lookup_prefix,
    bench_symbol_lookup_contains,
    bench_dependency_graph,
    bench_impact_analysis,
    bench_call_graph,
);

criterion_group!(
    index_benches,
    bench_symbol_index_build,
    bench_symbol_index_lookup,
    bench_symbol_index_prefix,
    bench_type_index_build,
    bench_language_index_build,
    bench_path_index_build,
    bench_embedding_index_search,
);

criterion_group!(
    string_benches,
    bench_string_compression,
    bench_string_decompression
);

criterion_main!(
    graph_benches,
    io_benches,
    query_benches,
    index_benches,
    string_benches,
);
