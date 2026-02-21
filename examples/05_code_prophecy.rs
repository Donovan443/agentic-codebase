//! Code prophecy: predict which units are most likely to break.
//!
//! Usage:
//!   cargo run --example 05_code_prophecy

use agentic_codebase::engine::query::{CouplingParams, ProphecyParams, QueryEngine};
use agentic_codebase::parse::parser::{ParseOptions, Parser};
use agentic_codebase::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};
use std::fs;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;

    // Create a codebase with varying complexity and coupling
    fs::write(
        dir.path().join("complex.rs"),
        r#"
/// Simple, stable function.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Complex function with high cyclomatic complexity.
pub fn process_order(order: &Order, config: &Config) -> Result<Receipt, Error> {
    if !order.is_valid() {
        return Err(Error::Invalid);
    }
    if config.require_auth && !order.is_authenticated() {
        return Err(Error::Unauthorized);
    }
    let total = calculate_total(order);
    if total > config.max_amount {
        return Err(Error::OverLimit);
    }
    if order.needs_approval() {
        request_approval(order);
    }
    apply_discount(order, config);
    charge_payment(order, total);
    Ok(Receipt { total })
}

pub fn calculate_total(order: &Order) -> f64 { 0.0 }
pub fn request_approval(order: &Order) {}
pub fn apply_discount(order: &Order, config: &Config) {}
pub fn charge_payment(order: &Order, total: f64) {}

pub struct Order;
impl Order {
    pub fn is_valid(&self) -> bool { true }
    pub fn is_authenticated(&self) -> bool { true }
    pub fn needs_approval(&self) -> bool { false }
}

pub struct Config {
    pub require_auth: bool,
    pub max_amount: f64,
}

pub struct Receipt { total: f64 }
pub enum Error { Invalid, Unauthorized, OverLimit }
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

    // Run prophecy
    println!("=== Code Prophecy ===");
    let params = ProphecyParams {
        top_k: 10,
        min_risk: 0.0,
    };
    let result = engine.prophecy(&graph, params)?;

    if result.predictions.is_empty() {
        println!("  No high-risk predictions. Codebase looks stable!");
    } else {
        for pred in &result.predictions {
            let name = graph
                .get_unit(pred.unit_id)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            let risk_label = if pred.risk_score >= 0.7 {
                "HIGH"
            } else if pred.risk_score >= 0.4 {
                "MEDIUM"
            } else {
                "LOW"
            };
            println!(
                "  [{}] {} (risk: {:.2}) - {}",
                risk_label, name, pred.risk_score, pred.reason
            );
        }
    }

    // Run coupling detection
    println!("\n=== Coupling Detection ===");
    let params = CouplingParams {
        unit_id: None,
        min_strength: 0.0,
    };
    let couplings = engine.coupling_detection(&graph, params)?;

    if couplings.is_empty() {
        println!("  No tightly coupled pairs detected.");
    } else {
        for c in &couplings {
            let name_a = graph
                .get_unit(c.unit_a)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            let name_b = graph
                .get_unit(c.unit_b)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            println!(
                "  {} <-> {} (strength: {:.0}%)",
                name_a,
                name_b,
                c.strength * 100.0
            );
        }
    }

    // Stability scores for each unit
    println!("\n=== Stability Scores ===");
    for unit in graph.units() {
        let stability_label = if unit.stability_score >= 0.7 {
            "stable"
        } else if unit.stability_score >= 0.4 {
            "moderate"
        } else {
            "unstable"
        };
        println!(
            "  {} = {:.2} ({})",
            unit.qualified_name, unit.stability_score, stability_label
        );
    }

    Ok(())
}
