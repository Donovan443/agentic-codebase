//! Query symbols by name with different match modes.
//!
//! Usage:
//!   cargo run --example 02_query_symbols

use agentic_codebase::engine::query::{MatchMode, QueryEngine, SymbolLookupParams};
use agentic_codebase::parse::parser::{ParseOptions, Parser};
use agentic_codebase::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};
use std::fs;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create sample source
    let dir = TempDir::new()?;
    fs::write(
        dir.path().join("app.py"),
        r#"
class UserService:
    def get_user(self, user_id: int) -> dict:
        pass

    def update_user(self, user_id: int, data: dict) -> bool:
        pass

class OrderService:
    def create_order(self, user_id: int, items: list) -> dict:
        pass

    def get_order(self, order_id: int) -> dict:
        pass

def process_payment(amount: float) -> bool:
    pass
"#,
    )?;

    // Parse and analyze
    let parser = Parser::new();
    let result = parser.parse_directory(dir.path(), &ParseOptions::default())?;
    let analyzer = SemanticAnalyzer::new();
    let graph = analyzer.analyze(result.units, &AnalyzeOptions::default())?;
    let engine = QueryEngine::new();

    // Search by substring
    println!("=== Contains 'user' ===");
    let params = SymbolLookupParams {
        name: "user".to_string(),
        mode: MatchMode::Contains,
        limit: 20,
        ..Default::default()
    };
    let results = engine.symbol_lookup(&graph, params)?;
    for unit in &results {
        println!(
            "  {} ({}) at {}:{}",
            unit.qualified_name,
            unit.unit_type,
            unit.file_path.display(),
            unit.span.start_line
        );
    }

    // Search by prefix
    println!("\n=== Prefix 'get' ===");
    let params = SymbolLookupParams {
        name: "get".to_string(),
        mode: MatchMode::Prefix,
        limit: 20,
        ..Default::default()
    };
    let results = engine.symbol_lookup(&graph, params)?;
    for unit in &results {
        println!("  {} ({})", unit.qualified_name, unit.unit_type);
    }

    // Search by exact match
    println!("\n=== Exact 'process_payment' ===");
    let params = SymbolLookupParams {
        name: "process_payment".to_string(),
        mode: MatchMode::Exact,
        limit: 20,
        ..Default::default()
    };
    let results = engine.symbol_lookup(&graph, params)?;
    for unit in &results {
        println!(
            "  {} ({}) complexity={}",
            unit.qualified_name, unit.unit_type, unit.complexity
        );
    }

    println!("\nTotal units in graph: {}", graph.unit_count());

    Ok(())
}
