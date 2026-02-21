//! Full pipeline: parse -> analyze -> write -> read -> query.
//!
//! Demonstrates the complete AgenticCodebase workflow end-to-end.
//!
//! Usage:
//!   cargo run --example 07_full_pipeline

use agentic_codebase::engine::query::{
    CallDirection, CallGraphParams, ImpactParams, MatchMode, ProphecyParams, QueryEngine,
    SimilarityParams, SymbolLookupParams,
};
use agentic_codebase::format::{AcbReader, AcbWriter};
use agentic_codebase::parse::parser::{ParseOptions, Parser};
use agentic_codebase::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};
use std::fs;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -----------------------------------------------------------------------
    // 1. Create sample source files (multi-file project)
    // -----------------------------------------------------------------------
    let src_dir = TempDir::new()?;

    fs::write(
        src_dir.path().join("models.rs"),
        r#"
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

pub struct Order {
    pub id: u64,
    pub user_id: u64,
    pub total: f64,
}
"#,
    )?;

    fs::write(
        src_dir.path().join("repository.rs"),
        r#"
use crate::models::{User, Order};

pub trait Repository<T> {
    fn find_by_id(&self, id: u64) -> Option<T>;
    fn save(&self, entity: &T) -> Result<(), String>;
}

pub struct UserRepository;
impl Repository<User> for UserRepository {
    fn find_by_id(&self, id: u64) -> Option<User> { None }
    fn save(&self, entity: &User) -> Result<(), String> { Ok(()) }
}

pub struct OrderRepository;
impl Repository<Order> for OrderRepository {
    fn find_by_id(&self, id: u64) -> Option<Order> { None }
    fn save(&self, entity: &Order) -> Result<(), String> { Ok(()) }
}
"#,
    )?;

    fs::write(
        src_dir.path().join("service.rs"),
        r#"
use crate::models::{User, Order};
use crate::repository::{UserRepository, OrderRepository, Repository};

pub struct UserService {
    repo: UserRepository,
}

impl UserService {
    pub fn get_user(&self, id: u64) -> Option<User> {
        self.repo.find_by_id(id)
    }

    pub fn create_user(&self, name: &str, email: &str) -> Result<(), String> {
        let user = User { id: 0, name: name.to_string(), email: email.to_string() };
        self.repo.save(&user)
    }
}

pub struct OrderService {
    user_repo: UserRepository,
    order_repo: OrderRepository,
}

impl OrderService {
    pub fn create_order(&self, user_id: u64, total: f64) -> Result<(), String> {
        let _user = self.user_repo.find_by_id(user_id);
        let order = Order { id: 0, user_id, total };
        self.order_repo.save(&order)
    }
}
"#,
    )?;

    // -----------------------------------------------------------------------
    // 2. Parse
    // -----------------------------------------------------------------------
    println!("=== Step 1: Parse ===");
    let parser = Parser::new();
    let result = parser.parse_directory(src_dir.path(), &ParseOptions::default())?;
    println!(
        "  Parsed {} files, found {} units",
        result.stats.files_parsed,
        result.units.len()
    );

    // -----------------------------------------------------------------------
    // 3. Semantic Analysis
    // -----------------------------------------------------------------------
    println!("\n=== Step 2: Semantic Analysis ===");
    let analyzer = SemanticAnalyzer::new();
    let graph = analyzer.analyze(result.units, &AnalyzeOptions::default())?;
    println!(
        "  Graph: {} units, {} edges, {} languages",
        graph.unit_count(),
        graph.edge_count(),
        graph.languages().len()
    );

    // -----------------------------------------------------------------------
    // 4. Write .acb
    // -----------------------------------------------------------------------
    println!("\n=== Step 3: Write .acb ===");
    let out_dir = TempDir::new()?;
    let acb_path = out_dir.path().join("project.acb");
    let writer = AcbWriter::with_default_dimension();
    writer.write_to_file(&graph, &acb_path)?;
    let file_size = fs::metadata(&acb_path)?.len();
    println!("  Written to {} ({} bytes)", acb_path.display(), file_size);

    // -----------------------------------------------------------------------
    // 5. Read back
    // -----------------------------------------------------------------------
    println!("\n=== Step 4: Read .acb ===");
    let graph = AcbReader::read_from_file(&acb_path)?;
    println!(
        "  Loaded: {} units, {} edges",
        graph.unit_count(),
        graph.edge_count()
    );

    // -----------------------------------------------------------------------
    // 6. Queries
    // -----------------------------------------------------------------------
    let engine = QueryEngine::new();

    // Symbol lookup
    println!("\n=== Step 5: Symbol Lookup ===");
    let params = SymbolLookupParams {
        name: "Service".to_string(),
        mode: MatchMode::Contains,
        limit: 10,
        ..Default::default()
    };
    let results = engine.symbol_lookup(&graph, params)?;
    println!("  Found {} units matching 'Service':", results.len());
    for unit in &results {
        println!(
            "    [{}] {} ({})",
            unit.id, unit.qualified_name, unit.unit_type
        );
    }

    // Impact analysis on first result
    if let Some(first) = results.first() {
        println!("\n=== Step 6: Impact Analysis (unit {}) ===", first.id);
        let impact = engine.impact_analysis(
            &graph,
            ImpactParams {
                unit_id: first.id,
                max_depth: 5,
                edge_types: vec![],
            },
        )?;
        println!(
            "  Overall risk: {:.2}, {} units impacted",
            impact.overall_risk,
            impact.impacted.len()
        );

        // Call graph
        println!("\n=== Step 7: Call Graph (unit {}) ===", first.id);
        let calls = engine.call_graph(
            &graph,
            CallGraphParams {
                unit_id: first.id,
                direction: CallDirection::Both,
                max_depth: 3,
            },
        )?;
        println!("  {} nodes in call graph", calls.nodes.len());
        for (nid, depth) in &calls.nodes {
            let name = graph
                .get_unit(*nid)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            println!("    [depth {}] {}", depth, name);
        }

        // Similarity
        println!("\n=== Step 8: Similarity (unit {}) ===", first.id);
        let similar = engine.similarity(
            &graph,
            SimilarityParams {
                unit_id: first.id,
                top_k: 5,
                min_similarity: 0.0,
            },
        )?;
        println!("  {} similar units:", similar.len());
        for m in &similar {
            let name = graph
                .get_unit(m.unit_id)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            println!("    {} ({:.1}%)", name, m.score * 100.0);
        }
    }

    // Prophecy
    println!("\n=== Step 9: Code Prophecy ===");
    let prophecy = engine.prophecy(
        &graph,
        ProphecyParams {
            top_k: 5,
            min_risk: 0.0,
        },
    )?;
    if prophecy.predictions.is_empty() {
        println!("  No high-risk predictions. Codebase looks healthy!");
    } else {
        for pred in &prophecy.predictions {
            let name = graph
                .get_unit(pred.unit_id)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            println!("  {} (risk: {:.2}): {}", name, pred.risk_score, pred.reason);
        }
    }

    println!("\n=== Pipeline Complete ===");
    Ok(())
}
