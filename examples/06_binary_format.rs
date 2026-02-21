//! Binary format: write and read the .acb file format.
//!
//! Usage:
//!   cargo run --example 06_binary_format

use agentic_codebase::format::{AcbReader, AcbWriter};
use agentic_codebase::parse::parser::{ParseOptions, Parser};
use agentic_codebase::semantic::analyzer::{AnalyzeOptions, SemanticAnalyzer};
use agentic_codebase::types::FileHeader;
use std::fs;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    fs::write(
        dir.path().join("sample.rs"),
        r#"
/// A sample function.
pub fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// Another function that calls hello.
pub fn greet() -> String {
    hello("world")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        assert_eq!(hello("x"), "Hello, x!");
    }
}
"#,
    )?;

    // Parse and analyze
    let parser = Parser::new();
    let result = parser.parse_directory(dir.path(), &ParseOptions::default())?;
    let analyzer = SemanticAnalyzer::new();
    let graph = analyzer.analyze(result.units, &AnalyzeOptions::default())?;

    println!(
        "Original graph: {} units, {} edges",
        graph.unit_count(),
        graph.edge_count()
    );

    // Write to .acb file
    let out_dir = TempDir::new()?;
    let acb_path = out_dir.path().join("example.acb");
    let writer = AcbWriter::with_default_dimension();
    writer.write_to_file(&graph, &acb_path)?;

    let file_size = fs::metadata(&acb_path)?.len();
    println!("Wrote: {} ({} bytes)", acb_path.display(), file_size);

    // Read back the file
    let loaded = AcbReader::read_from_file(&acb_path)?;
    println!(
        "Loaded graph: {} units, {} edges",
        loaded.unit_count(),
        loaded.edge_count()
    );

    // Read the header directly
    let data = fs::read(&acb_path)?;
    let header_bytes: [u8; 128] = data[..128].try_into()?;
    let header = FileHeader::from_bytes(&header_bytes)?;
    println!("\nHeader details:");
    println!("  Version:   {}", header.version);
    println!("  Units:     {}", header.unit_count);
    println!("  Edges:     {}", header.edge_count);
    println!("  Dimension: {}", header.dimension);

    // Verify round-trip
    assert_eq!(graph.unit_count(), loaded.unit_count());
    assert_eq!(graph.edge_count(), loaded.edge_count());
    println!("\nRound-trip verification: PASS");

    Ok(())
}
