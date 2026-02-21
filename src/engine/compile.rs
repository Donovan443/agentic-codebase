//! Main compilation pipeline.
//!
//! Orchestrates the full parse -> analyze -> build graph -> write `.acb` pipeline.
//! This is the central entry point used by both the CLI (`/compile`) and
//! programmatic callers.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::format::AcbWriter;
use crate::graph::CodeGraph;
use crate::parse::parser::{ParseOptions, Parser};
use crate::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};
use crate::types::{AcbResult, Language};

/// Options for the compilation pipeline.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Output path for the `.acb` file.
    pub output: PathBuf,
    /// Languages to include (empty = all supported).
    pub languages: Vec<Language>,
    /// Glob patterns to exclude from scanning.
    pub exclude_patterns: Vec<String>,
    /// Include test files in the graph.
    pub include_tests: bool,
    /// Detect design patterns during analysis.
    pub detect_patterns: bool,
    /// Extract high-level concepts.
    pub extract_concepts: bool,
    /// Trace FFI boundaries.
    pub trace_ffi: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            output: PathBuf::from("graph.acb"),
            languages: Vec::new(),
            exclude_patterns: Vec::new(),
            include_tests: true,
            detect_patterns: true,
            extract_concepts: true,
            trace_ffi: true,
        }
    }
}

/// Result of a compilation run.
pub struct CompileResult {
    /// The compiled code graph.
    pub graph: CodeGraph,
    /// Compilation statistics.
    pub stats: CompileStats,
}

/// Statistics from a compilation run.
#[derive(Debug, Clone)]
pub struct CompileStats {
    /// Number of source files parsed.
    pub files_parsed: usize,
    /// Number of parse errors encountered.
    pub parse_errors: usize,
    /// Number of code units in the graph.
    pub units_created: usize,
    /// Number of edges in the graph.
    pub edges_created: usize,
    /// Languages found in the codebase.
    pub languages: Vec<Language>,
    /// Total compilation duration.
    pub duration: std::time::Duration,
}

/// The compilation pipeline.
pub struct CompilePipeline;

impl CompilePipeline {
    /// Create a new compilation pipeline.
    pub fn new() -> Self {
        Self
    }

    /// Compile a directory into a code graph.
    ///
    /// Runs the full pipeline: parse -> analyze -> build graph.
    /// Does NOT write the `.acb` file -- call [`write`] separately.
    pub fn compile(&self, dir: &Path, opts: &CompileOptions) -> AcbResult<CompileResult> {
        let start = Instant::now();

        // Phase 1: Parse source files
        let parse_opts = ParseOptions {
            languages: opts.languages.clone(),
            exclude: opts.exclude_patterns.clone(),
            include_tests: opts.include_tests,
            ..ParseOptions::default()
        };

        tracing::info!("Parsing {}", dir.display());
        let parser = Parser::new();
        let parse_result = parser.parse_directory(dir, &parse_opts)?;

        let files_parsed = parse_result.stats.files_parsed;
        let parse_errors = parse_result.errors.len();

        if !parse_result.errors.is_empty() {
            for err in &parse_result.errors {
                tracing::warn!("Parse error: {}: {}", err.path.display(), err.message);
            }
        }

        // Phase 2: Semantic analysis
        let analyze_opts = AnalyzeOptions {
            detect_patterns: opts.detect_patterns,
            extract_concepts: opts.extract_concepts,
            trace_ffi: opts.trace_ffi,
        };

        tracing::info!("Analyzing {} units", parse_result.units.len());
        let analyzer = SemanticAnalyzer::new();
        let graph = analyzer.analyze(parse_result.units, &analyze_opts)?;

        let duration = start.elapsed();

        let stats = CompileStats {
            files_parsed,
            parse_errors,
            units_created: graph.unit_count(),
            edges_created: graph.edge_count(),
            languages: graph.languages().iter().copied().collect(),
            duration,
        };

        tracing::info!(
            "Compiled {} units, {} edges in {:.2?}",
            stats.units_created,
            stats.edges_created,
            stats.duration
        );

        Ok(CompileResult { graph, stats })
    }

    /// Write a compiled graph to an `.acb` file.
    pub fn write(&self, graph: &CodeGraph, output: &Path) -> AcbResult<()> {
        tracing::info!("Writing {}", output.display());

        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let writer = AcbWriter::new(graph.dimension());
        writer.write_to_file(graph, output)?;

        tracing::info!("Wrote {}", output.display());
        Ok(())
    }

    /// Compile and write in one step.
    pub fn compile_and_write(&self, dir: &Path, opts: &CompileOptions) -> AcbResult<CompileResult> {
        let result = self.compile(dir, opts)?;
        self.write(&result.graph, &opts.output)?;
        Ok(result)
    }
}

impl Default for CompilePipeline {
    fn default() -> Self {
        Self::new()
    }
}
