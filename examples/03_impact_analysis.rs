//! Impact analysis: what breaks if we change a specific unit?
//!
//! Usage:
//!   cargo run --example 03_impact_analysis

use agentic_codebase::engine::query::{ImpactParams, QueryEngine};
use agentic_codebase::parse::parser::{ParseOptions, Parser};
use agentic_codebase::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};
use std::fs;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a codebase with dependencies
    let dir = TempDir::new()?;
    fs::write(
        dir.path().join("core.rs"),
        r#"
/// Core database connection pool.
pub struct ConnectionPool {
    max_connections: usize,
}

impl ConnectionPool {
    pub fn new(max: usize) -> Self {
        Self { max_connections: max }
    }

    pub fn get_connection(&self) -> Connection {
        Connection
    }
}

pub struct Connection;

/// Repository that uses the connection pool.
pub struct UserRepository {
    pool: ConnectionPool,
}

impl UserRepository {
    pub fn find_by_id(&self, id: u64) -> Option<User> {
        let _conn = self.pool.get_connection();
        None
    }
}

pub struct User {
    pub id: u64,
    pub name: String,
}

/// Service layer depends on repository.
pub struct UserService {
    repo: UserRepository,
}

impl UserService {
    pub fn get_user(&self, id: u64) -> Option<User> {
        self.repo.find_by_id(id)
    }
}

/// API handler depends on service.
pub fn handle_get_user(service: &UserService, id: u64) -> String {
    match service.get_user(id) {
        Some(user) => format!("User: {}", user.name),
        None => "Not found".to_string(),
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

    // Run impact analysis on the first unit (ConnectionPool)
    if graph.unit_count() > 0 {
        let target_id = 0;
        let target_name = graph
            .get_unit(target_id)
            .map(|u| u.qualified_name.as_str())
            .unwrap_or("?");

        println!("Impact analysis for: {} (id={})", target_name, target_id);

        let params = ImpactParams {
            unit_id: target_id,
            max_depth: 5,
            edge_types: vec![],
        };
        let result = engine.impact_analysis(&graph, params)?;

        let risk_label = if result.overall_risk >= 0.7 {
            "HIGH"
        } else if result.overall_risk >= 0.4 {
            "MEDIUM"
        } else {
            "LOW"
        };

        println!(
            "  Overall risk: {:.2} ({})",
            result.overall_risk, risk_label
        );
        println!("  Impacted units: {}", result.impacted.len());
        println!();

        for imp in &result.impacted {
            let name = graph
                .get_unit(imp.unit_id)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            let test_status = if imp.has_tests { "tested" } else { "untested" };
            println!(
                "    [depth {}] {} (risk: {:.2}, {})",
                imp.depth, name, imp.risk_score, test_status
            );
        }

        if !result.recommendations.is_empty() {
            println!("\n  Recommendations:");
            for rec in &result.recommendations {
                println!("    - {}", rec);
            }
        }
    }

    Ok(())
}
