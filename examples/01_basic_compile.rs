//! Basic compile flow: parse a directory, analyze, and inspect the graph.
//!
//! Usage:
//!   cargo run --example 01_basic_compile

use agentic_codebase::parse::parser::{ParseOptions, Parser};
use agentic_codebase::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};
use std::fs;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory with sample Rust files
    let dir = TempDir::new()?;
    fs::write(
        dir.path().join("lib.rs"),
        r#"
/// A simple user service.
pub struct UserService {
    db: Database,
}

impl UserService {
    /// Creates a new user service.
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Saves a user to the database.
    pub fn save_user(&self, name: &str) -> Result<(), String> {
        self.db.insert(name)
    }
}

/// Database abstraction.
pub struct Database;

impl Database {
    pub fn insert(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }
}
"#,
    )?;

    // Parse
    let parser = Parser::new();
    let result = parser.parse_directory(dir.path(), &ParseOptions::default())?;
    println!("Parsed {} files", result.stats.files_parsed);
    println!("Found {} code units", result.units.len());
    println!("Parse errors: {}", result.errors.len());

    // Analyze
    let analyzer = SemanticAnalyzer::new();
    let graph = analyzer.analyze(result.units, &AnalyzeOptions::default())?;

    // Inspect
    println!("\nGraph summary:");
    println!("  Units: {}", graph.unit_count());
    println!("  Edges: {}", graph.edge_count());
    println!("  Languages: {:?}", graph.languages());

    println!("\nAll units:");
    for unit in graph.units() {
        println!(
            "  [{}] {} ({}) - complexity: {}",
            unit.id, unit.qualified_name, unit.unit_type, unit.complexity
        );
    }

    println!("\nAll edges:");
    for edge in graph.edges() {
        let src = graph
            .get_unit(edge.source_id)
            .map(|u| u.name.as_str())
            .unwrap_or("?");
        let tgt = graph
            .get_unit(edge.target_id)
            .map(|u| u.name.as_str())
            .unwrap_or("?");
        println!("  {} --[{}]--> {}", src, edge.edge_type, tgt);
    }

    Ok(())
}
