//! Dependency graph: trace forward and reverse dependencies.
//!
//! Usage:
//!   cargo run --example 04_dependency_graph

use agentic_codebase::engine::query::{DependencyParams, QueryEngine};
use agentic_codebase::parse::parser::{ParseOptions, Parser};
use agentic_codebase::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};
use std::fs;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    fs::write(
        dir.path().join("modules.rs"),
        r#"
pub mod config {
    pub fn load_config() -> Config {
        Config { debug: true }
    }
    pub struct Config {
        pub debug: bool,
    }
}

pub mod logger {
    use super::config;
    pub fn init_logger(cfg: &config::Config) {
        if cfg.debug {
            println!("Debug logging enabled");
        }
    }
}

pub mod app {
    use super::config;
    use super::logger;
    pub fn start() {
        let cfg = config::load_config();
        logger::init_logger(&cfg);
        println!("App started");
    }
}
"#,
    )?;

    // Parse and analyze
    let parser = Parser::new();
    let result = parser.parse_directory(dir.path(), &ParseOptions::default())?;
    let analyzer = SemanticAnalyzer::new();
    let graph = analyzer.analyze(result.units, &AnalyzeOptions::default())?;
    let engine = QueryEngine::new();

    println!(
        "Graph: {} units, {} edges\n",
        graph.unit_count(),
        graph.edge_count()
    );

    // Show all units
    println!("All units:");
    for unit in graph.units() {
        println!(
            "  [{}] {} ({})",
            unit.id, unit.qualified_name, unit.unit_type
        );
    }

    // Forward dependencies from first unit
    if graph.unit_count() > 0 {
        let start_id = 0;
        let start_name = graph
            .get_unit(start_id)
            .map(|u| u.qualified_name.as_str())
            .unwrap_or("?");

        println!(
            "\nForward dependencies of {} (id={}):",
            start_name, start_id
        );
        let params = DependencyParams {
            unit_id: start_id,
            max_depth: 5,
            edge_types: vec![],
            include_transitive: true,
        };
        let result = engine.dependency_graph(&graph, params)?;
        for node in &result.nodes {
            let name = graph
                .get_unit(node.unit_id)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            let indent = "  ".repeat(node.depth as usize);
            println!("  {}-> {} [depth {}]", indent, name, node.depth);
        }

        // Reverse dependencies
        println!(
            "\nReverse dependencies of {} (who depends on it):",
            start_name
        );
        let params = DependencyParams {
            unit_id: start_id,
            max_depth: 5,
            edge_types: vec![],
            include_transitive: true,
        };
        let result = engine.reverse_dependency(&graph, params)?;
        for node in &result.nodes {
            let name = graph
                .get_unit(node.unit_id)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            let indent = "  ".repeat(node.depth as usize);
            println!("  {}<- {} [depth {}]", indent, name, node.depth);
        }
    }

    Ok(())
}
