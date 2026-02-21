//! Slash command parsing and dispatch for the ACB REPL.
//!
//! Each slash command maps to functionality from the existing CLI,
//! adapted for interactive session context (e.g., loaded graph tracking).

use std::path::{Path, PathBuf};

use crate::cli::output::{format_size, Styled};
use crate::cli::repl_complete::COMMANDS;
use crate::engine::query::{
    CallDirection, CallGraphParams, CouplingParams, DependencyParams, ImpactParams, MatchMode,
    ProphecyParams, QueryEngine, SimilarityParams, SymbolLookupParams,
};
use crate::format::{AcbReader, AcbWriter};
use crate::graph::CodeGraph;
use crate::parse::parser::{ParseOptions, Parser};
use crate::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};

/// Session state preserved across commands.
pub struct ReplState {
    /// Currently loaded .acb graph for querying.
    pub graph: Option<CodeGraph>,
    /// Path to the loaded .acb file.
    pub graph_path: Option<PathBuf>,
}

impl Default for ReplState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplState {
    pub fn new() -> Self {
        Self {
            graph: None,
            graph_path: None,
        }
    }

    /// Get the loaded graph or print an error hint.
    fn require_graph(&self) -> Option<&CodeGraph> {
        if let Some(ref g) = self.graph {
            Some(g)
        } else {
            let s = Styled::auto();
            eprintln!(
                "  {} No graph loaded. Use {} or {}",
                s.info(),
                s.bold("/load <file.acb>"),
                s.bold("/compile <dir>")
            );
            None
        }
    }
}

/// Parse and execute a slash command. Returns `true` if the REPL should exit.
pub fn execute(input: &str, state: &mut ReplState) -> Result<bool, Box<dyn std::error::Error>> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(false);
    }

    // Strip leading / if present
    let input = input.strip_prefix('/').unwrap_or(input);

    // Bare `/` → show help
    if input.is_empty() {
        cmd_help();
        return Ok(false);
    }

    // Split into command and arguments
    let mut parts = input.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("").trim();

    match cmd {
        "exit" | "quit" => return Ok(true),
        "help" | "h" | "?" => cmd_help(),
        "clear" | "cls" => cmd_clear(),
        "compile" | "build" => cmd_compile(args, state)?,
        "info" => cmd_info(args, state)?,
        "load" => cmd_load(args, state)?,
        "query" | "q" => cmd_query(args, state)?,
        "get" => cmd_get(args, state)?,
        "units" | "ls" => cmd_units(state)?,
        _ => {
            let s = Styled::auto();
            if let Some(suggestion) = crate::cli::repl_complete::suggest_command(cmd) {
                eprintln!(
                    "  {} Unknown command '/{cmd}'. Did you mean {}?",
                    s.warn(),
                    s.bold(suggestion)
                );
            } else {
                eprintln!(
                    "  {} Unknown command '/{cmd}'. Type {} for commands.",
                    s.warn(),
                    s.bold("/help"),
                );
            }
        }
    }

    Ok(false)
}

/// /help — Show available commands.
fn cmd_help() {
    let s = Styled::auto();
    eprintln!();
    eprintln!("  {}", s.bold("Commands:"));
    eprintln!();
    for (cmd, desc) in COMMANDS {
        eprintln!("    {:<22} {}", s.cyan(cmd), s.dim(desc));
    }
    eprintln!();
    eprintln!(
        "  {}",
        s.dim("Tip: Tab completion works for commands, query types, and .acb files.")
    );
    eprintln!();
}

/// /clear — Clear the terminal.
fn cmd_clear() {
    eprint!("\x1b[2J\x1b[H");
}

/// /compile <dir> — Compile a directory into an .acb graph.
fn cmd_compile(args: &str, state: &mut ReplState) -> Result<(), Box<dyn std::error::Error>> {
    let s = Styled::auto();

    if args.is_empty() {
        eprintln!("  {} Usage: {}", s.info(), s.bold("/compile <directory>"));
        return Ok(());
    }

    let tokens: Vec<&str> = args.split_whitespace().collect();
    let dir_path = Path::new(tokens[0]);

    if !dir_path.exists() || !dir_path.is_dir() {
        eprintln!(
            "  {} Not a valid directory: {}",
            s.fail(),
            dir_path.display()
        );
        return Ok(());
    }

    let out_name = dir_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "output".to_string());
    let out_path = PathBuf::from(format!("{}.acb", out_name));

    eprintln!();
    eprintln!(
        "  {} Compiling {} {} {}",
        s.info(),
        s.bold(&dir_path.display().to_string()),
        s.arrow(),
        s.cyan(&out_path.display().to_string()),
    );

    let parser = Parser::new();
    let parse_result = parser.parse_directory(dir_path, &ParseOptions::default())?;
    eprintln!(
        "  {} Parsed {} files ({} units)",
        s.ok(),
        parse_result.stats.files_parsed,
        parse_result.units.len(),
    );

    let analyzer = SemanticAnalyzer::new();
    let graph = analyzer.analyze(parse_result.units, &AnalyzeOptions::default())?;

    let writer = AcbWriter::with_default_dimension();
    writer.write_to_file(&graph, &out_path)?;

    let file_size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    eprintln!(
        "  {} Compiled: {} units, {} edges ({})",
        s.ok(),
        s.bold(&graph.unit_count().to_string()),
        graph.edge_count(),
        s.dim(&format_size(file_size)),
    );

    // Auto-load the compiled graph
    state.graph_path = Some(out_path.clone());
    state.graph = Some(graph);
    eprintln!(
        "  {} Graph loaded. Try: {}",
        s.info(),
        s.cyan("/query symbol --name <search>")
    );
    eprintln!();

    Ok(())
}

/// /load <file.acb> — Load an .acb file into the session.
fn cmd_load(args: &str, state: &mut ReplState) -> Result<(), Box<dyn std::error::Error>> {
    let s = Styled::auto();

    if args.is_empty() {
        eprintln!("  {} Usage: {}", s.info(), s.bold("/load <file.acb>"));
        return Ok(());
    }

    let path = PathBuf::from(args.split_whitespace().next().unwrap_or(args));
    if !path.exists() {
        eprintln!("  {} File not found: {}", s.fail(), path.display());
        return Ok(());
    }

    let graph = AcbReader::read_from_file(&path)?;
    eprintln!(
        "  {} Loaded {} ({} units, {} edges)",
        s.ok(),
        s.bold(&path.display().to_string()),
        graph.unit_count(),
        graph.edge_count(),
    );

    state.graph_path = Some(path);
    state.graph = Some(graph);
    Ok(())
}

/// /info [file] — Display summary of loaded graph or a specified file.
fn cmd_info(args: &str, state: &mut ReplState) -> Result<(), Box<dyn std::error::Error>> {
    let s = Styled::auto();

    let graph = if args.is_empty() {
        match state.require_graph() {
            Some(g) => g,
            None => return Ok(()),
        }
    } else {
        let path = PathBuf::from(args.split_whitespace().next().unwrap_or(args));
        let g = AcbReader::read_from_file(&path)?;
        state.graph_path = Some(path);
        state.graph = Some(g);
        state.graph.as_ref().unwrap()
    };

    let file_label = state
        .graph_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(in-memory)".to_string());

    eprintln!();
    eprintln!("  {} {}", s.info(), s.bold(&file_label));
    eprintln!(
        "     Units:     {}",
        s.bold(&graph.unit_count().to_string())
    );
    eprintln!(
        "     Edges:     {}",
        s.bold(&graph.edge_count().to_string())
    );
    eprintln!(
        "     Languages: {}",
        s.bold(&graph.languages().len().to_string())
    );
    for lang in graph.languages() {
        let count = graph.units().iter().filter(|u| u.language == *lang).count();
        eprintln!(
            "     {} {} {}",
            s.arrow(),
            s.cyan(&format!("{:12}", lang)),
            s.dim(&format!("{} units", count))
        );
    }
    eprintln!();

    Ok(())
}

/// /query <type> [flags] — Run a query against the loaded graph.
fn cmd_query(args: &str, state: &mut ReplState) -> Result<(), Box<dyn std::error::Error>> {
    let s = Styled::auto();
    let graph = match state.require_graph() {
        Some(g) => g,
        None => return Ok(()),
    };

    let engine = QueryEngine::new();
    let tokens: Vec<&str> = args.split_whitespace().collect();

    if tokens.is_empty() {
        eprintln!(
            "  {} Usage: {}",
            s.info(),
            s.bold("/query <type> [--name <n>] [--unit-id <id>] [--depth <d>] [--limit <l>]")
        );
        eprintln!(
            "  {} Types: symbol, deps, rdeps, impact, calls, similar, prophecy, stability, coupling",
            s.dim("  ")
        );
        return Ok(());
    }

    let query_type = tokens[0];
    let mut name: Option<String> = None;
    let mut unit_id: Option<u64> = None;
    let mut depth: u32 = 3;
    let mut limit: usize = 20;

    // Simple flag parser
    let mut i = 1;
    while i < tokens.len() {
        match tokens[i] {
            "--name" | "-n" if i + 1 < tokens.len() => {
                name = Some(tokens[i + 1].to_string());
                i += 2;
            }
            "--unit-id" | "-u" if i + 1 < tokens.len() => {
                unit_id = tokens[i + 1].parse().ok();
                i += 2;
            }
            "--depth" | "-d" if i + 1 < tokens.len() => {
                depth = tokens[i + 1].parse().unwrap_or(3);
                i += 2;
            }
            "--limit" | "-l" if i + 1 < tokens.len() => {
                limit = tokens[i + 1].parse().unwrap_or(20);
                i += 2;
            }
            _ => {
                // Bare argument — treat as name for symbol, or unit-id for others
                if query_type == "symbol" && name.is_none() {
                    name = Some(tokens[i].to_string());
                } else if unit_id.is_none() {
                    unit_id = tokens[i].parse().ok();
                }
                i += 1;
            }
        }
    }

    match query_type {
        "symbol" | "sym" | "s" => {
            let search = match name {
                Some(n) => n,
                None => {
                    eprintln!("  {} --name is required for symbol queries", s.fail());
                    return Ok(());
                }
            };
            let params = SymbolLookupParams {
                name: search.clone(),
                mode: MatchMode::Contains,
                limit,
                ..Default::default()
            };
            let results = engine.symbol_lookup(graph, params)?;
            eprintln!(
                "\n  Symbol lookup: {} ({} results)\n",
                s.bold(&format!("\"{}\"", search)),
                results.len()
            );
            for (i, unit) in results.iter().enumerate() {
                eprintln!(
                    "  {:>3}. {} {} {}",
                    s.dim(&format!("#{}", i + 1)),
                    s.bold(&unit.qualified_name),
                    s.dim(&format!("({})", unit.unit_type)),
                    s.dim(&format!("[id:{}]", unit.id))
                );
            }
            eprintln!();
        }

        "deps" | "dep" | "d" => {
            let uid = match unit_id {
                Some(u) => u,
                None => {
                    eprintln!("  {} --unit-id is required for deps queries", s.fail());
                    return Ok(());
                }
            };
            let params = DependencyParams {
                unit_id: uid,
                max_depth: depth,
                edge_types: vec![],
                include_transitive: true,
            };
            let result = engine.dependency_graph(graph, params)?;
            let root = graph
                .get_unit(uid)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            eprintln!(
                "\n  Dependencies of {} ({} found)\n",
                s.bold(root),
                result.nodes.len()
            );
            for node in &result.nodes {
                let name = graph
                    .get_unit(node.unit_id)
                    .map(|u| u.qualified_name.as_str())
                    .unwrap_or("?");
                let indent = "  ".repeat(node.depth as usize);
                eprintln!("  {}{} {}", indent, s.arrow(), s.cyan(name));
            }
            eprintln!();
        }

        "rdeps" | "rdep" | "r" => {
            let uid = match unit_id {
                Some(u) => u,
                None => {
                    eprintln!("  {} --unit-id is required for rdeps queries", s.fail());
                    return Ok(());
                }
            };
            let params = DependencyParams {
                unit_id: uid,
                max_depth: depth,
                edge_types: vec![],
                include_transitive: true,
            };
            let result = engine.reverse_dependency(graph, params)?;
            let root = graph
                .get_unit(uid)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            eprintln!(
                "\n  Reverse deps of {} ({} found)\n",
                s.bold(root),
                result.nodes.len()
            );
            for node in &result.nodes {
                let name = graph
                    .get_unit(node.unit_id)
                    .map(|u| u.qualified_name.as_str())
                    .unwrap_or("?");
                let indent = "  ".repeat(node.depth as usize);
                eprintln!("  {}{} {}", indent, s.arrow(), s.cyan(name));
            }
            eprintln!();
        }

        "impact" | "imp" | "i" => {
            let uid = match unit_id {
                Some(u) => u,
                None => {
                    eprintln!("  {} --unit-id is required for impact queries", s.fail());
                    return Ok(());
                }
            };
            let params = ImpactParams {
                unit_id: uid,
                max_depth: depth,
                edge_types: vec![],
            };
            let result = engine.impact_analysis(graph, params)?;
            let root = graph
                .get_unit(uid)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");

            let risk_label = if result.overall_risk >= 0.7 {
                s.red("HIGH")
            } else if result.overall_risk >= 0.4 {
                s.yellow("MEDIUM")
            } else {
                s.green("LOW")
            };

            eprintln!("\n  Impact of {} (risk: {})\n", s.bold(root), risk_label,);
            for imp in &result.impacted {
                let name = graph
                    .get_unit(imp.unit_id)
                    .map(|u| u.qualified_name.as_str())
                    .unwrap_or("?");
                let risk_sym = if imp.risk_score >= 0.7 {
                    s.fail()
                } else if imp.risk_score >= 0.4 {
                    s.warn()
                } else {
                    s.ok()
                };
                eprintln!(
                    "  {} {} {} risk:{:.2}",
                    risk_sym,
                    s.cyan(name),
                    s.dim(&format!("(depth {})", imp.depth)),
                    imp.risk_score,
                );
            }
            eprintln!();
        }

        "calls" | "call" | "c" => {
            let uid = match unit_id {
                Some(u) => u,
                None => {
                    eprintln!("  {} --unit-id is required for calls queries", s.fail());
                    return Ok(());
                }
            };
            let params = CallGraphParams {
                unit_id: uid,
                direction: CallDirection::Both,
                max_depth: depth,
            };
            let result = engine.call_graph(graph, params)?;
            let root = graph
                .get_unit(uid)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            eprintln!(
                "\n  Call graph for {} ({} nodes)\n",
                s.bold(root),
                result.nodes.len()
            );
            for (nid, d) in &result.nodes {
                let name = graph
                    .get_unit(*nid)
                    .map(|u| u.qualified_name.as_str())
                    .unwrap_or("?");
                let indent = "  ".repeat(*d as usize);
                eprintln!("  {}{} {}", indent, s.arrow(), s.cyan(name));
            }
            eprintln!();
        }

        "similar" | "sim" => {
            let uid = match unit_id {
                Some(u) => u,
                None => {
                    eprintln!("  {} --unit-id is required for similar queries", s.fail());
                    return Ok(());
                }
            };
            let params = SimilarityParams {
                unit_id: uid,
                top_k: limit,
                min_similarity: 0.0,
            };
            let results = engine.similarity(graph, params)?;
            let root = graph
                .get_unit(uid)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            eprintln!(
                "\n  Similar to {} ({} matches)\n",
                s.bold(root),
                results.len()
            );
            for (i, m) in results.iter().enumerate() {
                let name = graph
                    .get_unit(m.unit_id)
                    .map(|u| u.qualified_name.as_str())
                    .unwrap_or("?");
                eprintln!(
                    "  {:>3}. {} {}",
                    s.dim(&format!("#{}", i + 1)),
                    s.cyan(name),
                    s.yellow(&format!("{:.1}%", m.score * 100.0)),
                );
            }
            eprintln!();
        }

        "prophecy" | "predict" | "p" => {
            let params = ProphecyParams {
                top_k: limit,
                min_risk: 0.0,
            };
            let result = engine.prophecy(graph, params)?;
            eprintln!(
                "\n  {} Prophecy ({} predictions)\n",
                s.info(),
                result.predictions.len()
            );
            if result.predictions.is_empty() {
                eprintln!("  {} Codebase looks stable!", s.ok());
            }
            for pred in &result.predictions {
                let name = graph
                    .get_unit(pred.unit_id)
                    .map(|u| u.qualified_name.as_str())
                    .unwrap_or("?");
                let risk_sym = if pred.risk_score >= 0.7 {
                    s.fail()
                } else if pred.risk_score >= 0.4 {
                    s.warn()
                } else {
                    s.ok()
                };
                eprintln!(
                    "  {} {} {}: {}",
                    risk_sym,
                    s.cyan(name),
                    s.dim(&format!("(risk {:.2})", pred.risk_score)),
                    pred.reason,
                );
            }
            eprintln!();
        }

        "stability" | "stab" => {
            let uid = match unit_id {
                Some(u) => u,
                None => {
                    eprintln!("  {} --unit-id is required for stability queries", s.fail());
                    return Ok(());
                }
            };
            let result = engine.stability_analysis(graph, uid)?;
            let root = graph
                .get_unit(uid)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            let score_color = if result.overall_score >= 0.7 {
                s.green(&format!("{:.2}", result.overall_score))
            } else if result.overall_score >= 0.4 {
                s.yellow(&format!("{:.2}", result.overall_score))
            } else {
                s.red(&format!("{:.2}", result.overall_score))
            };
            eprintln!("\n  Stability of {}: {}\n", s.bold(root), score_color);
            for factor in &result.factors {
                eprintln!(
                    "  {} {} = {:.2}: {}",
                    s.arrow(),
                    s.bold(&factor.name),
                    factor.value,
                    s.dim(&factor.description),
                );
            }
            eprintln!();
        }

        "coupling" | "couple" => {
            let params = CouplingParams {
                unit_id,
                min_strength: 0.0,
            };
            let results = engine.coupling_detection(graph, params)?;
            eprintln!("\n  Coupling analysis ({} pairs)\n", results.len());
            if results.is_empty() {
                eprintln!("  {} No tightly coupled pairs detected.", s.ok());
            }
            for c in &results {
                let name_a = graph
                    .get_unit(c.unit_a)
                    .map(|u| u.qualified_name.as_str())
                    .unwrap_or("?");
                let name_b = graph
                    .get_unit(c.unit_b)
                    .map(|u| u.qualified_name.as_str())
                    .unwrap_or("?");
                eprintln!(
                    "  {} {} {} {} {}",
                    s.warn(),
                    s.cyan(name_a),
                    s.dim("<->"),
                    s.cyan(name_b),
                    s.yellow(&format!("{:.0}%", c.strength * 100.0)),
                );
            }
            eprintln!();
        }

        other => {
            let known = [
                "symbol",
                "deps",
                "rdeps",
                "impact",
                "calls",
                "similar",
                "prophecy",
                "stability",
                "coupling",
            ];
            eprintln!(
                "  {} Unknown query type: {}. Available: {}",
                s.fail(),
                other,
                known.join(", ")
            );
        }
    }

    Ok(())
}

/// /get <unit-id> — Show detailed unit info.
fn cmd_get(args: &str, state: &mut ReplState) -> Result<(), Box<dyn std::error::Error>> {
    let s = Styled::auto();
    let graph = match state.require_graph() {
        Some(g) => g,
        None => return Ok(()),
    };

    let uid: u64 = match args.split_whitespace().next().and_then(|s| s.parse().ok()) {
        Some(id) => id,
        None => {
            eprintln!("  {} Usage: {}", s.info(), s.bold("/get <unit-id>"));
            return Ok(());
        }
    };

    let unit = match graph.get_unit(uid) {
        Some(u) => u,
        None => {
            eprintln!("  {} Unit {} not found", s.fail(), uid);
            return Ok(());
        }
    };

    let outgoing = graph.edges_from(uid);
    let incoming = graph.edges_to(uid);

    eprintln!();
    eprintln!("  {} {}", s.info(), s.bold(&format!("Unit {}", unit.id)));
    eprintln!("     Name:           {}", s.cyan(&unit.name));
    eprintln!("     Qualified name: {}", s.bold(&unit.qualified_name));
    eprintln!("     Type:           {}", unit.unit_type);
    eprintln!("     Language:       {}", unit.language);
    eprintln!(
        "     File:           {}",
        s.cyan(&unit.file_path.display().to_string())
    );
    eprintln!("     Span:           {}", unit.span);
    eprintln!("     Complexity:     {}", unit.complexity);
    eprintln!("     Stability:      {:.2}", unit.stability_score);

    if let Some(sig) = &unit.signature {
        eprintln!("     Signature:      {}", s.dim(sig));
    }
    if let Some(doc) = &unit.doc_summary {
        eprintln!("     Doc:            {}", s.dim(doc));
    }

    if !outgoing.is_empty() {
        eprintln!("\n     {} Outgoing edges ({})", s.arrow(), outgoing.len());
        for edge in &outgoing {
            let target_name = graph
                .get_unit(edge.target_id)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            eprintln!(
                "       {} {} {}",
                s.arrow(),
                s.cyan(target_name),
                s.dim(&format!("({})", edge.edge_type))
            );
        }
    }
    if !incoming.is_empty() {
        eprintln!("\n     {} Incoming edges ({})", s.arrow(), incoming.len());
        for edge in &incoming {
            let source_name = graph
                .get_unit(edge.source_id)
                .map(|u| u.qualified_name.as_str())
                .unwrap_or("?");
            eprintln!(
                "       {} {} {}",
                s.arrow(),
                s.cyan(source_name),
                s.dim(&format!("({})", edge.edge_type))
            );
        }
    }
    eprintln!();

    Ok(())
}

/// /units — List all units in the loaded graph.
fn cmd_units(state: &mut ReplState) -> Result<(), Box<dyn std::error::Error>> {
    let s = Styled::auto();
    let graph = match state.require_graph() {
        Some(g) => g,
        None => return Ok(()),
    };

    eprintln!("\n  {} units in graph:\n", graph.unit_count());
    for unit in graph.units() {
        eprintln!(
            "  {:>5}  {} {} {}",
            s.dim(&format!("[{}]", unit.id)),
            s.bold(&unit.qualified_name),
            s.dim(&format!("({})", unit.unit_type)),
            s.dim(&format!("c:{}", unit.complexity)),
        );
    }
    eprintln!();

    Ok(())
}
